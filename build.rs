use std::path::Path;

fn main() {
    // In test builds, skip snap7 link directives entirely — tests use PlcClient::new_mock().
    if std::env::var("CARGO_CFG_TEST").is_ok() {
        return;
    }

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let lib_dir = "libs/snap7";

    println!("cargo:rustc-link-search=native={}", lib_dir);

    if target_os == "macos" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
        // Allow linking without libsnap7.dylib present during development.
        // Undefined snap7 symbols are resolved at runtime; all call sites are
        // guarded by mock_mode so they are never reached in mock builds.
        println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");

        // Only request explicit library linking when the dylib is actually present.
        if Path::new(&format!("{}/libsnap7.dylib", lib_dir)).exists() {
            println!("cargo:rustc-link-lib=dylib=snap7");
        }
    } else if target_os == "windows" {
        if Path::new(&format!("{}/snap7.lib", lib_dir)).exists() {
            println!("cargo:rustc-link-lib=snap7");
        }
    } else {
        // Linux and others
        if Path::new(&format!("{}/libsnap7.so", lib_dir)).exists() {
            println!("cargo:rustc-link-lib=snap7");
        }
    }
}
