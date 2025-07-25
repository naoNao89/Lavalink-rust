fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let _target_family = std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default();

    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ENV");

    // Debug output for build detection
    println!("cargo:warning=Building for target: {target_arch}-{target_os}-{target_env}");

    // Set up cross-compilation environment variables early
    setup_cross_compilation_env(&target_arch, &target_os, &target_env);

    // Handle musl builds: ensure math library is linked
    // This addresses the undefined reference errors for math functions
    // like exp, sqrt, sqrtf, log, log10 in audiopus_sys when building for musl
    if target_env == "musl" {
        println!("cargo:rustc-link-lib=m");
        println!("cargo:warning=Applied musl math library linking fix");
    }

    // Architecture-specific configurations
    match target_arch.as_str() {
        "x86_64" => {
            println!("cargo:warning=Configuring for x86_64 architecture");
            configure_x86_64(&target_os, &target_env);
        }
        "aarch64" => {
            println!("cargo:warning=Configuring for aarch64 (ARM64) architecture");
            configure_aarch64(&target_os, &target_env);
        }
        "x86" => {
            println!("cargo:warning=Configuring for x86 (32-bit) architecture");
            configure_x86(&target_os, &target_env);
        }
        arch => {
            println!("cargo:warning=Unknown architecture: {arch}, using default configuration");
        }
    }

    // Platform-specific configurations
    match target_os.as_str() {
        "macos" => configure_macos(&target_arch),
        "linux" => configure_linux(&target_arch, &target_env),
        "windows" => configure_windows(&target_arch),
        os => {
            println!("cargo:warning=Unknown target OS: {os}, using default configuration");
        }
    }
}

fn configure_x86_64(target_os: &str, target_env: &str) {
    // Intel/x86_64 specific configurations
    match target_os {
        "linux" => {
            // Most of our dependencies are pure Rust and don't need C++ linking
            // Only link what's actually needed for specific features
            if target_env != "musl" {
                // Only for glibc builds if needed by specific dependencies
                // println!("cargo:rustc-link-lib=dylib=stdc++");
            }
        }
        "windows" => {
            // Windows-specific x86_64 configurations
            println!("cargo:rustc-link-lib=dylib=msvcrt");
        }
        "macos" => {
            // Intel macOS specific configurations
            // Set environment variables to help with cross-compilation issues
            println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.15");

            // Help with CMake issues for audio dependencies
            if std::env::var("CMAKE_OSX_ARCHITECTURES").is_err() {
                println!("cargo:rustc-env=CMAKE_OSX_ARCHITECTURES=x86_64");
            }

            // Ensure proper linking for Intel Mac
            println!("cargo:rustc-link-lib=framework=Security");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
            println!("cargo:rustc-link-lib=framework=AudioToolbox");
            println!("cargo:rustc-link-lib=framework=AudioUnit");
        }
        _ => {}
    }
}

fn configure_aarch64(target_os: &str, _target_env: &str) {
    // ARM64 specific configurations
    match target_os {
        "linux" => {
            // ARM64 Linux may need additional math library linking
            println!("cargo:rustc-link-lib=m");
        }
        "macos" => {
            // macOS ARM64 (Apple Silicon) configurations
            // Usually works well out of the box, but ensure proper framework linking
            println!("cargo:rustc-link-lib=framework=Security");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
        }
        _ => {}
    }
}

fn configure_x86(target_os: &str, _target_env: &str) {
    // 32-bit x86 specific configurations
    match target_os {
        "linux" => {
            // 32-bit builds may need additional linking
            println!("cargo:rustc-link-lib=m");
        }
        "windows" => {
            println!("cargo:rustc-link-lib=dylib=msvcrt");
        }
        _ => {}
    }
}

fn configure_macos(target_arch: &str) {
    // macOS-specific configurations
    println!("cargo:rustc-link-lib=framework=Security");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");

    match target_arch {
        "aarch64" => {
            // Apple Silicon specific
            println!("cargo:warning=Configuring for Apple Silicon (M1/M2)");
        }
        "x86_64" => {
            // Intel Mac specific
            println!("cargo:warning=Configuring for Intel Mac");
            // Intel Macs might need additional configurations
        }
        _ => {}
    }
}

fn configure_linux(target_arch: &str, target_env: &str) {
    // Linux-specific configurations
    if target_env == "musl" {
        // musl-specific configurations
        println!("cargo:rustc-link-lib=m");
        if target_arch == "x86_64" {
            // Additional x86_64 musl configurations
            // Most dependencies are pure Rust, avoid unnecessary static linking
            // println!("cargo:rustc-link-lib=static=gcc");
        }
    } else {
        // glibc configurations
        match target_arch {
            "x86_64" => {
                // Standard x86_64 Linux
                println!("cargo:rustc-link-lib=dylib=dl");
            }
            "aarch64" => {
                // ARM64 Linux
                println!("cargo:rustc-link-lib=dylib=dl");
                println!("cargo:rustc-link-lib=m");
            }
            _ => {}
        }
    }
}

fn configure_windows(target_arch: &str) {
    // Windows-specific configurations
    match target_arch {
        "x86_64" => {
            // 64-bit Windows
            println!("cargo:rustc-link-lib=dylib=ws2_32");
            println!("cargo:rustc-link-lib=dylib=userenv");
        }
        "x86" => {
            // 32-bit Windows
            println!("cargo:rustc-link-lib=dylib=ws2_32");
            println!("cargo:rustc-link-lib=dylib=userenv");
        }
        _ => {}
    }
}

