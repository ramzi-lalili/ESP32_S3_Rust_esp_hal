//! Board pin map and camera-pin reservation.
//!
//! The OV2640 camera occupies fourteen GPIOs. In Rust the cleanest way to
//! guarantee that no other driver can claim them is to *move the pin tokens*
//! out of `Peripherals` into a struct that nobody else can access. The
//! compiler then rejects any later attempt to use the same pin
//! (`use of moved value: peripherals.GPIO15`).
//!
//! | Signal | GPIO | Signal | GPIO |
//! |--------|------|--------|------|
//! | XCLK   | 15   | D4     | 12   |
//! | SIOD   | 4    | D5     | 18   |
//! | SIOC   | 5    | D6     | 17   |
//! | D0     | 11   | D7     | 16   |
//! | D1     | 9    | VSYNC  | 6    |
//! | D2     | 8    | HREF   | 7    |
//! | D3     | 10   | PCLK   | 13   |
use esp_hal::peripherals::{
    GPIO4, GPIO5, GPIO6, GPIO7, GPIO8, GPIO9, GPIO10, GPIO11, GPIO12, GPIO13, GPIO15, GPIO16,
    GPIO17, GPIO18,
};

/// All GPIO tokens that belong to the OV2640 camera interface.
///
/// Holding a value of this type for the whole program lifetime keeps the pins
/// reserved. Nothing here configures the pins; they stay in their reset state.
pub struct CameraPins<'d> {
    pub xclk: GPIO15<'d>,
    pub siod: GPIO4<'d>,
    pub sioc: GPIO5<'d>,
    pub d0: GPIO11<'d>,
    pub d1: GPIO9<'d>,
    pub d2: GPIO8<'d>,
    pub d3: GPIO10<'d>,
    pub d4: GPIO12<'d>,
    pub d5: GPIO18<'d>,
    pub d6: GPIO17<'d>,
    pub d7: GPIO16<'d>,
    pub vsync: GPIO6<'d>,
    pub href: GPIO7<'d>,
    pub pclk: GPIO13<'d>,
}

/// Move the camera GPIO tokens out of a `Peripherals` value.
///
/// This is a macro (not a function) so that only the *fields* are moved and
/// the rest of `peripherals` stays usable (Rust "partial move").
#[macro_export]
macro_rules! reserve_camera_pins {
    ($p:ident) => {
        $crate::board::CameraPins {
            xclk: $p.GPIO15,
            siod: $p.GPIO4,
            sioc: $p.GPIO5,
            d0: $p.GPIO11,
            d1: $p.GPIO9,
            d2: $p.GPIO8,
            d3: $p.GPIO10,
            d4: $p.GPIO12,
            d5: $p.GPIO18,
            d6: $p.GPIO17,
            d7: $p.GPIO16,
            vsync: $p.GPIO6,
            href: $p.GPIO7,
            pclk: $p.GPIO13,
        }
    };
}
