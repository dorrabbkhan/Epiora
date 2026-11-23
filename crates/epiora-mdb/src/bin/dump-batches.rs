//! Writes one whole-table EPTB batch per user table to a directory —
//! the native oracle files the wasm parity test compares against:
//!
//!   cargo run -p epiora-mdb --bin dump-batches -- \
//!     fixtures/legacy/samples/ei7/Sample/Sample.mdb \
//!     crates/epiora-mdb/tests/data/batches/sample

use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let mdb_path = args.next().expect("usage: dump-batches <mdb> <out_dir>");
    let out_dir = args.next().expect("usage: dump-batches <mdb> <out_dir>");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mdb = epiora_mdb::Mdb::open(Path::new(&mdb_path)).expect("open");
    for table in mdb.tables().expect("tables") {
        let batch = mdb.batch(&table, 0, u32::MAX).expect("batch");
        // The TS test resolves files by encodeURIComponent(name).
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
        let path = Path::new(&out_dir).join(format!("{fname}.eptb"));
        std::fs::write(&path, &batch).unwrap();
        println!("{table}: {} bytes", batch.len());
    }
}
