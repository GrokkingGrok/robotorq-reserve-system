#![cfg(feature = "persistence")]

use commons::util::persistence::SqliteDriver;
use sqlx::Row;

#[tokio::test]
async fn sqlite_transaction_commit_and_rollback() {
    let drv = SqliteDriver::new("sqlite::memory:")
        .await
        .expect("create driver");

    // Commit path: acquire connection, begin transaction, run statements, commit
    let mut conn = drv.acquire_connection().await.expect("acquire conn");
    sqlx::query("BEGIN;")
        .execute(&mut *conn)
        .await
        .expect("begin");
    sqlx::query("INSERT INTO kv(key, value) VALUES (?, ?);")
        .bind("txk")
        .bind("tv1")
        .execute(&mut *conn)
        .await
        .expect("insert");
    // read back inside transaction
    let row = sqlx::query("SELECT value FROM kv WHERE key = ?;")
        .bind("txk")
        .fetch_one(&mut *conn)
        .await
        .expect("fetch");
    let v: String = row.get(0);
    assert_eq!(v, "tv1".to_string());
    sqlx::query("COMMIT;")
        .execute(&mut *conn)
        .await
        .expect("commit");

    // After commit, key should be visible
    let got = drv.get("txk").await.expect("get post commit");
    assert_eq!(got, Some("tv1".to_string()));

    // Rollback path: acquire fresh connection, begin, insert, then rollback
    let mut conn2 = drv.acquire_connection().await.expect("acquire conn2");
    sqlx::query("BEGIN;")
        .execute(&mut *conn2)
        .await
        .expect("begin2");
    sqlx::query("INSERT INTO kv(key, value) VALUES (?, ?);")
        .bind("txk2")
        .bind("tv2")
        .execute(&mut *conn2)
        .await
        .expect("insert2");
    sqlx::query("ROLLBACK;")
        .execute(&mut *conn2)
        .await
        .expect("rollback");

    let got2 = drv.get("txk2").await.expect("get post rollback");
    assert_eq!(got2, None);
}
