# buzzer

## Prerequisites

  * A recent version of the nightly Rust compiler. Anything including or
    greater than `rustc 1.63.0-nightly (fee3a459d 2022-06-05)` can be used.
  * The rust-src rustup component - `$ rustup component add rust-src`
  * AVR-GCC on the system for linking
  * AVR-Libc on the system for support libraries
  * minipro (https://gitlab.com/DavidGriffith/minipro)

## How to build

```bash
cargo build --release
objcopy -O ihex target/attiny13/release/buzzer.elf target/attiny13/release/buzzer.hex
minipro -w target/attiny13/release/buzzer.hex -p ATTINY13@DIP8
```
