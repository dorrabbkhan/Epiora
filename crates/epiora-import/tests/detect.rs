//! Detection unit tests (v0.3 plan Task 3 Step 2; spec §6.1).

use epiora_import::{inspect, Container, DecimalSep, Encoding};

fn detection(bytes: &[u8], file_name: &str) -> epiora_import::Detection {
    inspect(bytes, file_name).expect("inspect").detection
}

fn utf16le(text: &str) -> Vec<u8> {
    let mut v = vec![0xFF, 0xFE];
    v.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    v
}

#[test]
fn delimiter_consistency_wins() {
    // Every line has exactly one ';'; commas appear inside fields, so
    // ',' is inconsistent while ';' is modal.
    let d = detection(b"A;B\nx,y;z\np,q;r\n", "d.csv");
    assert_eq!(d.delimiter, Some(';'));
}

#[test]
fn delimiter_tie_breaks_in_order() {
    // ',' and ';' both give a consistent 2 fields per line: ',' wins.
    let d = detection(b"a,b;c\n1,2;3\n", "d.csv");
    assert_eq!(d.delimiter, Some(','));
    // tab and '|' tie with ',' and ';' disqualified: tab wins.
    let d = detection(b"a\tb|c\n1\t2|3\n", "d.csv");
    assert_eq!(d.delimiter, Some('\t'));
}

#[test]
fn single_column_defaults_comma() {
    let d = detection(b"A\n1\n2\n", "d.csv");
    assert_eq!(d.delimiter, Some(','));
}

#[test]
fn utf8_invalid_falls_back_to_1252() {
    // 0xE9 is 'é' in Windows-1252 and invalid UTF-8.
    let d = detection(b"NAME\ncaf\xe9\n", "d.csv");
    assert_eq!(d.encoding, Some(Encoding::Windows1252));
    assert_eq!(d.container, Container::Text);
}

#[test]
fn bom_wins() {
    // A UTF-8 BOM forces UTF-8 even over other signal.
    let d = detection(b"\xef\xbb\xbfA,B\n1,2\n", "d.csv");
    assert_eq!(d.encoding, Some(Encoding::Utf8));
    // A UTF-16LE BOM forces UTF-16LE.
    let d = detection(&utf16le("A,B\n1,2\n"), "d.csv");
    assert_eq!(d.encoding, Some(Encoding::Utf16le));
    assert_eq!(d.delimiter, Some(','));
}

#[test]
fn decimal_comma_needs_semicolon() {
    // Comma decimals in a tab-delimited file stay Dot, so "1,5" is text.
    let d = detection(b"A\tB\n1,5\tx\n2,25\ty\n", "d.tsv");
    assert_eq!(d.delimiter, Some('\t'));
    assert_eq!(d.decimal, DecimalSep::Dot);
}

#[test]
fn extension_is_only_a_hint() {
    let d = detection(b"AGE,NAME\n20,alice\n,bob\n", "input.tsv");
    assert_eq!(d.delimiter, Some(','));
}