fn setup_cross_compilation_env(target_arch: &str, target_os: &str, target_env: &str) {
    // Set up environment variables to help with cross-compilation issues

    if target_os == "macos" && target_arch == "x86_64" {
        // Intel macOS cross-compilation from ARM64
        println!("cargo:warning=Setting up Intel macOS cross-compilation environment");

        // Set deployment target for compatibility
        if std::env::var("MACOSX_DEPLOYMENT_TARGET").is_err() {
            std::env::set_var("MACOSX_DEPLOYMENT_TARGET", "10.15");
            println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.15");
        }

        // Help CMake find the right architecture
        if std::env::var("CMAKE_OSX_ARCHITECTURES").is_err() {
            std::env::set_var("CMAKE_OSX_ARCHITECTURES", "x86_64");
            println!("cargo:rustc-env=CMAKE_OSX_ARCHITECTURES=x86_64");
        }

        // Set CMake system processor
        if std::env::var("CMAKE_SYSTEM_PROCESSOR").is_err() {
            std::env::set_var("CMAKE_SYSTEM_PROCESSOR", "x86_64");
            println!("cargo:rustc-env=CMAKE_SYSTEM_PROCESSOR=x86_64");
        }

        // Help with CMake minimum version issues
        if std::env::var("CMAKE_POLICY_VERSION_MINIMUM").is_err() {
            std::env::set_var("CMAKE_POLICY_VERSION_MINIMUM", "3.5");
            println!("cargo:rustc-env=CMAKE_POLICY_VERSION_MINIMUM=3.5");
        }
    }

    if target_env == "musl" {
        // musl-specific environment setup
        println!("cargo:warning=Setting up musl cross-compilation environment");

        // Help with static linking
        if std::env::var("OPENSSL_STATIC").is_err() {
            std::env::set_var("OPENSSL_STATIC", "1");
            println!("cargo:rustc-env=OPENSSL_STATIC=1");
        }

        if std::env::var("OPENSSL_VENDORED").is_err() {
            std::env::set_var("OPENSSL_VENDORED", "1");
            println!("cargo:rustc-env=OPENSSL_VENDORED=1");
        }

        // Fix for audiopus_sys CMake build with musl
        // Tell CMake to not try to link C++ standard library
        if std::env::var("CMAKE_CXX_STANDARD_LIBRARIES").is_err() {
            std::env::set_var("CMAKE_CXX_STANDARD_LIBRARIES", "");
            println!("cargo:rustc-env=CMAKE_CXX_STANDARD_LIBRARIES=");
        }

        // Disable C++ linking for Opus build
        if std::env::var("CMAKE_CXX_FLAGS").is_err() {
            std::env::set_var("CMAKE_CXX_FLAGS", "-nostdlib++");
            println!("cargo:rustc-env=CMAKE_CXX_FLAGS=-nostdlib++");
        }

        // Use C compiler for C++ files in Opus (since it's mostly C anyway)
        if std::env::var("CMAKE_CXX_COMPILER").is_err() {
            if let Ok(cc) = std::env::var("CC") {
                std::env::set_var("CMAKE_CXX_COMPILER", &cc);
                println!("cargo:rustc-env=CMAKE_CXX_COMPILER={cc}");
            }
        }

        // Additional Opus-specific environment variables for musl
        if std::env::var("OPUS_STATIC").is_err() {
            std::env::set_var("OPUS_STATIC", "1");
            println!("cargo:rustc-env=OPUS_STATIC=1");
        }

        // Disable C++ features in Opus build
        if std::env::var("CMAKE_DISABLE_CXX").is_err() {
            std::env::set_var("CMAKE_DISABLE_CXX", "1");
            println!("cargo:rustc-env=CMAKE_DISABLE_CXX=1");
        }

        // Tell CMake to use C linker instead of C++ linker
        if std::env::var("CMAKE_LINKER_TYPE").is_err() {
            std::env::set_var("CMAKE_LINKER_TYPE", "DEFAULT");
            println!("cargo:rustc-env=CMAKE_LINKER_TYPE=DEFAULT");
        }

        // Force CMake to treat everything as C, not C++
        if std::env::var("CMAKE_C_COMPILER_FORCED").is_err() {
            std::env::set_var("CMAKE_C_COMPILER_FORCED", "1");
            println!("cargo:rustc-env=CMAKE_C_COMPILER_FORCED=1");
        }

        // Alternative approach: tell audiopus_sys to use system opus if available
        if std::env::var("LIBOPUS_STATIC").is_err() {
            std::env::set_var("LIBOPUS_STATIC", "1");
            println!("cargo:rustc-env=LIBOPUS_STATIC=1");
        }
    }

    // Linux cross-compilation setup
    if target_os == "linux"
        && target_arch != std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default()
    {
        println!("cargo:warning=Setting up Linux cross-compilation environment");

        // Help with pkg-config for cross-compilation
        let pkg_config_var = format!("PKG_CONFIG_ALLOW_CROSS_{}", target_arch.replace('-', "_"));
        if std::env::var(&pkg_config_var).is_err() {
            std::env::set_var(&pkg_config_var, "1");
            println!("cargo:rustc-env={pkg_config_var}=1");
        }
    }
}
