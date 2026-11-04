fn main() {
    // Linker script handling is provided by esp-hal 1.x itself; this file only
    // adds a friendlier hint when the linker fails. Regenerate with
    // `esp-generate --chip esp32s3` and compare if the linker reports errors.
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}
