use serde_json::Value;

fn read_json(path: &str) -> Value {
    let content = std::fs::read_to_string(path).expect("json file should be readable");
    serde_json::from_str(&content).expect("json file should parse")
}

#[test]
fn installer_packaging_bundles_required_sidecars() {
    let tauri_config = read_json("tauri.conf.json");
    let package_json = read_json("../package.json");
    let cargo_toml = std::fs::read_to_string("Cargo.toml").expect("Cargo.toml should be readable");
    let installer_hooks =
        std::fs::read_to_string("installer-hooks.nsh").expect("installer hooks should be readable");

    let external_bins = tauri_config["bundle"]["externalBin"]
        .as_array()
        .expect("bundle.externalBin should be configured");
    let external_bin_values: Vec<&str> = external_bins
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("bundle.externalBin entries should be strings")
        })
        .collect();
    assert!(
        external_bin_values.contains(&"binaries/SuperRouteRepairBroker"),
        "repair broker sidecar should be bundled with the installer"
    );
    assert!(
        external_bin_values.contains(&"binaries/SuperRouteService"),
        "route persistence service sidecar should be bundled with the installer"
    );
    assert_eq!(
        external_bin_values.len(),
        2,
        "the release bundle should stage both the repair broker and route persistence service sidecars"
    );

    let bundle_targets = tauri_config["bundle"]["targets"]
        .as_array()
        .expect("bundle.targets should be an array");
    assert_eq!(
        bundle_targets
            .iter()
            .map(|value| value.as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["nsis"],
        "release packaging should only ship the installer path used for the elevated repair broker flow"
    );

    let before_build_command = tauri_config["build"]["beforeBuildCommand"]
        .as_str()
        .expect("beforeBuildCommand should be configured");
    assert!(
        before_build_command.contains("prepare:repair-sidecars"),
        "tauri build should prepare the repair sidecars before bundling"
    );

    let prepare_script = package_json["scripts"]["prepare:repair-sidecars"]
        .as_str()
        .expect("package.json should define prepare:repair-sidecars");
    assert!(
        prepare_script.contains("prepare-repair-sidecars.ps1"),
        "prepare:repair-sidecars should build and stage the repair sidecars"
    );
    assert!(
        cargo_toml.contains("default-run = \"SuperRoute\""),
        "Cargo.toml should mark SuperRoute as the main bundle binary when sidecar bins are present"
    );

    assert!(
        !installer_hooks.contains("NSIS_HOOK_POSTINSTALL"),
        "installer hooks should no longer try to install a placeholder Windows service"
    );
    assert!(
        installer_hooks.contains("NSIS_HOOK_PREUNINSTALL"),
        "installer hooks should clean up the route service before uninstall removes the binaries"
    );
    assert!(
        installer_hooks.contains("SuperRouteProRouteService"),
        "installer hooks should stop or delete the installed route service during update/uninstall"
    );
}
