//! Lab 02 - GPIO digital input, pull-up / pull-down, software debounce
//!
//! Default wiring (PULL = Pull::Up):   GPIO14 --[push button]--> GND
//! Alternative   (PULL = Pull::Down):  GPIO14 --[push button]--> 3V3
//! LED on GPIO21 (see Lab 01) mirrors the button.
//! API level: STABLE only.
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    main,
    time::{Duration, Instant},
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

/// Change to `Pull::Down` if the button connects the pin to 3V3.
const PULL: Pull = Pull::Up;
const DEBOUNCE: Duration = Duration::from_millis(20);

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut led = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());
    let button = Input::new(peripherals.GPIO14, InputConfig::default().with_pull(PULL));

    // With a pull-up the idle level is High and "pressed" is Low; with a
    // pull-down it is the opposite.
    let pressed_level = match PULL {
        Pull::Up => Level::Low,
        _ => Level::High,
    };

    info!("Lab 02: press the button on GPIO14 (pull = {:?})", PULL);

    let mut stable = button.level();
    let mut last_raw = stable;
    let mut last_change = Instant::now();

    loop {
        let raw = button.level();
        if raw != last_raw {
            last_raw = raw;
            last_change = Instant::now(); // restart the debounce window
        }
        // Accept a new level only after it stayed unchanged for DEBOUNCE.
        if raw != stable && last_change.elapsed() >= DEBOUNCE {
            stable = raw;
            if stable == pressed_level {
                info!("button PRESSED");
                led.set_high();
            } else {
                info!("button RELEASED");
                led.set_low();
            }
        }
    }
}
