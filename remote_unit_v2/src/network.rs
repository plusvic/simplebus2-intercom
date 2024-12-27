use cyw43::{Control, JoinOptions, PowerManagementMode, Runner, State};
use cyw43_pio::PioSpi;

use defmt::{error, info, unwrap};
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH1, PIN_23, PIN_24, PIN_25, PIN_29, PIO1};
use embassy_rp::pio::Pio;
use embassy_time::Timer;
use static_cell::StaticCell;
use crate::{net_task, Irqs, WIFI_PASSWORD, WIFI_SSID};

#[embassy_executor::task]
pub async fn cyw43_task(
    runner: Runner<'static, Output<'static>, PioSpi<'static, PIO1, 0, DMA_CH1>>,
) -> ! {
    runner.run().await
}

pub async fn init_network<'a>(
    spawner: &Spawner,
    pio: PIO1,
    pwr: PIN_23,
    dio: PIN_24,
    cs: PIN_25,
    clk: PIN_29,
    dma: DMA_CH1,
) -> (Stack<'a>, Control<'a>) {
    // Include the firmware for the CYW43 chip, which tells the chip how to do
    // its Wi-Fi stuff.
    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(pwr, Level::Low);
    let cs = Output::new(cs, Level::High);

    // The RP2040 chip communicates with the CYW43 chip via SPI. But instead of
    // using one of the two SPI channels in the RP2040, it uses one the PIO banks
    // (PIO0 in this case) for PIO-driven SPI interface. It also uses one DMA
    // channel (DMA0) for transferring data from one chip to the other.
    let mut pio = Pio::new(pio, Irqs);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, dio, clk, dma);

    static STATE: StaticCell<State> = StaticCell::new();
    let state = STATE.init(State::new());

    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(PowerManagementMode::PowerSave)
        .await;

    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll.

    // Init network stack
    //
    // NOTE: DHCP and DNS need one socket slot if enabled. This is why we're
    // provisioning space for 3 sockets here: one for DHCP, one for DNS, and
    // one for your code (e.g. TCP). If you use more sockets you must increase
    // this.
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

    let (stack, runner) = embassy_net::new(
        net_device,
        Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    // Connect to Wi-Fi.
    loop {
        match control
            .join(WIFI_SSID, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(_) => {
                info!("Joined Wi-Fi network");
                break;
            }
            Err(err) => {
                error!(
                    "Failed to join Wi-Fi network with status={}",
                    err.status
                );
            }
        }
    }

    let mut built_in_led_status = false;
    control.gpio_set(0, built_in_led_status).await;

    // Wait for DHCP.
    loop {
        match stack.config_v4() {
            Some(conf) => {
                info!(
                    "IP address: {}  Gateway: {}",
                    conf.address, conf.gateway
                );
                break;
            }
            None => {
                info!("Waiting for DHCP...");
                built_in_led_status = !built_in_led_status;
                control.gpio_set(0, built_in_led_status).await;
                Timer::after_secs(2).await;
            }
        }
    }

    (stack, control)
}
