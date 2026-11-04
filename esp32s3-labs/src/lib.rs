//! Shared helpers for the ESP32-S3 / esp-hal 1.1.1 laboratory manual.
//!
//! * [`board`]   – camera-pin reservation (type-level GPIO conflict protection)
//! * [`lcd1602`] – minimal HD44780 driver for the PCF8574 I²C "backpack"
#![no_std]

pub mod board;
pub mod lcd1602;
