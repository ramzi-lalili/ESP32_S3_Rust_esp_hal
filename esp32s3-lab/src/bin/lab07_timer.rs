#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    main,
    time::Duration,
    timer::{timg::TimerGroup, PeriodicTimer},
};

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let mut led = Output::new(p.GPIO2, Level::Low, OutputConfig::default());

    let tg0 = TimerGroup::new(p.TIMG0);
    let mut timer = PeriodicTimer::new(tg0.timer0);
    timer.start(Duration::from_millis(500)).unwrap();

    loop {
        timer.wait(); // blocks until the period elapsed
        led.toggle();
        println!("tick");
    }
}
