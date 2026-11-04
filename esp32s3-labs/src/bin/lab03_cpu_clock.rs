//! Lab 03 - CPU clock configuration
//!
//! The CPU clock is chosen once, at `esp_hal::init()`. Select it at build time:
//!
//!   cargo run --release --bin lab03_cpu_clock --features clk-80
//!   cargo run --release --bin lab03_cpu_clock --features clk-160
//!   cargo run --release --bin lab03_cpu_clock --features clk-240
//!
//! Without a feature the example uses `CpuClock::max()` (240 MHz on ESP32-S3).
//! Optional: probe GPIO21 with a scope to see the toggle rate change.
//! API level: `esp_hal::clock` is STABLE.
#![no_std]
#![no_main]

use core::hint::black_box;

use esp_backtrace as _;
use esp_hal::{
    clock::{CpuClock, cpu_clock, xtal_clock},
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

/// Pick the clock preset from Cargo features.
const fn selected_clock() -> CpuClock {
    if cfg!(feature = "clk-80") {
        CpuClock::_80MHz
    } else if cfg!(feature = "clk-160") {
        CpuClock::_160MHz
    } else if cfg!(feature = "clk-240") {
        CpuClock::_240MHz
    } else {
        CpuClock::max()
    }
}

/// A fixed amount of integer work. `black_box` stops the optimiser from
/// folding the loop into a constant.
fn workload() -> u32 {
    let mut acc: u32 = 0;
    for i in 0..1_000_000u32 {
        acc = black_box(acc.wrapping_mul(31).wrapping_add(i));
    }
    acc
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(selected_clock());
    let peripherals = esp_hal::init(config);

    let mut probe = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    info!("Requested preset : {:?}", selected_clock());
    info!("CPU clock        : {} MHz", cpu_clock().as_mhz());
    info!("XTAL clock       : {} MHz", xtal_clock().as_mhz());

    loop {
        let t0 = Instant::now();
        let result = workload();
        let us = t0.elapsed().as_micros();
        info!("1,000,000 iterations took {} us (checksum {})", us, result);

        // Toggle the probe pin as fast as the loop allows for ~1 s.
        let t1 = Instant::now();
        let mut toggles: u32 = 0;
        while t1.elapsed() < Duration::from_secs(1) {
            probe.toggle();
            toggles += 1;
        }
        info!("probe pin toggled {} times in 1 s", toggles);
    }
}
