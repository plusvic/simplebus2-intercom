# Async Button

Async button handling crate for `no_std` environments. Built around `embedded-hal 1.0` traits and `embassy-time`.

- [x] Detect button presses without blocking execution of other tasks or unnecessary polling.
- [x] Debouncing
- [x] Detect single and multiple short presses
- [x] Detect long presses
- [ ] Detect sequences of short and long presses or multiple long presses. Open an issue if this would be useful to you, or submit a PR!

## Example

```rust,ignore
let pin = /* Input pin */;
let mut button = Button::new(pin, ButtonConfig::default());

// In a separate task:
loop {
    match button.update().await {
        ButtonEvent::ShortPress { count } => {/* Do something with short presses */},
        ButtonEvent::LongPress => {/* Do something with long press */},
    }
}
```

## Features

- `defmt`: derives `defmt::Format` on public types (except `Button`).
- `std`: uses `tokio` instead of `embassy-time`. Mainly useful for tests.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
