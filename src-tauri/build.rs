fn main() {
    // Embed the packaged app manifest for release builds.
    // The UI process stays asInvoker; privilege elevation moves to repair-specific flows.
    #[cfg(target_os = "windows")]
    {
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
