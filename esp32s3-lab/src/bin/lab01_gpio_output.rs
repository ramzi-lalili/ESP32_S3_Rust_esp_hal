#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
};

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // GPIO2 is moved into the driver: nobody else can use it now.
    let mut led = Output::new(p.GPIO2, Level::Low, OutputConfig::default());
    let delay = Delay::new();

    loop {
        led.toggle();
        println!("LED is {}", if led.is_set_high() { "ON" } else { "OFF" });
        delay.delay_millis(500);
    }
}
