use super_route_pro_lib::repair_session::RepairSessionManager;

#[test]
fn repair_session_is_locked_by_default() {
    let manager = RepairSessionManager::new();
    let status = manager.status();

    assert!(status.locked, "new sessions should start locked");
    assert!(
        status.requires_unlock,
        "new sessions should require unlock before privileged work"
    );
    assert!(
        !status.connected,
        "new sessions should not report an active repair connection"
    );
    assert_eq!(status.target_sid, None);
}

#[test]
fn repair_session_unlocks_for_an_app_instance() {
    let mut manager = RepairSessionManager::new();

    manager.unlock("app-1", "conn-1");
    let status = manager.status();

    assert!(!status.locked, "unlock should open the repair session");
    assert!(
        !status.requires_unlock,
        "unlock should clear the requires_unlock gate"
    );
    assert!(status.connected, "unlock should bind an active connection");
}

#[test]
fn repair_session_locks_again_on_explicit_close() {
    let mut manager = RepairSessionManager::new();

    manager.unlock("app-1", "conn-1");
    manager.lock();
    let status = manager.status();

    assert!(status.locked, "explicit close should lock the session again");
    assert!(
        status.requires_unlock,
        "explicit close should require a fresh unlock"
    );
    assert!(
        !status.connected,
        "explicit close should drop the active connection"
    );
}

#[test]
fn repair_session_locks_again_when_the_active_connection_disconnects() {
    let mut manager = RepairSessionManager::new();

    manager.unlock("app-1", "conn-1");
    manager.on_disconnect("other-conn");
    assert!(
        !manager.status().locked,
        "disconnecting a different connection should not change the unlocked session"
    );

    manager.on_disconnect("conn-1");
    let status = manager.status();
    assert!(status.locked, "disconnecting the active connection should lock");
    assert!(
        status.requires_unlock,
        "disconnecting the active connection should require a fresh unlock"
    );
    assert!(
        !status.connected,
        "disconnecting the active connection should clear connected state"
    );
}

#[test]
fn unlock_nonce_transitions_session_from_locked_to_unlocked() {
    let mut manager = RepairSessionManager::new();

    let request = manager.issue_unlock_request("app-1", "conn-1");
    assert!(manager.status().locked, "issuing a nonce should not unlock by itself");

    let response = manager.unlock_with_request(&request);
    assert!(response.unlocked, "matching nonce should unlock the session");
    assert!(
        !manager.status().locked,
        "successful nonce verification should open the repair session"
    );
    assert!(
        !manager.status().requires_unlock,
        "successful nonce verification should clear the unlock gate"
    );
}
