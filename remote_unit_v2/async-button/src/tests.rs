use claims::{assert_matches, assert_none, assert_some_eq};
use embedded_hal_mock::eh1::pin::{Mock, State as PinState, Transaction, TransactionKind as Kind};

use crate::{Button, ButtonConfig, ButtonEvent, Duration, Mode, State};

// Use shorter times to speed up test execution
const CONFIG: ButtonConfig = ButtonConfig {
    debounce: Duration::from_millis(10),
    double_click: Duration::from_millis(100),
    long_press: Duration::from_millis(200),
    mode: Mode::PullUp,
};

const IMMEDIATELY: Duration = Duration::from_millis(0);

#[tokio::test(start_paused = true)]
async fn start_idle() {
    let expectations = [Transaction::new(Kind::Get(PinState::High))];
    let pin = Mock::new(&expectations);
    let mut button = Button::new(pin, CONFIG);

    button.update_step().await;

    assert_matches!(button.state, State::Idle);

    button.pin.done();
}

#[tokio::test]
async fn start_pressed() {
    let expectations = [Transaction::get(PinState::Low)];
    let pin = Mock::new(&expectations);
    let mut button = Button::new(pin, CONFIG);

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Pressed);

    button.pin.done();
}

#[tokio::test]
async fn single_press() {
    let expectations = [
        Transaction::wait_for_state(PinState::Low, IMMEDIATELY),
        Transaction::get(PinState::Low),
        Transaction::wait_for_state(PinState::High, IMMEDIATELY),
        Transaction::get(PinState::High),
    ];
    let pin = Mock::new(&expectations);
    let mut button = Button::new(pin, CONFIG);
    button.state = State::Idle;

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Pressed);

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Released);
    assert_eq!(button.count, 1);

    button.pin.done();
}

#[tokio::test]
async fn long_press() {
    let expectations = [
        Transaction::wait_for_state(PinState::Low, IMMEDIATELY),
        Transaction::get(PinState::Low),
        Transaction::wait_for_state(PinState::High, Duration::from_millis(250)),
        Transaction::wait_for_state(PinState::High, IMMEDIATELY),
        Transaction::get(PinState::High),
    ];
    let pin = Mock::new(&expectations);
    let mut button = Button::new(pin, CONFIG);
    button.state = State::Idle;

    let _ = button.update_step().await;

    let event = button.update_step().await;
    assert_some_eq!(event, ButtonEvent::LongPress);
    assert_matches!(button.state, State::PendingRelease);
    assert_eq!(button.count, 0);

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Idle);
    assert_eq!(button.count, 0);

    button.pin.done();
}

#[tokio::test]
async fn debounce_press() {
    let expectations = [
        Transaction::wait_for_state(PinState::Low, IMMEDIATELY),
        Transaction::get(PinState::High),
    ];
    let pin = Mock::new(&expectations);
    let mut button = Button::new(pin, CONFIG);
    button.state = State::Idle;

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Idle);

    button.pin.done();
}

#[tokio::test]
async fn debounce_release() {
    let expectations = [
        Transaction::wait_for_state(PinState::High, IMMEDIATELY),
        Transaction::get(PinState::Low),
    ];
    let pin = Mock::new(&expectations);
    let mut button = Button::new(pin, CONFIG);
    button.state = State::Pressed;

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Pressed);

    button.pin.done();
}

#[tokio::test]
async fn double_click() {
    let expectations = [
        Transaction::wait_for_state(PinState::Low, IMMEDIATELY),
        Transaction::get(PinState::Low),
        Transaction::wait_for_state(PinState::High, IMMEDIATELY),
        Transaction::get(PinState::High),
        Transaction::wait_for_state(PinState::Low, IMMEDIATELY),
        Transaction::get(PinState::Low),
        Transaction::wait_for_state(PinState::High, IMMEDIATELY),
        Transaction::get(PinState::High),
        Transaction::wait_for_state(PinState::Low, Duration::from_millis(250)),
    ];
    let pin = Mock::new(&expectations);
    let mut button = Button::new(pin, CONFIG);
    button.state = State::Idle;

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Pressed);

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Released);

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Pressed);

    let event = button.update_step().await;
    assert_none!(event);
    assert_matches!(button.state, State::Released);

    let event = button.update_step().await;
    assert_some_eq!(event, ButtonEvent::ShortPress { count: 2 });
    assert_matches!(button.state, State::Idle);

    button.pin.done();
}
