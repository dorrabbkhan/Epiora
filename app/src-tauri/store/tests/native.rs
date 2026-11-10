// Conformance of the native store itself (v0.3 plan Task 14 Step 1):
// the file is encrypted at rest, a wrong key is refused, blobs and
// i64s round-trip, and rollback really discards.

use epiora_store_native::{NativeStore, SqlValue, StoreErrorCode};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_db() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("epiora-native-{}-{n}.db", std::process::id()))
}

fn drop_store(store: NativeStore) {
    drop(store);
}

#[test]
fn file_has_no_plaintext_header() {
    let path = tmp_db();
    {
        let store = NativeStore::open(&path, &[7u8; 32]).expect("open with key");
        store.exec("CREATE TABLE t (v TEXT)", &[]).expect("create");
        store
            .exec(
                "INSERT INTO t VALUES (?)",
                &[SqlValue::Text("x".to_string())],
            )
            .expect("insert");
        drop_store(store);
    }
    let bytes = std::fs::read(&path).expect("read db file");
    assert!(
        bytes.len() >= 16,
        "db file too short: {} bytes",
        bytes.len()
    );
    assert_ne!(
        &bytes[..16],
        b"SQLite format 3\0",
        "database file carries the plaintext SQLite header"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn wrong_key_is_key_invalid() {
    let path = tmp_db();
    {
        let store = NativeStore::open(&path, &[1u8; 32]).expect("open with key");
        store
            .exec("CREATE TABLE t (v INTEGER)", &[])
            .expect("create");
        store
            .exec("INSERT INTO t VALUES (?)", &[SqlValue::Integer(42)])
            .expect("insert");
        drop_store(store);
    }
    let err = match NativeStore::open(&path, &[2u8; 32]) {
        Ok(_) => panic!("opening with the wrong key must fail"),
        Err(e) => e,
    };
    assert_eq!(err.code(), StoreErrorCode::KeyInvalid);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn blob_and_i64_roundtrip() {
    let path = tmp_db();
    let store = NativeStore::open(&path, &[3u8; 32]).expect("open with key");
    store
        .exec("CREATE TABLE t (b BLOB, n INTEGER)", &[])
        .expect("create");
    let blob = vec![0u8, 1, 2, 250, 255];
    let big: i64 = (1i64 << 53) + 3;
    store
        .exec(
            "INSERT INTO t VALUES (?, ?)",
            &[SqlValue::Blob(blob.clone()), SqlValue::Integer(big)],
        )
        .expect("insert");
    let rows = store.query("SELECT b, n FROM t", &[]).expect("select");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 2);
    assert_eq!(rows[0][0], ("b".to_string(), SqlValue::Blob(blob)));
    assert_eq!(rows[0][1], ("n".to_string(), SqlValue::Integer(big)));
    drop_store(store);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rollback_discards() {
    let path = tmp_db();
    let store = NativeStore::open(&path, &[4u8; 32]).expect("open with key");
    store
        .exec("CREATE TABLE t (v INTEGER)", &[])
        .expect("create");
    store.begin().expect("begin");
    store
        .exec("INSERT INTO t VALUES (?)", &[SqlValue::Integer(1)])
        .expect("insert in tx");
    store.rollback().expect("rollback");
    let rows = store
        .query("SELECT COUNT(*) AS n FROM t", &[])
        .expect("count");
    assert_eq!(rows[0][0].1, SqlValue::Integer(0));
    drop_store(store);
    let _ = std::fs::remove_file(&path);
}
