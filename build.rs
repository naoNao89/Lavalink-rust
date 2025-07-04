fn main() {
    // Fix for musl builds: ensure math library is linked
    // This addresses the undefined reference errors for math functions
    // like exp, sqrt, sqrtf, log, log10 in audiopus_sys when building for musl
    if std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "musl" {
        println!("cargo:rustc-link-lib=m");
    }
}
