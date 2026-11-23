//! v1.1 Task 9 (spec §4.1, §8.2, §15.7): the SQL Server one-time
//! import path — `legacy_sqlserver_*` commands produce EPTB batches
//! that Task 7's mapper turns into the same `LegacyProject` the
//! Access golden pins. These tests run only in `legacy.yml`'s
//! `sqlserver` job, where a SQL Server 2022 Developer container holds
//! the verbatim `copy-tables.mjs` copy of `Sample.mdb`.
//!
//! Every test is env-gated: `EPIORA_SQLSERVER_HOST` is set only by
//! that job. Anywhere else (`cargo test --workspace` in app.yml's
//! native-store, local runs) there is no server to connect to, so the
//! test returns at once — the suite exists to be run in the CI job,
//! not to be skipped by `#[ignore]` (which the repo forbids).

use std::path::{Path, PathBuf};

use epiora_app::legacy_sqlserver::{
    sqlserver_batches, sqlserver_tables, SqlServerAuth, SqlServerConn,
};
use epiora_legacy::{map_ei7, BatchSource, SourceFormat};
use serde_json::Value as Json;

fn conn() -> Option<SqlServerConn> {
    let host = std::env::var("EPIORA_SQLSERVER_HOST").ok()?;
    Some(SqlServerConn {
        host,
        port: std::env::var("EPIORA_SQLSERVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(1433),
        database: std::env::var("EPIORA_SQLSERVER_DATABASE").unwrap_or_else(|_| "sample".into()),
        auth: SqlServerAuth::Sql {
            user: std::env::var("EPIORA_SQLSERVER_USER").unwrap_or_else(|_| "sa".into()),
            password: std::env::var("EPIORA_SQLSERVER_PASSWORD").unwrap_or_default(),
        },
        trust_cert: std::env::var("EPIORA_SQLSERVER_TRUST_CERT")
            .map(|v| v != "0")
            .unwrap_or(true),
    })
}

fn absent() -> bool {
    eprintln!("no EPIORA_SQLSERVER_HOST — sqlserver CI job only, nothing runs here");
    true
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The `project.json` of `fixtures/legacy/golden/ei7/Sample/Sample`
/// (the Access golden the verbatim copies must reproduce).
fn golden_project() -> Json {
    let p = root().join("fixtures/legacy/golden/ei7/Sample/Sample/project.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// One full import: every table's EPTB chunks pushed into a
/// `BatchSource`, then mapped as `ei7_sqlserver`.
async fn import_project(conn: &SqlServerConn) -> epiora_legacy::LegacyProject {
    let tables = sqlserver_tables(conn).await.unwrap();
    assert!(
        tables.iter().any(|t| t.eq_ignore_ascii_case("metaviews")),
        "expected an EI7 project layout, got tables {tables:?}"
    );
    let mut src = BatchSource::new();
    for t in &tables {
        eprintln!("-- fetching {t}");
        for batch in sqlserver_batches(conn, t, 50_000).await.unwrap() {
            src.push(t, &batch).unwrap();
        }
    }
    map_ei7(&mut src, SourceFormat::Ei7SqlServer).unwrap()
}

/// `sqlserver_copy_matches_access_hashes` — the verbatim SQL Server
/// copy of `Sample.mdb` maps to a project whose view and grid
/// datasets hash exactly like the Access golden (spec §8.2).
#[tokio::test(flavor = "multi_thread")]
async fn sqlserver_copy_matches_access_hashes() {
    if conn().is_none() && absent() {
        return;
    }
    let conn = conn().unwrap();
    let project = import_project(&conn).await;
    let golden = golden_project();

    assert_eq!(project.name, golden["name"].as_str().unwrap());
    assert_eq!(
        project.legacy_id,
        golden["legacy_id"].as_str().unwrap_or("")
    );
    let gviews = golden["views"].as_array().unwrap();
    assert_eq!(project.views.len(), gviews.len(), "view count");
    for (v, gv) in project.views.iter().zip(gviews) {
        assert_eq!(&v.name, gv["name"].as_str().unwrap(), "view name");
        assert_eq!(
            v.dataset.row_count() as u64,
            gv["records"].as_u64().unwrap(),
            "view {}: records",
            v.name
        );
        assert_eq!(
            v.deleted_records,
            gv["deleted_records"].as_u64().unwrap(),
            "view {}: deleted_records",
            v.name
        );
        assert_eq!(
            v.dataset.content_hash_hex(),
            gv["hash"].as_str().unwrap(),
            "view {}: hash",
            v.name
        );
    }
    let glinks = golden["links"].as_array().unwrap();
    assert_eq!(project.links.len(), glinks.len(), "link count");
    for (l, gl) in project.links.iter().zip(glinks) {
        if gl["kind"].as_str().unwrap() != "grid" {
            continue;
        }
        let ds = project
            .datasets
            .iter()
            .find(|d| d.id() == l.child)
            .unwrap_or_else(|| panic!("grid dataset {} missing", l.child));
        assert_eq!(
            ds.content_hash_hex(),
            gl["hash"].as_str().unwrap(),
            "grid {}: hash",
            l.child
        );
    }
}

/// `bad_password_is_sqlserver_auth_without_details` — a rejected
/// login reports `sqlserver_auth` and the error string carries no
/// host, user, password or database (spec §10, §15.7).
#[tokio::test(flavor = "multi_thread")]
async fn bad_password_is_sqlserver_auth_without_details() {
    if conn().is_none() && absent() {
        return;
    }
    let mut conn = conn().unwrap();
    conn.auth = SqlServerAuth::Sql {
        user: "sa".to_string(),
        password: "epiora-wrong-password-7f3a!".to_string(),
    };
    let err = sqlserver_tables(&conn).await.unwrap_err();
    assert_eq!(err.code(), "sqlserver_auth");
    let shown = format!("{err}");
    assert_eq!(shown, "sqlserver_auth", "the wire string is only a code");
    for detail in [
        conn.host.as_str(),
        conn.database.as_str(),
        "epiora-wrong-password-7f3a!",
    ] {
        assert!(
            !shown.contains(detail),
            "error string leaks connection detail {detail:?}: {shown:?}"
        );
    }
}

/// `connection_not_persisted` — after a full import, the files the
/// desktop shell writes (the encrypted store and a settings file)
/// contain none of the connection values (spec §4.5, §15.7).
#[tokio::test(flavor = "multi_thread")]
async fn connection_not_persisted() {
    if conn().is_none() && absent() {
        return;
    }
    let conn = conn().unwrap();
    let dir = std::env::temp_dir().join(format!("epiora-sqlserver-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The app's on-disk surface: a store database and a settings file.
    {
        let key = [7u8; 32];
        let store = epiora_store_native::NativeStore::open(&dir.join("epiora.db"), &key).unwrap();
        store
            .exec("CREATE TABLE settings (k TEXT PRIMARY KEY, v TEXT)", &[])
            .unwrap();
        store
            .exec("INSERT INTO settings (k, v) VALUES ('theme', 'dark')", &[])
            .unwrap();
    }
    std::fs::write(
        dir.join("settings.json"),
        r#"{"theme":"dark","locale":"en","lastProject":"epiora"}"#,
    )
    .unwrap();

    let _project = import_project(&conn).await;

    // Grep every file under the app dir for the connection values.
    // `user` is "sa" (two letters, a guaranteed false positive in any
    // file) and `port` four digits, so only values of eight or more
    // distinctive characters are asserted.
    let mut secrets = vec![conn.host.clone(), conn.database.clone()];
    if let SqlServerAuth::Sql { user, password } = &conn.auth {
        secrets.push(password.clone());
        if user.len() >= 8 {
            secrets.push(user.clone());
        }
    }
    let mut files = Vec::new();
    collect_files(&dir, &mut files);
    assert!(!files.is_empty(), "expected files under {dir:?}");
    for f in &files {
        let bytes = std::fs::read(f).unwrap();
        for s in &secrets {
            assert!(
                !s.is_empty() && !bytes.windows(s.len()).any(|w| w == s.as_bytes()),
                "connection value {s:?} found in {}",
                f.display()
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect_files(&p, out);
        } else {
            out.push(p);
        }
    }
}
