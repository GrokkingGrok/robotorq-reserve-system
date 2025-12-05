use commons::util::persistence::InMemoryDriver;

#[test]
fn put_get_delete_roundtrip() {
    let drv = InMemoryDriver::new();
    assert!(drv.put("alpha", "one").is_ok());
    assert_eq!(drv.get("alpha"), Some("one".to_string()));
    assert!(drv.delete("alpha"));
    assert_eq!(drv.get("alpha"), None);
}

#[tokio::test]
async fn concurrent_puts_and_reads() {
    let drv = InMemoryDriver::new();
    let mut handles = Vec::new();

    for i in 0..20 {
        let d = drv.clone();
        handles.push(tokio::spawn(async move {
            let key = format!("k{i}");
            d.put(&key, "x").unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    for i in 0..20 {
        let key = format!("k{i}");
        assert_eq!(drv.get(&key), Some("x".to_string()));
    }
}
