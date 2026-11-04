#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation},
    delay::Delay,
    main,
};

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    // GPIO1 = ADC1 channel 0 on the ESP32-S3.
    let mut cfg = AdcConfig::new();
    let mut pin = cfg.enable_pin(p.GPIO1, Attenuation::_11dB);
    let mut adc1 = Adc::new(p.ADC1, cfg);
    let delay = Delay::new();

    loop {
        // read_oneshot returns nb::Result<u16, ()>; nb::block! retries on WouldBlock.
        let raw: u16 = nb::block!(adc1.read_oneshot(&mut pin)).unwrap();
        println!("ADC raw = {:4}  (~{} %)", raw, (raw as u32) * 100 / 4095);
        delay.delay_millis(200);
    }
}
