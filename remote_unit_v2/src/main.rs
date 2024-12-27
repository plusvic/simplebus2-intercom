#![no_std]
#![no_main]

use crate::network::init_network;
use ::cyw43::NetDriver;
use async_button::{Button, ButtonConfig, ButtonEvent};
use core::mem::MaybeUninit;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::tcp::{ConnectError, TcpSocket};
use embassy_net::{dns, Runner, Stack};
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::{PIN_10, PIO0, PIO1, UART0};
use embassy_rp::pio::{
    Instance, InterruptHandler as PioInterruptHandler, Pio, PioPin,
};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_rp::uart::{
    Async, InterruptHandler as UartInterruptHandler, Uart,
};
use embassy_rp::{bind_interrupts, dma, uart, Peripheral};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::pubsub::{PubSubChannel, WaitResult};
use embassy_time::Timer;
use embedded_io_async::{Read, Write};
use rand_core::RngCore;
use rgb::RGB8;
use rust_mqtt::client::client::MqttClient;
use rust_mqtt::client::client_config::ClientConfig;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;
use rust_mqtt::utils::rng_generator::CountingRng;
use serde::{Deserialize, Serialize};
use {defmt_rtt as _, panic_probe as _};

mod network;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    PIO1_IRQ_0 => PioInterruptHandler<PIO1>;
    UART0_IRQ => UartInterruptHandler<UART0>;
});

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
const MQTT_BROKER: &str = env!("MQTT_BROKER");

const MY_INTERCOM_ADDRESS: u8 = 12;
const MQTT_MSG_TOPIC: &str = "plusvic/intercom/messages";
const MQTT_CMD_TOPIC: &str = "plusvic/intercom/commands";

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, NetDriver<'static>>) -> ! {
    runner.run().await
}

const NUM_LEDS: usize = 3;
const RED: RGB8 = RGB8::new(255, 0, 0);
const GREEN: RGB8 = RGB8::new(0, 255, 0);
const BLUE: RGB8 = RGB8::new(0, 0, 255);
const BLACK: RGB8 = RGB8::new(0, 0, 0);

const MUTED_COLOR: RGB8 = RGB8::new(60, 0, 20);

struct LedStrip<'d, P, const N: usize>
where
    P: Instance,
{
    pio: Mutex<CriticalSectionRawMutex, PioWs2812<'d, P, 0, N>>,
}

impl<'d, P, const N: usize> LedStrip<'d, P, N>
where
    P: Instance,
{
    fn new(
        mut pio: Pio<'d, P>,
        dma: impl Peripheral<P = impl dma::Channel> + 'd,
        pin: impl PioPin,
        program: &PioWs2812Program<'d, P>,
    ) -> Self {
        Self {
            pio: Mutex::new(PioWs2812::new(
                &mut pio.common,
                pio.sm0,
                dma,
                pin,
                &program,
            )),
        }
    }

    async fn write(&self, colors: &[RGB8; N]) {
        let mut pio = self.pio.lock().await;
        pio.write(colors).await
    }

    async fn all(&self, color: RGB8) {
        let colors = [color; N];
        self.write(&colors).await
    }
}

/// SimpleBus2 message codes.
#[derive(Clone, Debug, Default, Format, Serialize, Deserialize, PartialEq)]
struct Code(u8);

impl Code {
    const OPEN_DOOR: Code = Code(16);
    const CAMERA_ON: Code = Code(20);
    const CALL_FLOOR_DOOR: Code = Code(21);
    const CALL: Code = Code(48);
    const CALL_END: Code = Code(50);
}

/// Describes a SimpleBus2 message.
#[derive(Clone, Debug, Format, Serialize, Deserialize)]
struct Message {
    /// Message code.
    code: Code,
    /// Intercom address. This is the target or the source of the message,
    /// depending on the type of message. For instance, in `OpenDoor` messages
    /// this contains message's source (i.e: the address of the intercom
    /// sending the message), but in `CallFromEntryDoor` messages this contains
    /// the message's target (i.e: the address of the intercom that it being
    /// called).
    address: u8,
}

impl Message {
    pub fn from_raw_bytes(b: &[u8; 3]) -> Option<Self> {
        let address = b[1] >> 4 | b[2] << 4;
        let code = b[0] >> 6 | b[1] << 2;
        let checksum = b[2] >> 4;
        // Make sure the message is valid, the number of 1s in code and
        // address must be equal to checksum.
        if u8::count_ones(code) + u8::count_ones(address) == checksum as u32 {
            Some(Self { code: Code(code), address })
        } else {
            None
        }
    }

    pub fn into_raw_bytes(&self, b: &mut [u8; 3]) {
        let checksum =
            u8::count_ones(self.code.0) + u8::count_ones(self.address);
        b[0] = self.code.0 << 6;
        b[1] = self.address << 4 | self.code.0 >> 2;
        b[2] = (checksum as u8) << 4 | self.address >> 4;
    }
}

