#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    delay::Delay,
    gpio::DriveMode,
    ledc::{
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
        LSGlobalClkSource, Ledc, LowSpeed,
    },
    main,
    time::Rate,
};

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let mut ledc = Ledc::new(p.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut timer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    timer0
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty8Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(5),
        })
        .unwrap();

    let mut ch0 = ledc.channel::<LowSpeed>(channel::Number::Channel0, p.GPIO2);
    ch0.configure(channel::config::Config {
        timer: &timer0,
        duty_pct: 0,
        drive_mode: DriveMode::PushPull,
    })
    .unwrap();

    let delay = Delay::new();
    loop {
        for pct in (0..=100u8).chain((0..100u8).rev()) {
            ch0.set_duty(pct).unwrap();
            delay.delay_millis(10);
        }
    }
}
