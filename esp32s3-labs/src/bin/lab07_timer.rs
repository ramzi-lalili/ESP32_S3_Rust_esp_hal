//! Lab 07 - Hardware timers and periodic events (TIMG0 timer + interrupt)
//!
//! Wiring: GPIO21 --[330 ohm]--> LED --> GND (toggled from the timer interrupt).
//! The CPU is free to do other work (here: print a counter) while the timer
//! peripheral raises an interrupt every 250 ms.
//! API level: UNSTABLE (`esp_hal::timer`, `esp_hal::handler`).
#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use critical_section::Mutex;
use esp_backtrace as _;
use esp_hal::{
    Blocking,
    gpio::{Level, Output, OutputConfig},
    handler,
    main, ram,
    time::{Duration, Instant},
    timer::{PeriodicTimer, timg::TimerGroup},
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

// Resources shared between `main` and the interrupt handler live in statics
// behind a critical-section Mutex<RefCell<Option<..>>>.
static TIMER: Mutex<RefCell<Option<PeriodicTimer<'static, Blocking>>>> =
    Mutex::new(RefCell::new(None));
static LED: Mutex<RefCell<Option<Output<'static>>>> = Mutex::new(RefCell::new(None));
static TICKS: AtomicU32 = AtomicU32::new(0);

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let led = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    // A timer group contains two general-purpose timers and a watchdog.
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut periodic = PeriodicTimer::new(timg0.timer0);

    critical_section::with(|cs| {
        periodic.set_interrupt_handler(on_timer); // register the ISR
        periodic.listen(); // unmask the timer interrupt
        periodic.start(Duration::from_millis(250)).unwrap(); // auto-reload
        TIMER.borrow_ref_mut(cs).replace(periodic);
        LED.borrow_ref_mut(cs).replace(led);
    });

    info!("Lab 07: timer interrupt every 250 ms");
    let mut last = 0;
    loop {
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(1000) {}
        let now = TICKS.load(Ordering::Relaxed);
        info!("ticks so far: {} (+{} in the last second)", now, now - last);
        last = now;
    }
}

/// Interrupt service routine. `#[ram]` keeps it in IRAM so it can run even
/// while flash cache is busy.
#[handler]
#[ram]
fn on_timer() {
    critical_section::with(|cs| {
        // 1) Acknowledge the interrupt, otherwise it fires again immediately.
        TIMER.borrow_ref_mut(cs).as_mut().unwrap().clear_interrupt();
        // 2) Do the (short!) work.
        LED.borrow_ref_mut(cs).as_mut().unwrap().toggle();
    });
    TICKS.fetch_add(1, Ordering::Relaxed);
}
