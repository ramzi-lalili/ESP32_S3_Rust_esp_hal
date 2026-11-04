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

// 50 Hz => 20 ms period. 1 ms pulse = 5 %, 1.5 ms = 7.5 %, 2 ms = 10 %.
// With an 80 MHz APB clock, 14-bit resolution is valid at 50 Hz (10-bit is not).
#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let mut ledc = Ledc::new(p.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut timer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    timer0
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty14Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(50),
        })
        .unwrap();

    let mut servo = ledc.channel::<LowSpeed>(channel::Number::Channel0, p.GPIO14);
    servo
        .configure(channel::config::Config {
            timer: &timer0,
            duty_pct: 5,
            drive_mode: DriveMode::PushPull,
        })
        .unwrap();

    let delay = Delay::new();
    loop {
        // set_duty takes whole percent: only 6 positions between 5 % and 10 %.
        for pct in 5..=10u8 {
            servo.set_duty(pct).unwrap();
            println!("servo duty {} %", pct);
            delay.delay_millis(600);
        }
        for pct in (5..10u8).rev() {
            servo.set_duty(pct).unwrap();
            delay.delay_millis(600);
        }
    }
}
