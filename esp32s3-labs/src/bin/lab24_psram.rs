//! Lab 24 (bonus) - External PSRAM as heap memory
//!
//! No wiring. Requires a board whose module has PSRAM (e.g. ESP32-S3-WROOM-1 N16R8 / N8R8:
//! octal PSRAM; N16R2 / N4R2: quad PSRAM). `PsramMode::Auto` (default) tries octal, then quad.
//! IMPORTANT consequences for this manual's pin plan:
//!   * Octal PSRAM (R8) occupies GPIO35, GPIO36, GPIO37 -> never use them as GPIO.
//!   * GPIO26..GPIO32 (and GPIO33/34 with octal flash) belong to flash/PSRAM too.
//! WARNING: atomic instructions do not work correctly in PSRAM (esp-alloc docs). Never allocate
//! `Atomic*` types (or anything containing them, e.g. `Arc`) from the PSRAM heap.
//! Build in release mode only.
//! API level: UNSTABLE (`esp_hal::psram`) + esp-alloc 0.10.0.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use esp_backtrace as _;
use esp_hal::{
    main,
    time::{Duration, Instant},
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Small internal-RAM heap + the PSRAM as a second heap region.
    esp_alloc::heap_allocator!(size: 32 * 1024);
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    info!("heap after init: used {} B, free {} B", esp_alloc::HEAP.used(), esp_alloc::HEAP.free());

    // 500 KiB cannot fit in the 32 KiB internal heap, so it must come from PSRAM.
    let words = 500 * 1024 / 4;
    let mut big: Vec<u32> = Vec::with_capacity(words);
    for i in 0..words {
        big.push((i & 0xFF) as u32);
    }
    let checksum: u32 = big.iter().fold(0u32, |a, &b| a.wrapping_add(b));
    info!("allocated {} KiB, checksum {}", words * 4 / 1024, checksum);
    info!("heap after alloc: used {} B, free {} B", esp_alloc::HEAP.used(), esp_alloc::HEAP.free());

    loop {
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(5) {}
        info!("still alive, vector[1000] = {}", big[1000]);
    }
}
