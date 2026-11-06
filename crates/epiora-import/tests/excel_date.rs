//! Excel serial-date conversion (v0.3 spec §6.2, plan Task 4):
//! `serial_to_days` maps the 1900 and 1904 systems to days since
//! 1970-01-01 — serial 60 (Excel's phantom 1900-02-29) and any serial
//! with a non-zero time part are `None`.

use epiora_import::serial_to_days;

#[test]
fn serial_1_is_1900_01_01() {
    assert_eq!(serial_to_days(1.0, false), Some(-25567));
}

#[test]
fn serial_59_is_1900_02_28() {
    assert_eq!(serial_to_days(59.0, false), Some(-25509));
}

#[test]
fn serial_60_is_none() {
    assert_eq!(serial_to_days(60.0, false), None);
}

#[test]
fn serial_61_is_1900_03_01() {
    assert_eq!(serial_to_days(61.0, false), Some(-25508));
}

#[test]
fn serial_45658_is_2025_01_01() {
    assert_eq!(serial_to_days(45658.0, false), Some(20089));
}

#[test]
fn system_1904_serial_0_is_1904_01_01() {
    assert_eq!(serial_to_days(0.0, true), Some(-24107));
}

#[test]
fn time_part_is_none() {
    assert_eq!(serial_to_days(45658.5, false), None);
    assert_eq!(serial_to_days(0.25, true), None);
}
