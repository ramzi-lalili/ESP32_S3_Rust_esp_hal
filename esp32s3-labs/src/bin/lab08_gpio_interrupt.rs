//! Lab 08 - GPIO interrupts and interrupt handlers
//!
//! Wiring: push button between GPIO14 and GND (internal pull-up is enabled),
//!         LED on GPIO21 (330 ohm to GND) toggles on each valid press.
//! The ISR does only the minimum (debounce, counter, flag); logging is done in `main`.
//! API level: UNSTABLE (`Io::set_interrupt_handler`, `Input::listen`, ...).
#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

use critical_section::Mutex;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull},
    handler, main, ram,
    time::Instant,
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

static BUTTON: Mutex<RefCell<Option<Input<'static>>>> = Mutex::new(RefCell::new(None));
static PRESSES: AtomicU32 = AtomicU32::new(0);
static LED_REQUEST: AtomicBool = AtomicBool::new(false);
static LAST_MS: AtomicU32 = AtomicU32::new(0);

const DEBOUNCE_MS: u32 = 50;

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // All GPIO interrupts share ONE handler; register it through `Io`.
    let mut io = Io::new(peripherals.IO_MUX);
    io.set_interrupt_handler(gpio_isr);

    let mut led = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());
    let mut button = Input::new(
        peripherals.GPIO14,
        InputConfig::default().with_pull(Pull::Up),
    );

    critical_section::with(|cs| {
        button.listen(Event::FallingEdge); // button press pulls the pin low
        BUTTON.borrow_ref_mut(cs).replace(button);
    });

    info!("Lab 08: press the button on GPIO14");
    let mut shown = 0;
    loop {
        // `swap(false)` reads the flag and clears it atomically.
        if LED_REQUEST.swap(false, Ordering::AcqRel) {
            led.toggle();
        }
        let n = PRESSES.load(Ordering::Relaxed);
        if n != shown {
            shown = n;
            info!("valid presses: {}", n);
        }
    }
}

#[handler]
#[ram]
fn gpio_isr() {
    let now_ms = Instant::now().duration_since_epoch().as_millis() as u32;

    critical_section::with(|cs| {
        let mut slot = BUTTON.borrow_ref_mut(cs);
        let button = slot.as_mut().unwrap();
        // The handler is shared by every GPIO: check that *our* pin fired.
        if button.is_interrupt_set() {
            if now_ms.wrapping_sub(LAST_MS.load(Ordering::Relaxed)) > DEBOUNCE_MS {
                LAST_MS.store(now_ms, Ordering::Relaxed);
                PRESSES.fetch_add(1, Ordering::Relaxed);
                LED_REQUEST.store(true, Ordering::Release);
            }
            button.clear_interrupt(); // always acknowledge
        }
    });
}
