#[cfg(target_os = "windows")]
fn ensure_sidecar_stubs() {
    let target = std::env::var("TARGET").expect("TARGET should be available during build");
    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be available"),
    );
    let binaries_dir = manifest_dir.join("binaries");
    std::fs::create_dir_all(&binaries_dir).expect("binaries directory should be creatable");

    for binary_name in ["SuperRouteRepairService", "SuperRouteRepairBroker"] {
        let staged_path = binaries_dir.join(format!("{binary_name}-{target}.exe"));
        if !staged_path.exists() {
            std::fs::write(&staged_path, []).expect("sidecar stub should be writable");
        }
    }
}

fn main() {
    // Embed the packaged app manifest for release builds.
    // The UI process stays asInvoker; privilege elevation moves to repair-specific flows.
    #[cfg(target_os = "windows")]
    {
        ensure_sidecar_stubs();

        if std::env::var("PROFILE").unwrap_or_default() == "release" {
            let mut windows = tauri_build::WindowsAttributes::new();
            windows = windows.app_manifest(include_str!("super-route-pro.exe.manifest"));
            let attrs = tauri_build::Attributes::new().windows_attributes(windows);
            tauri_build::try_build(attrs).expect("failed to run tauri-build");
        } else {
            tauri_build::build();
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        tauri_build::build();
    }
}