/// A task responsible for UART communication.
///
/// This task waits for messages to be received or transmitted over UART.
/// When a message is received over UART the message is published in the
/// UART_RX_MESSAGES pubsub to be consumed by other tasks. When a message
/// is published to the UART_TX_MESSAGES pubsub by other tasks, this task
/// consumes the message and transmit it over UART.
#[embassy_executor::task]
async fn uart_task(mut uart: Uart<'static, UART0, Async>) {
    let mut msg_bytes = [0u8; 3];

    let uart_rx = INBOUND_MESSAGES.publisher().unwrap();
    let mut uart_tx = OUTBOUND_MESSAGES.subscriber().unwrap();

    loop {
        // Wait for a message received through UART, or a message published to
        // UART_TX_MESSAGES that must be sent through UART, whatever comes
        // first.
        match select(uart.read(&mut msg_bytes), uart_tx.next_message()).await {
            // A message was received through UART, it must be published to the
            // UART_RX_MESSAGES pubsub.
            Either::First(Ok(_)) => {
                if let Some(msg) = Message::from_raw_bytes(&msg_bytes) {
                    info!("UART RX: {:?}", msg);
                    uart_rx.publish(msg).await;
                } else {
                    error!("Corrupted message: {:?}", msg_bytes);
                }
            }
            // Error while receiving message through UART.
            Either::First(Err(err)) => {
                error!("Error reading from UART: {:?}", err);
            }
            // A message was received from the UART_TX_MESSAGES pubsub, it must
            // be sent through UART.
            Either::Second(WaitResult::Message(msg)) => {
                info!("UART TX: {:?}", msg);
                msg.into_raw_bytes(&mut msg_bytes);
                let _ = uart.write(&msg_bytes).await;
            }
            Either::Second(WaitResult::Lagged(_)) => {}
        }
    }
}

/// A task responsible for communication with the MQTT broker.
///
/// This task handles sending messages received from UART to the MQTT broker
/// and forwarding messages from the broker back to UART.
#[embassy_executor::task]
async fn mqtt_task(stack: Stack<'static>) {
    loop {
        if let Err(err) = mqtt_task_loop(stack).await {
            error!("MQTT error: {:?}", err);
        };
        info!("Reconnecting to MQTT broker ...");
    }
}

#[derive(Debug, Format)]
enum MqttError {
    ConnectError(ConnectError),
    MqttError(ReasonCode),
    DnsError(dns::Error),
    PubSubError(embassy_sync::pubsub::Error),
}

async fn mqtt_task_loop(stack: Stack<'static>) -> Result<(), MqttError> {
    // Resolve MQTT server address.
    let mqtt_broker_addr = stack
        .dns_query(MQTT_BROKER, embassy_net::dns::DnsQueryType::A)
        .await
        .map_err(MqttError::DnsError)?
        .pop()
        .unwrap();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    socket
        .connect((mqtt_broker_addr, 1883))
        .await
        .map_err(MqttError::ConnectError)?;

    let mut recv_buf = [0; 80];
    let mut send_buf = [0; 80];
    let recv_buf_len = recv_buf.len();
    let send_buf_len = send_buf.len();

    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );

    config.add_max_subscribe_qos(
        rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1,
    );

    let mut mqtt_client = MqttClient::<_, 5, _>::new(
        socket,
        &mut send_buf,
        send_buf_len,
        &mut recv_buf,
        recv_buf_len,
        config,
    );

    mqtt_client.connect_to_broker().await.map_err(MqttError::MqttError)?;
    mqtt_client
        .subscribe_to_topic(MQTT_CMD_TOPIC)
        .await
        .map_err(MqttError::MqttError)?;

    info!("Connected to MQTT broker: {}", mqtt_broker_addr);

    let mut inbound =
        INBOUND_MESSAGES.subscriber().map_err(MqttError::PubSubError)?;

    let outbound =
        OUTBOUND_MESSAGES.publisher().map_err(MqttError::PubSubError)?;

    loop {
        // Wait for a message received from UART or from the MQTT broker,
        // whatever comes first.
        match select(inbound.next_message(), mqtt_client.receive_message())
            .await
        {
            // Got a message from UART, forward it to MQTT.
            Either::First(inbound_msg) => {
                let message = match inbound_msg {
                    WaitResult::Message(message) => message,
                    WaitResult::Lagged(_) => {
                        continue;
                    }
                };
                info!("MQTT TX: {:?}", message);
                mqtt_send_message(
                    &mut mqtt_client,
                    serde_json_core::to_vec::<Message, 100>(&message)
                        .unwrap()
                        .as_ref(),
                )
                .await
                .map_err(MqttError::MqttError)?;
            }
            // Got a message from MQTT, forward it to UART.
            Either::Second(mqtt_msg) => {
                match mqtt_msg.map_err(MqttError::MqttError)? {
                    (MQTT_CMD_TOPIC, b"open_door") => {
                        info!("MQTT RX: open door");
                        outbound
                            .publish(Message {
                                code: Code::OPEN_DOOR,
                                address: MY_INTERCOM_ADDRESS,
                            })
                            .await;
                    }
                    (MQTT_CMD_TOPIC, b"camera_on") => {
                        info!("MQTT RX: camera on");
                    }
                    (topic, cmd) => {
                        error!("MQTT: unknown command {}/{}", topic, cmd)
                    }
                }
            }
        }
    }
}

