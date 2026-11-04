#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    clock::CpuClock,
    main,
    time::{Duration, Instant},
};

// Rebuild with CpuClock::_80MHz, _160MHz and _240MHz and compare the printed counts.
const CLOCK: CpuClock = CpuClock::_240MHz;

#[main]
fn main() -> ! {
    let _p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CLOCK));

    loop {
        let start = Instant::now();
        let mut n: u32 = 0;
        while start.elapsed() < Duration::from_secs(1) {
            n = n.wrapping_add(1);
            core::hint::black_box(n);
        }
        println!("loop iterations in 1 s: {}", n);
    }
}
