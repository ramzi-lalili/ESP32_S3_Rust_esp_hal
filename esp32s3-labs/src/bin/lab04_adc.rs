//! Lab 04 - ADC (analog-to-digital conversion)
//!
//! Wiring: potentiometer 10 k: one end 3V3, other end GND, wiper -> GPIO1 (ADC1_CH0).
//!         Optional second input on GPIO2 (ADC1_CH1), read as raw counts.
//! Use ADC1 pins (GPIO1..GPIO10): ADC2 is unusable while Wi-Fi is running.
//! GPIO4..GPIO10 are taken by the camera, so GPIO1/GPIO2 are the free ADC1 pins.
//! API level: UNSTABLE (`esp_hal::analog::adc` requires the `unstable` feature).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    analog::adc::{Adc, AdcCalCurve, AdcConfig, Attenuation},
    main,
    peripherals::ADC1,
    time::{Duration, Instant},
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

const SAMPLES: u32 = 16;

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut adc_config = AdcConfig::new();

    // GPIO1: 11 dB attenuation (~0 .. 3.1 V) with the factory curve calibration
    // stored in eFuse -> readings are in millivolts.
    let mut pin_mv =
        adc_config.enable_pin_with_cal::<_, AdcCalCurve<ADC1>>(peripherals.GPIO1, Attenuation::_11dB);
    // GPIO2: no calibration -> raw 12-bit counts (0..=4095).
    let mut pin_raw = adc_config.enable_pin(peripherals.GPIO2, Attenuation::_11dB);

    let mut adc = Adc::new(peripherals.ADC1, adc_config);

    loop {
        // Oversample to reduce noise.
        let mut sum_mv: u32 = 0;
        let mut sum_raw: u32 = 0;
        for _ in 0..SAMPLES {
            // read_oneshot() returns nb::Result<u16, ()>; WouldBlock means
            // "conversion not finished yet", block! simply retries.
            sum_mv += nb::block!(adc.read_oneshot(&mut pin_mv)).unwrap() as u32;
            sum_raw += adc.read_blocking(&mut pin_raw) as u32;
        }
        let mv = sum_mv / SAMPLES;
        let raw = sum_raw / SAMPLES;

        info!("GPIO1 = {} mV   |   GPIO2 raw = {} / 4095", mv, raw);

        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(500) {}
    }
}
