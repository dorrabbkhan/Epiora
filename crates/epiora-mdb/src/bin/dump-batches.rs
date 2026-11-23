//! Writes EPTB batches to a directory — the native oracle files the
//! wasm parity tests compare against:
//!
//!   cargo run -p epiora-mdb --bin dump-batches -- \
//!     fixtures/legacy/samples/ei7/Sample/Sample.mdb \
//!     crates/epiora-mdb/tests/data/batches/sample
//!
//! `--table NAME` restricts the dump to one table; `--chunk ROWS`
//! emits one file per ROWS-row chunk (`{name}.{offset}.eptb`) instead
//! of a single whole-table `{name}.eptb` batch.

use std::path::Path;

/// The TS tests resolve files by `encodeURIComponent(name)`.
fn encode_name(table: &str) -> String {
    let mut fname = String::new();
    for b in table.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'!'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'-'
            | b'.'
            | b'_'
            | b'~' => fname.push(b as char),
            _ => fname.push_str(&format!("%{b:02X}")),
        }
    }
    fname
}

/// u32 row_count at EPTB header offset 6.
fn batch_rows(batch: &[u8]) -> u32 {
    u32::from_le_bytes(batch[6..10].try_into().unwrap())
}

fn main() {
    let mut positional = Vec::new();
    let mut table_filter: Option<String> = None;
    let mut chunk: Option<u32> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--table" => table_filter = Some(args.next().expect("usage: --table NAME")),
            "--chunk" => {
                chunk = Some(
                    args.next()
                        .expect("usage: --chunk ROWS")
                        .parse()
                        .expect("--chunk ROWS is a u32"),
                )
            }
            _ => positional.push(a),
        }
    }
    assert_eq!(
        positional.len(),
        2,
        "usage: dump-batches <mdb> <out_dir> [--table NAME] [--chunk ROWS]"
    );
    std::fs::create_dir_all(&positional[1]).unwrap();

    let mdb = epiora_mdb::Mdb::open(Path::new(&positional[0])).expect("open");
    for table in mdb.tables().expect("tables") {
        if table_filter.as_deref().is_some_and(|t| t != table) {
            continue;
        }
        let fname = encode_name(&table);
        match chunk {
            None => {
                let batch = mdb.batch(&table, 0, u32::MAX).expect("batch");
                let path = Path::new(&positional[1]).join(format!("{fname}.eptb"));
                std::fs::write(&path, &batch).unwrap();
                println!("{table}: {} bytes", batch.len());
            }
            Some(rows) => {
                let mut offset = 0u32;
                loop {
                    let batch = mdb.batch(&table, offset, rows).expect("batch");
                    let n = batch_rows(&batch);
                    if n == 0 {
                        break;
                    }
                    let path = Path::new(&positional[1]).join(format!("{fname}.{offset}.eptb"));
                    std::fs::write(&path, &batch).unwrap();
                    println!("{table} @{offset}: {} bytes", batch.len());
                    offset += n;
                }
            }
        }
    }
}
