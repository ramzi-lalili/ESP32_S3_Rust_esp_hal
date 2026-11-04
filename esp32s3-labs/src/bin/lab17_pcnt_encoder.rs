//! Lab 17 - PCNT pulse counter: rotary (quadrature) encoder
//!
//! Wiring (KY-040 style encoder, 3.3 V logic):
//!   CLK (A) -> GPIO38      DT (B) -> GPIO39      + -> 3V3      GND -> GND
//!   (internal pull-ups are enabled; add 10 k external ones if the module has none)
//! The PCNT hardware decodes both edges of A and B in hardware (x4 resolution), so no
//! CPU time is spent on edges. Two channels per unit implement the quadrature logic.
//! The 16-bit counter is extended to i32 by counting limit events in an interrupt.
//! API level: UNSTABLE (`esp_hal::pcnt`).
#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicI32, Ordering},
};

use critical_section::Mutex;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Pull},
    handler, main,
    pcnt::{Pcnt, channel, unit},
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

static UNIT0: Mutex<RefCell<Option<unit::Unit<'static, 1>>>> = Mutex::new(RefCell::new(None));
static OVERFLOW: AtomicI32 = AtomicI32::new(0);

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    let mut pcnt = Pcnt::new(peripherals.PCNT);
    pcnt.set_interrupt_handler(on_pcnt);

    let u0 = pcnt.unit1;
    u0.set_low_limit(Some(-100)).unwrap();
    u0.set_high_limit(Some(100)).unwrap();
    // Glitch filter: ignore pulses shorter than 1023 APB cycles (~12.8 us at 80 MHz).
    u0.set_filter(Some(1023)).unwrap();
    u0.clear();

    let cfg = InputConfig::default().with_pull(Pull::Up);
    let pin_a = Input::new(peripherals.GPIO38, cfg);
    let pin_b = Input::new(peripherals.GPIO39, cfg);
    let a = pin_a.peripheral_input();
    let b = pin_b.peripheral_input();

    // Channel 0: edges of A are counted, direction is controlled by level of B.
    let ch0 = &u0.channel0;
    ch0.set_ctrl_signal(b.clone());
    ch0.set_edge_signal(a.clone());
    ch0.set_ctrl_mode(channel::CtrlMode::Reverse, channel::CtrlMode::Keep);
    ch0.set_input_mode(channel::EdgeMode::Increment, channel::EdgeMode::Decrement);

    // Channel 1: the mirror image, counts the edges of B controlled by A.
    let ch1 = &u0.channel1;
    ch1.set_ctrl_signal(a);
    ch1.set_edge_signal(b);
    ch1.set_ctrl_mode(channel::CtrlMode::Reverse, channel::CtrlMode::Keep);
    ch1.set_input_mode(channel::EdgeMode::Decrement, channel::EdgeMode::Increment);

    u0.listen(); // enable the limit interrupts
    u0.resume(); // start counting
    let counter = u0.counter.clone();
    critical_section::with(|cs| UNIT0.borrow_ref_mut(cs).replace(u0));

    info!("Lab 17: turn the encoder");
    let mut last: i32 = 0;
    loop {
        let value = counter.get() as i32 + OVERFLOW.load(Ordering::SeqCst);
        if value != last {
            last = value;
            info!("encoder position: {} (about {} detents)", value, value / 4);
        }
        delay.delay_millis(20);
    }
}

#[handler]
fn on_pcnt() {
    critical_section::with(|cs| {
        let mut slot = UNIT0.borrow_ref_mut(cs);
        if let Some(u0) = slot.as_mut() {
            if u0.interrupt_is_set() {
                let events = u0.events();
                if events.high_limit {
                    OVERFLOW.fetch_add(100, Ordering::SeqCst);
                } else if events.low_limit {
                    OVERFLOW.fetch_add(-100, Ordering::SeqCst);
                }
                u0.reset_interrupt();
            }
        }
    });
}
