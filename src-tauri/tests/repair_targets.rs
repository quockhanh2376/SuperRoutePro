use std::path::PathBuf;

use super_route_pro_lib::repair_actions::{
    build_profile_cleanup_plan_for_target,
    validate_profile_cleanup_request,
};
use super_route_pro_lib::repair_protocol::ProfileCleanupRequest;
use super_route_pro_lib::repair_targets::{
    RepairTargetUser,
    normalize_profile_root,
    validate_target_sid,
};

#[test]
fn repair_targets_accept_valid_windows_sid_shapes() {
    assert!(validate_target_sid("S-1-5-18"));
    assert!(validate_target_sid("S-1-5-21-111111111-222222222-333333333-1001"));
}

#[test]
fn repair_targets_reject_invalid_windows_sid_shapes() {
    assert!(!validate_target_sid(""));
    assert!(!validate_target_sid("1-5-21-1001"));
    assert!(!validate_target_sid("S-1-5-ABC"));
    assert!(!validate_target_sid("S-1-5--1001"));
}

#[test]
fn repair_targets_normalize_profile_roots() {
    let normalized = normalize_profile_root(r" c:/Users/demo-user\ ")
        .expect("profile root should normalize");
    assert_eq!(normalized, PathBuf::from(r"C:\Users\demo-user"));
}

#[test]
fn repair_targets_reject_non_profile_roots() {
    assert_eq!(normalize_profile_root(r"C:\Windows\Temp"), None);
    assert_eq!(normalize_profile_root(r"C:\Users\demo-user\AppData\Local"), None);
    assert_eq!(normalize_profile_root(r"..\Users\demo-user"), None);
}

#[test]
fn target_cleanup_requires_target_sid() {
    let request = ProfileCleanupRequest {
        target_sid: "".to_string(),
        targets: vec!["user_temp".to_string()],
    };

    assert!(validate_profile_cleanup_request(&request).is_err());
}

#[test]
fn target_cleanup_resolves_profile_sensitive_paths_from_target_user() {
    let request = ProfileCleanupRequest {
        target_sid: "S-1-5-21-1001".to_string(),
        targets: vec!["user_temp".to_string(), "edge_cache".to_string()],
    };
    let target_user = RepairTargetUser {
        sid: "S-1-5-21-1001".to_string(),
        account_name: "demo-user".to_string(),
        profile_path: r"C:\Users\demo-user".to_string(),
        is_loaded: true,
    };

    let plan = build_profile_cleanup_plan_for_target(&target_user, &request)
        .expect("cleanup plan should resolve from target profile path");

    assert!(plan.iter().any(|path| path == r"C:\Users\demo-user\AppData\Local\Temp"));
    assert!(plan.iter().any(|path| path == r"C:\Users\demo-user\AppData\Local\Microsoft\Edge\User Data\Default\Cache"));
    assert!(
        !plan.iter().any(|path| path.contains("ADM_")),
        "cleanup paths must come from the selected target profile, not the current process profile"
    );
}
