# Verification report

Toolchain: Espressif Rust fork 1.95.0.0 (`rustc 1.95.0-nightly`), target `xtensa-esp32s3-none-elf`, Xtensa GCC 16.1.0 (crosstool-NG esp-16.1.0_20260609), esp-hal `=1.1.1` (Cargo.lock).

**Result: 30 of 30 binaries built successfully with 0 compiler warnings** (final record `docs/final_build_record.txt`).

| Binary | Build (`--release`) | Warnings | Code+data in flash (bytes) | Built without `unstable` |
|:--------------------------|:--------|:------:|----------:|:--------------------------------|
| `lab00_hello` | OK | 0 | 41,449 | not tested |
| `lab01_gpio_output` | OK | 0 | 42,025 | yes, OK |
| `lab02_gpio_input` | OK | 0 | 42,541 | yes, OK |
| `lab03_cpu_clock` | OK | 0 | 43,481 | yes, OK |
| `lab04_adc` | OK | 0 | 44,381 | NO: fails with E0603 (negative control) |
| `lab05_ledc_pwm` | OK | 0 | 46,493 | NO: fails with E0603 (negative control) |
| `lab06_servo` | OK | 0 | 44,993 | not applicable (uses unstable modules) |
| `lab07_timer` | OK | 0 | 45,097 | not applicable (uses unstable modules) |
| `lab08_gpio_interrupt` | OK | 0 | 43,389 | not applicable (uses unstable modules) |
| `lab09_uart` | OK | 0 | 58,765 | yes, OK |
| `lab10_i2c_scan` | OK | 0 | 62,321 | yes, OK |
| `lab11_lcd1602` | OK | 0 | 63,629 | not applicable (uses unstable modules) |
| `lab12_spi` | OK | 0 | 55,409 | yes, OK |
| `lab13_rc522` | OK | 0 | 63,685 | not applicable (uses unstable modules) |
| `lab14_dma_mem2mem` | OK | 0 | 57,745 | not applicable (uses unstable modules) |
| `lab14b_spi_dma` | OK | 0 | 69,673 | not applicable (uses unstable modules) |
| `lab15_watchdog` | OK | 0 | 44,141 | not applicable (uses unstable modules) |
| `lab16_rmt_ws2812` | OK | 0 | 53,013 | not applicable (uses unstable modules) |
| `lab17_pcnt_encoder` | OK | 0 | 45,625 | not applicable (uses unstable modules) |
| `lab18_mcpwm` | OK | 0 | 46,161 | not applicable (uses unstable modules) |
| `lab19_twai` | OK | 0 | 46,165 | not applicable (uses unstable modules) |
| `lab20_wifi` | OK | 0 | 504,273 | not tested (companion crates) |
| `lab21_ble` | OK | 0 | 335,141 | not tested (companion crates) |
| `lab22a_usb_serial_jtag` | OK | 0 | 34,605 | not applicable (uses unstable modules) |
| `lab22b_usb_otg_cdc` | OK | 0 | 75,397 | not tested (companion crates) |
| `lab23_embassy` | OK | 0 | 67,137 | not tested (companion crates) |
| `lab24_psram` | OK | 0 | 52,693 | not applicable (uses unstable modules) |
| `proj1_basic_monitor` | OK | 0 | 66,645 | not applicable (uses unstable modules) |
| `proj2_rfid_access` | OK | 0 | 76,665 | not applicable (uses unstable modules) |
| `proj3_iot_monitor` | OK | 0 | 538,725 | not tested (companion crates) |

**Additional evidence runs**

| Test | Expected | Observed |
|:--------------------------------------|:----------------------------|:------------------------------------------|
| Camera pin used twice (`docs/compile_fail/pin_conflict.rs`) | Compile error | `error[E0382]: use of moved value: peripherals.GPIO15` |
| Stable-labelled labs built with `unstable` off (labs 01, 02, 03, 09, 10, 12) | Build succeeds | All 6 succeeded |
| Unstable labs 04 and 05 built with `unstable` off | Build fails | `E0603: module analog is private` / `module ledc is private` |
| BLE lab with `embassy-sync 0.8` | (Predicted mismatch) | `E0277 NoopRawMutex ... RawMutex is not satisfied` + "multiple different versions of crate embassy_sync" |
| Every API in Part III exists in the tag | All resolve | 171 of 171 resolved, 0 errors |
| Pin audit of all source files | No camera/flash/strapping/UART0 pin used; USB pins only in Lab 22b | Passed (section 7.4) |

**Not verified:** behaviour on hardware (timing, electrical levels, RC522/LCD/servo/WS2812 operation, TWAI bus, Wi-Fi association and DHCP, BLE interoperability, USB enumeration, PSRAM).
