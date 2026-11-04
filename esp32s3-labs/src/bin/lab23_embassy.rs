//! Lab 23 - Asynchronous programming with Embassy (esp-rtos executor)
//!
//! Wiring: LED GPIO21 --[330 ohm]--> GND;  button GPIO14 --> GND (internal pull-up).
//! Four concurrent activities share ONE core without threads or interrupts in user code:
//!   blinker      - blinks the LED; its period can change at any moment
//!   button_task  - awaits a falling edge (no polling!), then cycles the blink period
//!   main         - prints a heartbeat with a drift-free `Ticker`
//! Tasks communicate through an Embassy `Signal` (latest-value mailbox).
//! `await` points are where the executor switches tasks: never block inside a task.
//! API level: `Input::wait_for_falling_edge` is a STABLE esp-hal method in 1.1.1 (the generic
//! `wait_for` is unstable); esp-rtos 0.3.0 and embassy-executor 0.10 are companion crates.
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Ticker, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    timer::timg::TimerGroup,
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

/// New blink half-period in milliseconds, written by `button_task`, read by `blinker`.
static BLINK_MS: Signal<CriticalSectionRawMutex, u64> = Signal::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    esp_alloc::heap_allocator!(size: 16 * 1024); // see Cargo.toml note: esp-rtos alloc support is on

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let led = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());
    let button = Input::new(peripherals.GPIO14, InputConfig::default().with_pull(Pull::Up));

    // `spawn` needs `Result<SpawnToken, SpawnError>::unwrap()` in embassy-executor 0.10.
    spawner.spawn(blinker(led).unwrap());
    spawner.spawn(button_task(button).unwrap());

    let mut ticker = Ticker::every(Duration::from_secs(2));
    let mut beats = 0u32;
    loop {
        ticker.next().await;
        beats += 1;
        info!("heartbeat #{}", beats);
    }
}

#[embassy_executor::task]
async fn blinker(mut led: Output<'static>) {
    let mut half_period = 500u64;
    loop {
        led.toggle();
        // Wait for EITHER the timer OR a new period, whichever happens first.
        match select(Timer::after_millis(half_period), BLINK_MS.wait()).await {
            Either::First(()) => {}
            Either::Second(new_ms) => {
                info!("blink half-period -> {} ms", new_ms);
                half_period = new_ms;
            }
        }
    }
}

#[embassy_executor::task]
async fn button_task(mut button: Input<'static>) {
    const PERIODS: [u64; 4] = [500, 250, 100, 1000];
    let mut idx = 0;
    loop {
        button.wait_for_falling_edge().await; // sleeps until the pin goes low
        idx = (idx + 1) % PERIODS.len();
        BLINK_MS.signal(PERIODS[idx]);
        Timer::after_millis(200).await; // crude debounce
    }
}
