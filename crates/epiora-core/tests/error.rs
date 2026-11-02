use epiora_core::EngineError;

fn all_variants() -> Vec<EngineError> {
    vec![
        EngineError::UnknownVariable {
            name: "VANILLA".into(),
        },
        EngineError::AmbiguousLevels {
            variable: "T2".into(),
        },
        EngineError::EmptyDataset,
        EngineError::InvalidColumn {
            column: "AGE".into(),
            reason: "empty name".into(),
        },
        EngineError::TypeMismatch {
            variable: "ILL".into(),
            expected: "boolean".into(),
            found: "text".into(),
        },
        EngineError::InvalidOptions {
            reason: "no variables".into(),
        },
        EngineError::CorpusFormat {
            reason: "bad leaf".into(),
        },
    ]
}

#[test]
fn every_variant_has_its_code() {
    let expected = [
        "unknown_variable",
        "ambiguous_levels",
        "empty_dataset",
        "invalid_column",
        "type_mismatch",
        "invalid_options",
        "corpus_format",
    ];
    let variants = all_variants();
    assert_eq!(variants.len(), expected.len());
    for (variant, code) in variants.iter().zip(expected) {
        assert_eq!(variant.code(), code);
    }
}

#[test]
fn codes_are_unique() {
    let variants = all_variants();
    let mut codes: Vec<&'static str> = variants.iter().map(EngineError::code).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), variants.len());
}
