#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};
use critical_section::Mutex;
use esp_hal::{
    delay::Delay,
    gpio::{Event, Input, InputConfig, Io, Pull},
    handler, main,
};

static BUTTON: Mutex<RefCell<Option<Input<'static>>>> = Mutex::new(RefCell::new(None));
static PRESSES: AtomicU32 = AtomicU32::new(0);

#[handler]
fn gpio_isr() {
    critical_section::with(|cs| {
        if let Some(btn) = BUTTON.borrow_ref_mut(cs).as_mut() {
            if btn.is_interrupt_set() {
                PRESSES.fetch_add(1, Ordering::Relaxed);
                btn.clear_interrupt(); // the driver does NOT clear the status for you
            }
        }
    });
}

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let mut io = Io::new(p.IO_MUX);
    io.set_interrupt_handler(gpio_isr);

    let mut button = Input::new(p.GPIO21, InputConfig::default().with_pull(Pull::Up));
    critical_section::with(|cs| {
        button.listen(Event::FallingEdge);
        BUTTON.borrow_ref_mut(cs).replace(button);
    });

    let delay = Delay::new();
    let mut last = 0;
    loop {
        let n = PRESSES.load(Ordering::Relaxed);
        if n != last {
            println!("presses (incl. bounces): {}", n);
            last = n;
        }
        delay.delay_millis(50);
    }
}
