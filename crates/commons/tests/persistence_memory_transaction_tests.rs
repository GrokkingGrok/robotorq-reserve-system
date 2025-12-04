use commons::util::persistence::InMemoryDriver;

#[test]
fn transaction_isolation_and_commit() {
    let drv = InMemoryDriver::new();
    // ensure key absent
    assert_eq!(drv.get("txkey"), None);

    let mut tx = drv.begin_transaction();
    tx.put("txkey", "v1");
    // staged change visible inside transaction
    assert_eq!(tx.get("txkey"), Some("v1".to_string()));
    // not yet visible to the live store
    assert_eq!(drv.get("txkey"), None);

    // commit
    tx.commit().unwrap();
    assert_eq!(drv.get("txkey"), Some("v1".to_string()));
}

#[test]
fn transaction_abort_does_not_apply() {
    let drv = InMemoryDriver::new();

    let mut tx = drv.begin_transaction();
    tx.put("txkey2", "v2");
    // abort by calling abort()
    tx.abort();
    // not visible after abort
    assert_eq!(drv.get("txkey2"), None);
}
