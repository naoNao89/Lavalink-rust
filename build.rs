fn main() {
    // Fix for musl builds: ensure math library is linked
    // This addresses the undefined reference errors for math functions
    // like exp, sqrt, sqrtf, log, log10 in audiopus_sys when building for musl
    if std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "musl" {
        println!("cargo:rustc-link-lib=m");

        // Set CMAKE environment variables to ensure libm is linked
        // during the audiopus_sys build process for musl targets
        println!("cargo:rustc-env=CMAKE_C_FLAGS=-lm");
        println!("cargo:rustc-env=CMAKE_EXE_LINKER_FLAGS=-lm");
        println!("cargo:rustc-env=CMAKE_SHARED_LINKER_FLAGS=-lm");
        println!("cargo:rustc-env=CMAKE_STATIC_LINKER_FLAGS=-lm");

        // Set environment variables that will be inherited by the build process
        std::env::set_var("CMAKE_C_FLAGS", "-lm");
        std::env::set_var("CMAKE_EXE_LINKER_FLAGS", "-lm");
        std::env::set_var("CMAKE_SHARED_LINKER_FLAGS", "-lm");
        std::env::set_var("CMAKE_STATIC_LINKER_FLAGS", "-lm");

        // Also set traditional C compiler flags
        let existing_cflags = std::env::var("CFLAGS").unwrap_or_default();
        let existing_ldflags = std::env::var("LDFLAGS").unwrap_or_default();

        std::env::set_var("CFLAGS", format!("{} -lm", existing_cflags));
        std::env::set_var("LDFLAGS", format!("{} -lm", existing_ldflags));
    }
}
