use std::path::PathBuf;

use super_route_pro_lib::repair_targets::{
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
