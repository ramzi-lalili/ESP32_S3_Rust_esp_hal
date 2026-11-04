#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    main,
};

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let mut led = Output::new(p.GPIO2, Level::Low, OutputConfig::default());
    // Button between GPIO21 and GND; internal pull-up keeps the pin HIGH when released.
    let button = Input::new(p.GPIO21, InputConfig::default().with_pull(Pull::Up));
    let delay = Delay::new();

    loop {
        let pressed = button.is_low();
        led.set_level(Level::from(pressed));
        delay.delay_millis(10);
    }
}
