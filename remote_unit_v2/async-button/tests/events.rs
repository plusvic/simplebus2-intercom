use std::{sync::Arc, time::Duration};

use async_button::{Button, ButtonConfig, ButtonEvent, Mode};
use claims::{assert_err, assert_matches};
use embedded_hal::digital::{Error, ErrorType};
use tokio::{sync::RwLock, task::yield_now, time::timeout};

// Use shorter times to speed up test execution
const CONFIG: ButtonConfig = ButtonConfig {
    debounce: Duration::from_millis(10),
    double_click: Duration::from_millis(100),
    long_press: Duration::from_millis(200),
    mode: Mode::PullUp,
};

#[tokio::test]
async fn short_press() {
    let mut pin = MockPin::new();
    let mut button = {
        let pin = pin.clone();
        Button::new(pin, CONFIG)
    };

    tokio::spawn(async move {
        pin.set_low().await;
        sleep_millis(50).await;
        pin.set_high().await;
        sleep_millis(150).await;
    });

    let event = button.update().await;
    assert_matches!(event, ButtonEvent::ShortPress { count: 1 });

    verify_no_event(&mut button).await;
}

#[tokio::test]
async fn double_press() {
    let mut pin = MockPin::new();
    let mut button = {
        let pin = pin.clone();
        Button::new(pin, CONFIG)
    };

    tokio::spawn(async move {
        pin.set_low().await;
        sleep_millis(50).await;
        pin.set_high().await;
        sleep_millis(50).await;
        pin.set_low().await;
        sleep_millis(50).await;
        pin.set_high().await;
        sleep_millis(150).await;
    });

    let event = button.update().await;
    assert_matches!(event, ButtonEvent::ShortPress { count: 2 });

    verify_no_event(&mut button).await;
}

#[tokio::test]
async fn long_press() {
    let mut pin = MockPin::new();
    let mut button = {
        let pin = pin.clone();
        Button::new(pin, CONFIG)
    };

    tokio::spawn(async move {
        pin.set_low().await;
        sleep_millis(250).await;
        pin.set_high().await;
        sleep_millis(150).await;
    });

    let event = button.update().await;
    assert_matches!(event, ButtonEvent::LongPress);

    verify_no_event(&mut button).await;
}

#[tokio::test]
async fn two_short_presses() {
    let mut pin = MockPin::new();
    let mut button = {
        let pin = pin.clone();
        Button::new(pin, CONFIG)
    };

    tokio::spawn(async move {
        pin.set_low().await;
        sleep_millis(50).await;
        pin.set_high().await;
        sleep_millis(250).await;
        pin.set_low().await;
        sleep_millis(50).await;
        pin.set_high().await;
        sleep_millis(250).await;
    });

    let event = button.update().await;
    assert_matches!(event, ButtonEvent::ShortPress { count: 1 });
    let event = button.update().await;
    assert_matches!(event, ButtonEvent::ShortPress { count: 1 });

    verify_no_event(&mut button).await;
}

#[tokio::test]
async fn debounce() {
    let mut pin = MockPin::new();
    let mut button = {
        let pin = pin.clone();
        Button::new(pin, CONFIG)
    };

    tokio::spawn(async move {
        pin.set_low().await;
        pin.set_high().await;
        pin.set_low().await;
        sleep_millis(50).await;
        pin.set_high().await;
        sleep_millis(150).await;
    });

    let event = button.update().await;
    assert_matches!(event, ButtonEvent::ShortPress { count: 1 });

    verify_no_event(&mut button).await;
}

#[derive(Debug, Clone)]
struct MockPin(Arc<RwLock<bool>>);

#[derive(Debug)]
struct MockError;

impl MockPin {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(true)))
    }

    async fn set_high(&mut self) {
        *self.0.write().await = true;
    }

    async fn set_low(&mut self) {
        *self.0.write().await = false;
    }
}

impl embedded_hal::digital::InputPin for MockPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        match self.0.try_read() {
            Ok(rw) => Ok(*rw),
            Err(_) => Err(MockError),
        }
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        match self.0.try_read() {
            Ok(rw) => Ok(!*rw),
            Err(_) => Err(MockError),
        }
    }
}

impl embedded_hal_async::digital::Wait for MockPin {
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        loop {
            yield_now().await;
            let value = *self.0.read().await;
            if value {
                return Ok(());
            }
        }
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        loop {
            yield_now().await;
            let value = *self.0.read().await;
            if !value {
                return Ok(());
            }
        }
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        unimplemented!()
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        unimplemented!()
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

impl ErrorType for MockPin {
    type Error = MockError;
}

impl Error for MockError {
    fn kind(&self) -> embedded_hal::digital::ErrorKind {
        embedded_hal::digital::ErrorKind::Other
    }
}

async fn sleep_millis(millis: u64) {
    tokio::time::sleep(Duration::from_millis(millis)).await;
}

async fn verify_no_event(button: &mut Button<MockPin>) {
    assert_err!(
        timeout(Duration::from_millis(500), button.update()).await,
        "Unexpected event"
    );
}
