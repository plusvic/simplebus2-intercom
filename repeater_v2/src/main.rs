#![feature(abi_avr_interrupt)]
#![no_std]
#![no_main]

use core::mem;

use attiny_hal as hal;
use attiny_hal::pac::TC0;
use attiny_hal::port::mode::{Input, PullUp};
use attiny_hal::port::{PB2, PB3, PB4};
use panic_halt as _;

use hal::clock::MHz1;
use hal::port::mode::Output;
use hal::port::{Pin, PinOps};
use hal::prelude::*;

/// The device is shipped with its internal clock configured at 8MHz,
/// but the CKDIV8 fuse is programmed by default, which means the clock
/// frequency is actually 1MHz.
type ClockFreq = MHz1;

struct Bus {
    tx: Pin<Output, PB2>,
    timer: TC0,
    received_bits: u32,
    num_received_bits: u8,
    received_msg: Option<Message>,
}

struct Uart {
    rx: Pin<Input<PullUp>, PB4>,
    tx: Pin<Output, PB3>,
    received_msg: Option<Message>,
}

struct Message {
    addr: u8,
    code: u8,
    checksum: u8,
}

impl Message {
    pub fn new(addr: u8, code: u8, checksum: u8) -> Self {
        Self {
            addr,
            code,
            checksum,
        }
    }

    /// Makes sure that the message's checksum is the expected one. Returns
    /// `None` if the message is invalid.
    pub fn validate(self) -> Option<Self> {
        if u8::count_ones(self.addr) + u8::count_ones(self.code) == self.checksum as u32 {
            Some(self)
        } else {
            None
        }
    }
}

static mut BUS: mem::MaybeUninit<Bus> = mem::MaybeUninit::uninit();
static mut UART: mem::MaybeUninit<Uart> = mem::MaybeUninit::uninit();

#[attiny_hal::entry]
fn main() -> ! {
    let peripherals = hal::Peripherals::take().unwrap();
    let pins: hal::Pins = hal::pins!(peripherals);

    // Calibrate internal oscillator. This changes from chip to chip.
    // https://becomingmaker.com/tuning-attiny-oscillator/
    peripherals.CPU.osccal.write(|w| w.osccal().bits(105));

    // As the timer's pre-scaler is set to 256, the timer's clock is
    // 1MHz/256 = 3.906KHz. In other words, the timer is incremented
    // every 256us.
    peripherals.TC0.tccr0b.write(|w| w.cs0().prescale_256());

    // Configure the Analog Comparator Interrupt to occur on both the
    // raising and falling edge. Also set the ACIE bit, which enables
    // the interrupt. The interrupt won't be effectively enabled yet
    // because interrupts are globally disabled.
    peripherals
        .AC
        .acsr
        .write(|w| w.acis().on_toggle().acie().set_bit());

    // Enable interrupt on change of pin PB4.
    peripherals.EXINT.pcmsk.write(|w| w.bits(0b10000));
    // Enable pin change interrupt.
    peripherals.EXINT.gimsk.write(|w| w.pcie().set_bit());

    // SAFETY: Interrupts are not enabled at this point so we can safely
    // initialize the global variables.
    unsafe {
        UART = mem::MaybeUninit::new(Uart {
            rx: pins.pb4.into_pull_up_input(),
            tx: pins.pb3.into_output_high(),
            received_msg: None,
        });

        BUS = mem::MaybeUninit::new(Bus {
            tx: pins.pb2.into_output(),
            timer: peripherals.TC0,
            received_bits: 0,
            num_received_bits: 0,
            received_msg: None,
        });
    }

    // Enable interrupts globally. This executes a SEI instruction.
    unsafe {
        avr_device::interrupt::enable();
    }

    let bus = unsafe { &mut *BUS.as_mut_ptr() };
    let uart = unsafe { &mut *UART.as_mut_ptr() };

    loop {
        // Go to sleep and wait for interrupts. Interrupts will occur
        // when a message is received over UART or SimpleBus.
        avr_device::asm::sleep();

        avr_device::interrupt::free(|_| {
            // If a message was received over SimpleBus, retransmit it
            // over UART.
            if let Some(msg) = bus.received_msg.take() {
                uart_tx(&mut uart.tx, msg);
            }
            // If a message was received over UART, retransmit it over
            // SimpleBus.
            if let Some(msg) = uart.received_msg.take() {
                bus_tx(&mut bus.tx, msg);
            }
        });
    }
}

/// Pin change interrupt handler.
///
/// This interrupt occurs when there's a change in PB4, the UART's RX
/// pin, which is connected to the HC-12 TX pin. So, the interrupt occurs
/// when the HC-12 starts transmitting data.
#[avr_device::interrupt(attiny85)]
fn PCINT0() {
    let mut delay = hal::delay::Delay::<ClockFreq>::new();
    let uart = unsafe { &mut *UART.as_mut_ptr() };

    // Wait 70us into the start bit, we don't want to sample the bits
    // right after the falling edge of the start bit.
    delay.delay_us(70_u8);

    // RX must be low during the start bit. If not, this is a spurious
    // interrupt.
    if uart.rx.is_high() {
        return;
    }

    // Read 3 bytes from the UART.
    let byte1 = uart_rx(&mut uart.rx);
    let byte2 = uart_rx(&mut uart.rx);
    let byte3 = uart_rx(&mut uart.rx);

    // Decode the content the message, which will be processed
    // in the main loop.
    let addr = byte2 >> 4 | byte3 << 4;
    let code = byte1 >> 6 | byte2 << 2;
    let checksum = byte3 >> 4;

    uart.received_msg = Message::new(addr, code, checksum).validate();
}

