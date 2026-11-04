//! Lab 01 - GPIO digital output (LED blink)
//!
//! Wiring: GPIO21 --[330 ohm]--> LED anode, LED cathode --> GND.
//! API level: STABLE only (gpio, time, main). Works without the `unstable` feature.
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use esp32s3_labs::reserve_camera_pins;
use log::info;

// App descriptor required by the ESP-IDF 2nd-stage bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    // Takes ownership of every peripheral exactly once.
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Ownership demo: the OV2640 pins are moved into `_camera` and can never be
    // claimed again by accident. Remove this line and the build still works;
    // keep it and any later use of e.g. `peripherals.GPIO15` is a compile error.
    let _camera = reserve_camera_pins!(peripherals);

    // `Output::new(pin, initial_level, config)` consumes the GPIO21 token.
    let mut led = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    info!("Lab 01: blinking LED on GPIO21");
    loop {
        led.toggle();
        info!("LED is {}", if led.is_set_high() { "ON" } else { "OFF" });

        // Busy-wait for 500 ms using the free-running system timer.
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(500) {}
    }
}