pub(crate) async fn mqtt_send_message<'a, T, const MAX_PROPERTIES: usize, R>(
    client: &mut MqttClient<'a, T, MAX_PROPERTIES, R>,
    message: &[u8],
) -> Result<(), ReasonCode>
where
    T: Read + Write,
    R: RngCore,
{
    client
        .send_message(
            MQTT_MSG_TOPIC,
            message,
            rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1,
            false,
        )
        .await
}

#[embassy_executor::task]
async fn feedback_task(
    led_strip: &'static LedStrip<'static, PIO0, NUM_LEDS>,
    motor: PIN_10,
) {
    let mut inbound = INBOUND_MESSAGES.subscriber().unwrap();
    let mut motor = Output::new(motor, Level::Low);

    led_strip.all(BLACK).await;

    loop {
        match inbound.next_message().await {
            WaitResult::Message(message) => match message.code {
                Code::CALL | Code::CALL_END | Code::OPEN_DOOR=> {
                    // The LED strip and the motor are turned on alternately
                    // and not simultaneously to reduce peak power demand.
                    led_strip.all(BLUE).await;
                    Timer::after_millis(500).await;
                    if *MUTED.lock().await {
                        led_strip.all(MUTED_COLOR).await;
                    } else {
                        led_strip.all(BLACK).await;
                        // Provide haptic feedback only if not muted.
                        motor.set_high();
                        Timer::after_millis(500).await;
                        motor.set_low();
                    }
                }
                Code::CALL_FLOOR_DOOR  => {
                    led_strip.all(BLUE).await;
                    Timer::after_millis(750).await;
                    led_strip.all(RED).await;
                    Timer::after_millis(750).await;
                    led_strip
                        .all(if *MUTED.lock().await {
                            MUTED_COLOR
                        } else {
                            BLACK
                        })
                        .await;
                }
                _ => {}
            },
            WaitResult::Lagged(_) => {}
        }
    }
}

/// PubSub channel where we put the incoming messages.
static INBOUND_MESSAGES: PubSubChannel<
    CriticalSectionRawMutex,
    Message,
    3, // Capacity
    3, // Subscribers, `uart_task`, `mqtt_task` and `feedback_task`.
    1, // Publishers
> = PubSubChannel::new();

/// PubSub channel where we put messages the outgoing messages.
static OUTBOUND_MESSAGES: PubSubChannel<
    CriticalSectionRawMutex,
    Message,
    3, // Capacity
    1, // Subscribers
    2, // Publishers, one for `uart_task` and another one for the main loop.
> = PubSubChannel::new();

/// When true, the haptic feedback is disabled.
static MUTED: Mutex<CriticalSectionRawMutex, bool> = Mutex::new(false);

/// Represents the WS2812 LED strip used for providing feedback.
static mut LED_STRIP: MaybeUninit<LedStrip<PIO0, NUM_LEDS>> =
    MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    info!("Program started");

    let peripherals = embassy_rp::init(Default::default());

    let mut pio0 = Pio::new(peripherals.PIO0, Irqs);
    let ws2812_prog = PioWs2812Program::new(&mut pio0.common);

    let led_strip = unsafe {
        LED_STRIP = MaybeUninit::new(LedStrip::new(
            pio0,
            peripherals.DMA_CH0,
            peripherals.PIN_22,
            &ws2812_prog,
        ));
        #[allow(static_mut_refs)]
        LED_STRIP.assume_init_ref()
    };

    unwrap!(spawner.spawn(feedback_task(led_strip, peripherals.PIN_10)));

    // Spawn task that waits for messages over UART.
    let mut config = uart::Config::default();
    config.baudrate = 4800;
    unwrap!(spawner.spawn(uart_task(Uart::new(
        peripherals.UART0,
        peripherals.PIN_16, // TX
        peripherals.PIN_17, // RX
        Irqs,
        peripherals.DMA_CH2,
        peripherals.DMA_CH3,
        config,
    ),)));

    let (stack, _control) = init_network(
        &spawner,
        peripherals.PIO1,
        peripherals.PIN_23,
        peripherals.PIN_24,
        peripherals.PIN_25,
        peripherals.PIN_29,
        peripherals.DMA_CH1,
    )
    .await;

    unwrap!(spawner.spawn(mqtt_task(stack)));

    let mut button = Button::new(
        Input::new(peripherals.PIN_6, Pull::Up),
        ButtonConfig::default(),
    );

    let outbound = OUTBOUND_MESSAGES.publisher().unwrap();

    loop {
        match button.update().await {
            ButtonEvent::ShortPress { count: _ } => {
                outbound
                    .publish(Message {
                        code: Code::OPEN_DOOR,
                        address: MY_INTERCOM_ADDRESS,
                    })
                    .await;
            }
            ButtonEvent::LongPress => {
                // A long press toggles the muted state.
                let mut muted = MUTED.lock().await;
                *muted = !*muted;
                led_strip.all(if *muted { MUTED_COLOR } else { BLACK }).await;
                info!("Muted: {}", *muted);
            }
        }
    }
}
