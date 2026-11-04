//! Lab 18 - MCPWM: motor-control PWM signals
//!
//! Wiring (H-bridge such as TB6612FNG / L298N / DRV8833, logic side only):
//!   PWM A -> GPIO38  (bridge IN1)     PWM B -> GPIO39  (bridge IN2)
//!   Motor supply and GND are on the bridge; share GND with the ESP32-S3.
//! With no motor you can watch both pins with a scope or LEDs (330 ohm).
//! One timer + one operator give two independently-controlled outputs:
//!   forward : A = duty ramp, B = 0     reverse : A = 0, B = duty ramp
//! API level: UNSTABLE (`esp_hal::mcpwm`). The complementary dead-time helper
//! `Operator::with_linked_pins` exists but is NOT used here: the module docs of
//! 1.1.1 still list dead time as "not yet implemented" - verify on a scope first.
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    main,
    mcpwm::{
        McPwm, PeripheralClockConfig,
        operator::PwmPinConfig,
        timer::PwmWorkingMode,
    },
    time::Rate,
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // 32 MHz peripheral clock derived from the 160 MHz MCPWM source clock.
    let clock_cfg = PeripheralClockConfig::with_frequency(Rate::from_mhz(32)).unwrap();
    let mut mcpwm = McPwm::new(peripherals.MCPWM0, clock_cfg);

    // Operator 0 follows timer 0.
    mcpwm.operator0.set_timer(&mcpwm.timer0);
    let (mut pwm_a, mut pwm_b) = mcpwm.operator0.with_pins(
        peripherals.GPIO38,
        PwmPinConfig::UP_ACTIVE_HIGH,
        peripherals.GPIO39,
        PwmPinConfig::UP_ACTIVE_HIGH,
    );

    // Timer period register = 99 -> timestamps 0..=100 are percent duty.
    // 20 kHz is above the audible range, typical for small DC motors.
    let timer_cfg = clock_cfg
        .timer_clock_with_frequency(99, PwmWorkingMode::Increase, Rate::from_khz(20))
        .unwrap();
    mcpwm.timer0.start(timer_cfg);

    info!("Lab 18: MCPWM 20 kHz, ramping A then B");
    loop {
        pwm_b.set_timestamp(0);
        for duty in (0..=100).step_by(5) {
            pwm_a.set_timestamp(duty);
            info!("forward duty {} %", duty);
            delay.delay_millis(200);
        }
        pwm_a.set_timestamp(0);
        for duty in (0..=100).step_by(5) {
            pwm_b.set_timestamp(duty);
            info!("reverse duty {} %", duty);
            delay.delay_millis(200);
        }
        pwm_b.set_timestamp(0);
        delay.delay_millis(1000);
    }
}
