use commons::util::persistence::InMemoryDriver;

#[test]
fn checkpoint_and_restore_roundtrip() {
    let drv = InMemoryDriver::new();
    drv.put("a", "1").unwrap();
    drv.put("b", "2").unwrap();

    let snap = drv.create_checkpoint();

    // mutate store
    drv.put("c", "3").unwrap();
    let _ = drv.delete("a");
    assert_eq!(drv.get("c"), Some("3".to_string()));
    assert_eq!(drv.get("a"), None);

    // restore
    drv.restore_checkpoint(snap).unwrap();
    assert_eq!(drv.get("a"), Some("1".to_string()));
    assert_eq!(drv.get("b"), Some("2".to_string()));
    assert_eq!(drv.get("c"), None);
}