/// Analog comparator interrupt handler.
///
/// This interrupts occurs when the voltage in AIN0 goes above the reference
/// voltage in AIN1. This happens when a message is transmitted through the
/// SimpleBus line.
#[avr_device::interrupt(attiny85)]
fn ANA_COMP() {
    // SAFETY: We _know_ that interrupts will only be enabled after BUS_RX
    // has been initialized so this ISR will never run with an invalid bus_rx.
    let bus = unsafe { &mut *BUS.as_mut_ptr() };

    // Check the time elapsed since the last ANA_COMP interrupt. The timer is
    // incremented every 256us. If a different CPU speed is used the ranges
    // below must be adjusted accordingly.
    match bus.timer.tcnt0.read().bits() {
        // ~3ms elapsed (3000us / 256us = 11.719). A zero was received.
        (8..=14) => {
            bus.received_bits >>= 1;
            bus.num_received_bits += 1;
        }
        // ~6ms elapsed (6000us / 256us = 23.437). A one was received.
        (19..=26) => {
            bus.received_bits |= 0b100_0000_0000_0000_0000;
            bus.received_bits >>= 1;
            bus.num_received_bits += 1;
        }
        // ~17ms elapsed (17000us / 256us = 66.406). Preamble.
        (62..=70) => {
            bus.received_bits = 0;
            bus.num_received_bits = 0;
        }
        _ => {}
    }

    if bus.num_received_bits == 18 {
        let checksum: u8 = (bus.received_bits >> 14) as u8;
        let addr = ((bus.received_bits >> 6) & 0b_11111111) as u8;
        let code = (bus.received_bits & 0b_111111) as u8;

        bus.received_msg = Message::new(addr, code, checksum).validate();
        bus.num_received_bits = 0;
    }

    // Set the timer counter back to zero.
    bus.timer.tcnt0.reset();
}

/// Transmits a message over the SimpleBus line.
fn bus_tx<PIN: PinOps>(pin: &mut Pin<Output, PIN>, mut message: Message) {
    let mut delay = hal::delay::Delay::<ClockFreq>::new();

    // Preamble.
    burst_25khz_3ms(pin);
    delay.delay_ms(17_u8);
    burst_25khz_3ms(pin);

    for _ in 0..6 {
        if (message.code & 1) == 1 {
            delay.delay_ms(6_u8); // 6ms = 1
        } else {
            delay.delay_ms(3_u8); // 3ms = 0
        }
        burst_25khz_3ms(pin);
        message.code >>= 1;
    }

    for _ in 0..8 {
        if (message.addr & 1) == 1 {
            delay.delay_ms(6_u8);
        } else {
            delay.delay_ms(3_u8);
        }
        burst_25khz_3ms(pin);
        message.addr >>= 1;
    }

    for _ in 0..4 {
        if (message.checksum & 1) == 1 {
            delay.delay_ms(6_u8);
        } else {
            delay.delay_ms(3_u8);
        }
        burst_25khz_3ms(pin);
        message.checksum >>= 1;
    }
}

/// Creates a 25KHz burst of pulses for 3ms.
#[inline(never)]
fn burst_25khz_3ms<PIN: PinOps>(pin: &mut Pin<Output, PIN>) {
    let mut delay = hal::delay::Delay::<ClockFreq>::new();
    for _ in 0..75 {
        pin.set_high();
        // In theory this should be 40us, but the delay_us seems to be
        // very imprecise with low delays.
        delay.delay_us(45_u8);
        pin.set_low();
    }
}

/// Transmits a message over UART.
fn uart_tx<PIN: PinOps>(pin: &mut Pin<Output, PIN>, msg: Message) {
    uart_tx_byte(pin, msg.code << 6);
    uart_tx_byte(pin, msg.addr << 4 | msg.code >> 2);
    uart_tx_byte(pin, msg.checksum << 4 | msg.addr >> 4);
}

/// Transmits a byte using bit-banged UART.
///
/// Settings: 4800 bps, 8 databits, no parity, 1 stop bit, no flow control.
#[inline(never)]
fn uart_tx_byte<PIN: PinOps>(pin: &mut Pin<Output, PIN>, mut byte: u8) {
    let mut delay = hal::delay::Delay::<ClockFreq>::new();

    // Start bit
    pin.set_low();

    // At 4800 bps 1 bit takes 208 microseconds.
    delay.delay_us(208_u8);

    for _ in 0..8 {
        if byte & 1 == 0 {
            pin.set_low();
        } else {
            pin.set_high();
        }
        byte >>= 1;
        // Wait for 190 us, compensating for the time it takes to run the loop
        // control instructions.
        delay.delay_us(190_u8);
    }

    // Stop bit
    pin.set_high();
    delay.delay_us(208_u8);
}

#[inline(never)]
fn uart_rx(pin: &mut Pin<Input<PullUp>, PB4>) -> u8 {
    let mut delay = hal::delay::Delay::<ClockFreq>::new();
    let mut result = 0u8;

    // Skip start bit.
    delay.delay_us(208_u8);

    for _ in 0..8 {
        result >>= 1;
        if pin.is_high() {
            result |= 0b1000_0000;
        } else {
        }
        delay.delay_us(200_u8);
    }

    // Skip stop bit.
    delay.delay_us(208_u8);

    result
}
