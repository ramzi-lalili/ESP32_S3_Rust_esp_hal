# esp32s3-labs

Source code of the **ESP32-S3 Embedded Rust Laboratory Manual** (esp-hal **1.1.1**, `no_std`, Xtensa).
One Cargo package, one binary per laboratory in `src/bin/`.

> **Status.** All 30 binaries compile with `cargo build --release` for `xtensa-esp32s3-none-elf`
> (0 warnings, see `docs/final_build_record.txt`). **Nothing has been run on real hardware.**
> Read each lab header for its verification status and API stability.

## Requirements
* Rust Xtensa toolchain (`espup install --targets esp32s3`), `. ~/export-esp.sh` sourced in every shell
* `espflash` 4.x (`cargo install espflash --locked` or `cargo binstall espflash`)
* An ESP32-S3 board; OV2640 camera pins and LCD1602 pins as given in chapter 7 of the manual

## Quick start
```bash
. "$HOME/export-esp.sh"
cargo run --release --bin lab01_gpio_output          # build + flash + monitor
cargo run --release --bin lab03_cpu_clock --features clk-160
SSID="MyWifi" PASSWORD="secret" cargo run --release --bin lab20_wifi
```

## Important rules
* `esp-hal` is pinned `=1.1.1`. Do not loosen it (1.1.2 and 1.2.x exist and differ).
* `embassy-sync` is pinned `0.7.2` because trouble-host 0.6.0 needs it (see `docs/ble_embassy_sync_mismatch_error.txt`).
* The camera pins (GPIO4-13, 15-18) are reserved with `reserve_camera_pins!`; LCD I2C is GPIO41 (SDA) / GPIO40 (SCL).
* Free GPIO pool for everything else: 1, 2, 14, 21, 38, 39, 42, 47, 48. Check your board schematic first.
* Use `--release` always.

## Contents
| Lab | Binary | Topic | Verification | API level |
|:----|:-------|:------|:-------------|:----------|
| 00 | `lab00_hello` | Hello world, logging and the build pipeline | BUILD-VERIFIED (generator output, unchanged) | esp-hal root, `clock`, `time` (STABLE) + |
| 01 | `lab01_gpio_output` | GPIO digital output: blinking an LED | BUILD-VERIFIED | STABLE |
| 02 | `lab02_gpio_input` | GPIO digital input, pull-up / pull-down and debouncing | BUILD-VERIFIED | STABLE |
| 03 | `lab03_cpu_clock` | CPU clock configuration | BUILD-VERIFIED | STABLE |
| 04 | `lab04_adc` | ADC analog-to-digital conversion | BUILD-VERIFIED | UNSTABLE |
| 05 | `lab05_ledc_pwm` | LEDC PWM: LED brightness control | BUILD-VERIFIED | UNSTABLE |
| 06 | `lab06_servo` | Servo motor control with PWM | BUILD-VERIFIED | UNSTABLE |
| 07 | `lab07_timer` | Hardware timers and periodic events | BUILD-VERIFIED | UNSTABLE |
| 08 | `lab08_gpio_interrupt` | GPIO interrupts and interrupt handlers | BUILD-VERIFIED | UNSTABLE |
| 09 | `lab09_uart` | UART serial communication | BUILD-VERIFIED | STABLE |
| 10 | `lab10_i2c_scan` | I2C master: bus scan | BUILD-VERIFIED | STABLE |
| 11 | `lab11_lcd1602` | LCD1602 on I2C (PCF8574 backpack) | BUILD-VERIFIED | STABLE I2C |
| 12 | `lab12_spi` | SPI master: full-duplex loopback | BUILD-VERIFIED | STABLE |
| 13 | `lab13_rc522` | RC522 RFID reader over SPI | BUILD-VERIFIED | STABLE SPI/GPIO |
| 14 | `lab14_dma_mem2mem` | DMA: memory-to-memory transfer | BUILD-VERIFIED | UNSTABLE |
| 14B | `lab14b_spi_dma` | DMA with SPI: full-duplex loopback | BUILD-VERIFIED | UNSTABLE |
| 15 | `lab15_watchdog` | Watchdog timer | BUILD-VERIFIED | UNSTABLE |
| 16 | `lab16_rmt_ws2812` | RMT peripheral: driving a WS2812 RGB LED | BUILD-VERIFIED | UNSTABLE |
| 17 | `lab17_pcnt_encoder` | PCNT pulse counter and rotary encoder | BUILD-VERIFIED | UNSTABLE |
| 18 | `lab18_mcpwm` | MCPWM: motor-control PWM signals | BUILD-VERIFIED | UNSTABLE |
| 19 | `lab19_twai` | TWAI (CAN 2.0) communication | BUILD-VERIFIED | UNSTABLE |
| 20 | `lab20_wifi` | Wi-Fi connectivity: station, DHCP and a TCP echo server | BUILD-VERIFIED (companion crates esp-radio 0 | esp-radio `wifi` API (not part of esp-ha |
| 21 | `lab21_ble` | Bluetooth Low Energy: GATT Battery Service | BUILD-VERIFIED (adapted from the official bas_peripheral example | esp-radio `ble` + `trouble-host` 0.6.0 ( |
| 22A | `lab22a_usb_serial_jtag` | USB Serial/JTAG: blocking echo | BUILD-VERIFIED | UNSTABLE |
| 22B | `lab22b_usb_otg_cdc` | USB OTG: CDC-ACM serial device with embassy-usb | BUILD-VERIFIED (adapted from the official embassy_usb_serial example) | UNSTABLE |
| 23 | `lab23_embassy` | Asynchronous programming with Embassy | BUILD-VERIFIED | `#[esp_rtos |
| 24 | `lab24_psram` | Bonus: external PSRAM as heap | BUILD-VERIFIED | UNSTABLE |
| P | `proj1_basic_monitor` | Project 1: ADC + PWM + LCD monitor | BUILD-VERIFIED | mixed |
| P | `proj2_rfid_access` | Project 2: RC522 + LCD + servo access control | BUILD-VERIFIED | mixed |
| P | `proj3_iot_monitor` | Project 3: sensors + LCD + Wi-Fi + Embassy dashboard | BUILD-VERIFIED | mixed |

## Evidence in `docs/`
`final_build_record.txt` (30 builds), `stable_only_check_record.txt` (labs built with the `unstable` feature off, plus negative controls),
`pin_conflict_compiler_output.txt`, `unstable_feature_missing_errors.txt`, `ble_embassy_sync_mismatch_error.txt`, `compile_fail/pin_conflict.rs`.
# ESP32_S3_Rust_hal_documentation
