use std::path::Path;

fn main() {
    // Declare the custom cfg key so rustc does not warn about unexpected_cfgs.
    println!("cargo::rustc-check-cfg=cfg(snap7_available)");

    // In test builds, skip snap7 link directives entirely — tests use PlcClient::new_mock().
    if std::env::var("CARGO_CFG_TEST").is_ok() {
        return;
    }

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let lib_dir = "libs/snap7";

    println!("cargo:rustc-link-search=native={}", lib_dir);

    // Detect whether the snap7 native library is present for this platform.
    // Emits `snap7_available` cfg so poller.rs can select mock vs real client
    // at compile time — preventing FFI calls when the dylib is absent at runtime.
    let snap7_exists = match target_os.as_str() {
        "macos" => Path::new(&format!("{}/libsnap7.dylib", lib_dir)).exists(),
        "windows" => Path::new(&format!("{}/snap7.lib", lib_dir)).exists(),
        _ => Path::new(&format!("{}/libsnap7.so", lib_dir)).exists(),
    };

    if snap7_exists {
        println!("cargo:rustc-cfg=snap7_available");
    }

    if target_os == "macos" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
        // Allow linking without libsnap7.dylib present during development.
        // Undefined snap7 symbols are never reached when snap7_available is unset
        // (poller uses PlcClient::new_mock() in that case).
        println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");

        if snap7_exists {
            println!("cargo:rustc-link-lib=dylib=snap7");
        }
    } else if target_os == "windows" {
        if snap7_exists {
            println!("cargo:rustc-link-lib=snap7");
        }
    } else {
        if snap7_exists {
            println!("cargo:rustc-link-lib=snap7");
        }
    }
}
