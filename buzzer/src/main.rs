#![no_std]
#![no_main]

use attiny_hal as hal;
use panic_halt as _;

use hal::prelude::*;
use hal::port::{Pin, PinOps};
use hal::port::mode::Output;
use hal::clock::MHz12;

const ADDR: u8 = 12;
const MSG: u8 = 21;  // MSG_CALL_FROM_FLOOR_DOOR
const CHECKSUM: u8 = (u8::count_ones(MSG) + u8::count_ones(ADDR)) as u8;

#[attiny_hal::entry]
fn main() -> ! {
    let dp = hal::Peripherals::take().unwrap();
    let pins = hal::pins!(dp);

    // attiny13 comes from factory with internal RC oscillator configured at
    // 9.6MHz, but the CKDIV8 fuse is programmed, dividing the clock by 8, and
    // making the effective speed 1.2MHz. The hal crate doesn't have 1.2MHz
    // implementation for `Delay`, but it has a 12MHz one. So we use that, 
    // and divide the delays by 10. For waiting 100 microseconds we call 
    // delay_us(10).
    let mut delay = hal::delay::Delay::<MHz12>::new();
    let mut output = pins.pb0.into_output();

    output.set_high();


    loop {
        uart_tx(MSG << 6, &mut output);
        uart_tx(ADDR << 4 | MSG >> 2, &mut output);
        uart_tx(CHECKSUM << 4 | ADDR >> 4, &mut output);

        delay.delay_ms(25_u16);  // 250 ms
    }
}

/// Transmits a byte using bit-banged UART.
/// 
/// Settings: 4800 bps, 8 databits, no parity, 1 stop bit, no flow control.
fn uart_tx<PIN: PinOps>(byte: u8, pin: &mut Pin<Output, PIN>) {

    let mut delay = hal::delay::Delay::<MHz12>::new();

    // Start bit
    pin.set_low();
    
    // At 4800 bps 1 bit takes 208 microseconds, wait 210 us.
    delay.delay_us(21_u8);

    for i in 0..8 {
        if byte & (1 << i) == 0 {
            pin.set_low();
        } else {
            pin.set_high();
        }
        // Wait for 190 us, compensating for the time it takes to run the loop
        // control instructions.  
        delay.delay_us(19_u8);
    }

    // Stop bit
    pin.set_high();
    delay.delay_us(21_u8);
}