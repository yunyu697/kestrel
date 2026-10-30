use std::{env, fs, path::Path};

fn main() {
    // Expose version / build number / build time to the backend at compile time.
    let info_path = Path::new("..").join("build-info.json");
    println!("cargo:rerun-if-changed=../build-info.json");
    println!("cargo:rerun-if-changed=build.rs");

    let (version, build, built_at) = match fs::read_to_string(&info_path) {
        Ok(raw) => {
            let v: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
            (
                v["version"].as_str().unwrap_or("1.0.0").to_string(),
                v["build"].as_u64().unwrap_or(0).to_string(),
                v["builtAt"].as_str().unwrap_or("unknown").to_string(),
            )
        }
        Err(_) => (
            env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "1.0.0".into()),
            "0".to_string(),
            "unknown".to_string(),
        ),
    };

    println!("cargo:rustc-env=KESTREL_VERSION={version}");
    println!("cargo:rustc-env=KESTREL_BUILD={build}");
    println!("cargo:rustc-env=KESTREL_BUILT_AT={built_at}");

    tauri_build::build()
}
