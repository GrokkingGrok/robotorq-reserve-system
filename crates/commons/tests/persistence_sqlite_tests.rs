#![cfg(feature = "persistence")]

use commons::util::persistence::SqliteDriver;

#[tokio::test]
async fn sqlite_basic_kv_roundtrip() {
    // Use an in-memory SQLite database for quick tests.
    let drv = SqliteDriver::new("sqlite::memory:")
        .await
        .expect("create driver");

    drv.put("k1", "v1").await.expect("put");
    let v = drv.get("k1").await.expect("get");
    assert_eq!(v, Some("v1".to_string()));

    drv.put("k1", "v2").await.expect("put2");
    let v2 = drv.get("k1").await.expect("get2");
    assert_eq!(v2, Some("v2".to_string()));

    drv.delete("k1").await.expect("delete");
    let v3 = drv.get("k1").await.expect("get3");
    assert_eq!(v3, None);
}
