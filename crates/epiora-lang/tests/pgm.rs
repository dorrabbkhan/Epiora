//! Task 12's `.pgm` tests — the program parser, planner, command
//! register and `print_command` (v1.1 spec §6.1).

mod common;
#[path = "pgm/plan.rs"]
mod plan;
#[path = "pgm/print.rs"]
mod print;

use epiora_lang::{decode_pgm_text, parse_program, LangError, Statement};
use std::path::{Path, PathBuf};

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn collect_pgm(dir: &Path, out: &mut Vec<PathBuf>) {
    if dir.is_dir() {
        for e in std::fs::read_dir(dir).expect("read dir") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                collect_pgm(&p, out);
            } else if p.extension().is_some_and(|x| x == "pgm") {
                out.push(p);
            }
        }
    }
}

/// Every valid Epi Info program parses — parsing never depends on
/// support (spec §6.1). Runs over the corpus `.pgm` fixtures.
#[test]
fn every_fixture_program_parses() {
    let mut files = vec![];
    collect_pgm(&root().join("fixtures/ei7"), &mut files);
    files.sort();
    assert!(files.len() >= 8, "fixture .pgm files: {files:?}");
    for f in &files {
        let text = std::fs::read_to_string(f).expect("read");
        let p = parse_program(&text).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        assert!(
            !p.statements.is_empty(),
            "{} produced no statements",
            f.display()
        );
    }
}

/// Bytes that are not UTF-8 decode as Windows-1252; CRLF and bare CR
/// line endings and mixed-case keywords all parse (Review Focus 4).
#[test]
fn crlf_cp1252_mixed_case_program_parses() {
    // \xE9 is 'é' in cp1252 (invalid UTF-8, so decoding is exercised).
    let bytes: &[u8] = b"read 'x.mdb':t\r\nFrEq iLl WEIGHTVAR=w\rselect ill = 1\r\nTYPEOUT \"caf\xE9\"\r\nquit\r\n";
    let text = decode_pgm_text(bytes);
    let p = parse_program(&text).expect("cp1252 program parses");
    assert_eq!(p.statements.len(), 5);
    match &p.statements[0].node {
        Statement::Read(target) => assert_eq!(target.name, "x.mdb:t"),
        other => panic!("{other:?}"),
    }
    match &p.statements[1].node {
        Statement::Procedure { name, options, .. } => {
            assert_eq!(name, "FREQ");
            assert_eq!(options["vars"][0], "iLl");
            assert_eq!(options["opts"]["WEIGHTVAR"], "w");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(p.statements[1].line, 2);
    match &p.statements[3].node {
        Statement::Output(t) => assert!(t.contains("café"), "{t}"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(p.statements[4].node, Statement::Quit));
}

/// A real program in Epi Info 3.5-era syntax from the Task 2 sample
/// projects: the saved programs in the sample .mdb's `Programs`
/// table (read-only). EI3 forms we cannot represent (untyped DEFINE,
/// `LIST * UPDATE`, FILESPEC, MAP, DIALOG) parse as Unsupported
/// statements or fail with `pgm_syntax` at a real line — never a
/// panic (Review Focus 4).
#[test]
fn ei3_style_program_parses_or_reports() {
    let (names, texts) = ei3_programs();
    assert!(!names.is_empty(), "sample has saved programs");
    let mut parsed = 0usize;
    for (name, text) in names.iter().zip(texts.iter()) {
        match parse_program(text) {
            Ok(p) => {
                parsed += 1;
                assert!(!p.statements.is_empty(), "{name}");
            }
            Err(e) => {
                assert_eq!(e.code(), "pgm_syntax", "{name}: {e}");
                let LangError::Syntax { line, .. } = e else {
                    panic!("{name}: {e:?}");
                };
                assert!(
                    line as usize <= text.lines().count(),
                    "{name}: error at line {line} beyond the program"
                );
            }
        }
    }
    // At least one whole EI3 program must parse end to end.
    assert!(parsed >= 1, "no program parsed");
}

/// Errors carry the 1-based line and column of the offending token.
#[test]
fn syntax_error_reports_line_and_column() {
    let err = parse_program("READ t\nFREQ x\n  BONKERS y\n").expect_err("must fail");
    match err {
        LangError::Syntax { line, column, .. } => {
            assert_eq!(line, 3);
            assert_eq!(column, 3);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(err.code(), "pgm_syntax");
    // An unterminated string reports the opening quote's line.
    match parse_program("TYPEOUT \"abc").expect_err("unterminated") {
        LangError::Syntax { line: 1, .. } => {}
        other => panic!("{other:?}"),
    }
    // A dangling IF reports the missing END.
    match parse_program("IF x = 1 THEN\nASSIGN y = 2\n").expect_err("no END") {
        LangError::Syntax { line: 2, .. } => {}
        other => panic!("{other:?}"),
    }
}

/// `(name, text)` pairs from `Asthma Final.mdb`'s Programs table.
fn ei3_programs() -> (Vec<String>, Vec<String>) {
    use epiora_legacy::batch::{decode_batch, RawData};
    let path = root().join("fixtures/legacy/samples/ei3/Asthma Final.mdb");
    let mdb = epiora_mdb::Mdb::open(&path).expect("open sample");
    let bytes = mdb.batch("Programs", 0, 100).expect("batch");
    let t = decode_batch(&bytes).expect("decode");
    let col = |name: &str| {
        let i = t.columns.iter().position(|c| c.name == name).expect(name);
        match &t.columns[i].data {
            RawData::Text(v) => v.clone(),
            other => panic!("{name}: {other:?}"),
        }
    };
    let names: Vec<String> = col("Name").into_iter().flatten().collect();
    let texts: Vec<String> = col("Content").into_iter().flatten().collect();
    (names, texts)
}
