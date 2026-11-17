//! Epi Info command generation tests (v1.0 Task 11, spec §7.3/G4):
//! `epi_info_command` renders the Classic Analysis command a result
//! card shows — `FREQ A B [STRATAVAR=S]`, `TABLES E O
//! [STRATAVAR=S1 S2]`, `MEANS V [G]` — in Epi Info's spelling and
//! upper case. Names that are not plain identifiers are wrapped in
//! `[ ]`; options with no Epi Info equivalent become `* Epiora:`
//! comment lines.

use epiora_core::export::epi_info_command;
use serde_json::json;

#[test]
fn freq_command() {
    let got =
        epi_info_command("freq", &json!({"variables": ["AGE", "SEX"]})).expect("freq command");
    assert_eq!(got, "FREQ AGE SEX");
}

#[test]
fn freq_stratified_command() {
    let got = epi_info_command("freq", &json!({"variables": ["AGE"], "stratify_by": "SEX"}))
        .expect("stratified freq");
    assert_eq!(got, "FREQ AGE STRATAVAR=SEX");
}

#[test]
fn tables_stratified_command() {
    let got = epi_info_command(
        "tables",
        &json!({
            "exposure": {"name": "VANILLA", "levels": null},
            "outcome": {"name": "ILL", "levels": null},
            "stratify_by": ["SEX", "RACE"],
            "include_missing": false
        }),
    )
    .expect("tables command");
    assert_eq!(got, "TABLES VANILLA ILL STRATAVAR=SEX RACE");
}

#[test]
fn means_grouped_command() {
    let got = epi_info_command(
        "means",
        &json!({"variable": "AGE", "group_by": "SEX", "include_missing": false}),
    )
    .expect("means command");
    assert_eq!(got, "MEANS AGE SEX");
}

#[test]
fn means_ungrouped_command() {
    let got = epi_info_command(
        "means",
        &json!({"variable": "AGE", "group_by": null, "include_missing": false}),
    )
    .expect("ungrouped means");
    assert_eq!(got, "MEANS AGE");
}

#[test]
fn name_with_space_is_bracketed() {
    let got =
        epi_info_command("freq", &json!({"variables": ["My Var", "AGE"]})).expect("bracketed name");
    assert_eq!(got, "FREQ [My Var] AGE");
}

#[test]
fn name_with_non_identifier_char_is_bracketed() {
    let got = epi_info_command(
        "tables",
        &json!({
            "exposure": {"name": "expo-sure", "levels": null},
            "outcome": {"name": "2ND", "levels": null},
            "stratify_by": [],
            "include_missing": false
        }),
    )
    .expect("bracketed names");
    assert_eq!(got, "TABLES [expo-sure] [2ND]");
}

#[test]
fn include_missing_becomes_comment() {
    let got = epi_info_command(
        "freq",
        &json!({"variables": ["AGE"], "include_missing": true}),
    )
    .expect("include_missing comment");
    assert_eq!(got, "FREQ AGE\n* Epiora: include_missing");
}

#[test]
fn unknown_procedure_is_invalid_options() {
    match epi_info_command("regress", &json!({})) {
        Err(epiora_core::EngineError::InvalidOptions { .. }) => {}
        other => panic!("expected InvalidOptions, got {other:?}"),
    }
}
