# repeater

## Prerequisites

  * A recent version of the nightly Rust compiler. Anything including or
    greater than `rustc 1.63.0-nightly (fee3a459d 2022-06-05)` can be used.
  * The rust-src rustup component - `$ rustup component add rust-src`
  * AVR-GCC on the system for linking
  * AVR-Libc on the system for support libraries
  * minipro (https://gitlab.com/DavidGriffith/minipro)

## How to build

This project must be built with a nightly version of rustc because we must use the 
`-Z build-std=core` in order to build the `core` standard library.

```bash
cargo build -Z build-std=core --target attiny85.json --release
objcopy -O ihex target/attiny85/release/repeater_v2.elf target/attiny85/release/repeater_v2.hex
minipro -w target/attiny85/release/repeater_v2.hex -p ATTINY85@DIP8
```