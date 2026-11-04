//! Lab 15 - Watchdog timer (TIMG0 main-system watchdog)
//!
//! No wiring needed. `esp_hal::init()` DISABLES all watchdogs, so we enable the
//! TIMG0 watchdog ourselves, feed it five times, then "hang" on purpose. After
//! 3 s without feeding the watchdog resets the chip and the reset reason is
//! printed on the next boot.
//! API level: UNSTABLE (`esp_hal::timer::timg::Wdt`).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    main,
    system::reset_reason,
    time::{Duration, Instant},
    timer::timg::{MwdtStage, TimerGroup},
};
use log::{info, warn};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("reset reason: {:?}", reset_reason());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut wdt = timg0.wdt;
    wdt.set_timeout(MwdtStage::Stage0, Duration::from_millis(3000));
    wdt.enable(); // stage 0 action = reset the whole system
    info!("watchdog armed: 3 s timeout");

    for i in 1..=5 {
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(1000) {}
        wdt.feed(); // restart the 3 s countdown
        info!("fed the watchdog ({}/5)", i);
    }

    warn!("simulating a software hang - no more feeding; reset expected in ~3 s");
    loop {} // never feeds again
}
