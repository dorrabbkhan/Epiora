//! Planner tests — actions, `may_differ`, notes, RUNPGM inlining.

use epiora_lang::{parse_program, plan, plan_with, register, Action, LangError, ProgramSource};
use std::collections::BTreeMap;

fn actions(text: &str) -> Vec<Action> {
    let p = parse_program(text).expect("program parses");
    plan(&p, register())
}

/// A tiny `ProgramSource`: name → program text.
struct MapSource(BTreeMap<String, String>);
impl ProgramSource for MapSource {
    fn program(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}
fn mk_src(pairs: &[(&str, &str)]) -> MapSource {
    MapSource(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}

/// Every unsupported command is listed with its line and the
/// register's target version; `not_planned` commands have none
/// (spec §6.1: "reported (line, command, target version)").
#[test]
fn unsupported_listed_with_line_and_version() {
    let a = actions("READ 'f':t\nGRAPH ill * sex\nLOGISTIC ill = age\nEXECUTE \"x\"\n");
    match &a[1] {
        Action::Unsupported {
            line,
            command,
            target_version,
        } => {
            assert_eq!(*line, 2);
            assert_eq!(command, "GRAPH");
            assert_eq!(target_version.as_deref(), Some("1.4"));
        }
        other => panic!("{other:?}"),
    }
    match &a[2] {
        Action::Unsupported {
            line,
            command,
            target_version,
        } => {
            assert_eq!(*line, 3);
            assert_eq!(command, "LOGISTIC");
            assert_eq!(target_version.as_deref(), Some("1.2"));
        }
        other => panic!("{other:?}"),
    }
    match &a[3] {
        Action::Unsupported {
            command,
            target_version,
            ..
        } => {
            assert_eq!(command, "EXECUTE");
            assert_eq!(*target_version, None);
        }
        other => panic!("{other:?}"),
    }
}

/// `may_differ` is true for every procedure after an Unsupported
/// action the register marks `affects_data: true`; once set it
/// never resets — the plan names no reset rule.
#[test]
fn procedures_after_unsupported_data_command_may_differ() {
    let a = actions(
        "READ 'f':t\nDEFINE x\nFREQ y\nGRAPH z * w\nFREQ y\nREAD 'f':u\nGRAPH z * w\nFREQ y\n",
    );
    let differ = |i: usize| match &a[i] {
        Action::Procedure { may_differ, .. } => *may_differ,
        other => panic!("{other:?}"),
    };
    // `DEFINE x` (unsupported, affects_data) precedes the first FREQs.
    assert!(differ(2), "FREQ after unsupported DEFINE differs");
    assert!(differ(4), "still differs — the unsupported DEFINE ran");
    // The second READ does not reset the flag (the plan names no
    // reset): the earlier DEFINE still shadows this FREQ.
    assert!(differ(7), "READ does not reset may_differ");
}

/// The flag never resets (plan-literal): an `affects_data`
/// `Unsupported` command, then READ, then a procedure — the
/// procedure still `may_differ`. A procedure *before* the
/// unsupported command does not.
#[test]
fn may_differ_never_resets() {
    let a = actions("FREQ y\nDEFINE x\nREAD 'f':u\nFREQ y\n");
    let differ = |i: usize| match &a[i] {
        Action::Procedure { may_differ, .. } => *may_differ,
        other => panic!("{other:?}"),
    };
    assert!(!differ(0), "FREQ before the unsupported DEFINE");
    assert!(differ(3), "FREQ after DEFINE + READ still differs");
}

/// Output commands with no effect on numbers are recorded as notes
/// (TYPEOUT, HEADER, ROUTEOUT, CLOSEOUT, ... — spec §6.1).
#[test]
fn output_commands_become_notes() {
    let a = actions(
        "ROUTEOUT \"o.htm\" REPLACE\nHEADER\nTYPEOUT \"hi\"\nCLOSEOUT\nNEWPAGE\nPRINTOUT\nSYSINFO\nABOUT\nBEEP\nDISPLAY TABLES\n",
    );
    assert_eq!(a.len(), 10);
    for act in &a {
        match act {
            Action::Note { line, .. } => assert!(*line >= 1),
            other => panic!("expected a Note, got {other:?}"),
        }
    }
    if let Action::Note { text, .. } = &a[0] {
        assert!(text.contains("ROUTEOUT"), "{text}");
    }
}

/// RUNPGM inlines the saved program's actions, each keeping the
/// origin program's own line numbers, and a missing program is an
/// Unsupported action naming it.
#[test]
fn runpgm_inlines_saved_program_with_origin_lines() {
    let src = mk_src(&[
        ("inner", "READ 'f':u\nFREQ x\n"),
        ("caller", "READ 'f':t\nRUNPGM inner\nFREQ z\n"),
    ]);
    let p = parse_program("RUNPGM caller\nRUNPGM missing\n").unwrap();
    let a = plan_with(&p, register(), &src).expect("plans");
    // a[0]: the RUNPGM caller note; a[1..]: caller's own actions
    // (line 1 READ, line 2 RUNPGM note, inner's lines 1-2, line 3
    // FREQ); a[6]: the missing program's Unsupported.
    match &a[0] {
        Action::Note { line, text } => {
            assert_eq!(*line, 1);
            assert!(text.contains("caller"), "{text}");
        }
        other => panic!("{other:?}"),
    }
    // Caller's READ at its own line 1.
    assert!(matches!(&a[1], Action::Read { line: 1, .. }));
    // Caller's RUNPGM note at line 2, then inner's actions at
    // *inner's* lines 1 and 2.
    match &a[2] {
        Action::Note { line, text } => {
            assert_eq!(*line, 2);
            assert!(text.contains("inner"), "{text}");
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(&a[3], Action::Read { line: 1, .. }));
    match &a[4] {
        Action::Procedure { line, name, .. } => {
            assert_eq!(*line, 2, "inlined FREQ keeps inner's line");
            assert_eq!(name, "FREQ");
        }
        other => panic!("{other:?}"),
    }
    // Caller's FREQ z at its own line 3.
    match &a[5] {
        Action::Procedure { line, name, .. } => {
            assert_eq!(*line, 3);
            assert_eq!(name, "FREQ");
        }
        other => panic!("{other:?}"),
    }
    match &a[6] {
        Action::Unsupported {
            command,
            target_version,
            ..
        } => {
            assert!(command.contains("missing"), "{command}");
            assert_eq!(*target_version, None);
        }
        other => panic!("{other:?}"),
    }
}

/// A RUNPGM chain that names a program already running is
/// `LangError::RunPgmCycle` (code `pgm_runpgm_cycle`).
#[test]
fn runpgm_cycle_is_an_error() {
    let src = mk_src(&[("a", "RUNPGM b\n"), ("b", "RUNPGM a\n")]);
    let p = parse_program("RUNPGM a\n").unwrap();
    let e = plan_with(&p, register(), &src).expect_err("cycle");
    assert_eq!(e.code(), "pgm_runpgm_cycle");
    assert!(matches!(e, LangError::RunPgmCycle { .. }));
    // A self-run cycles too.
    let src = mk_src(&[("self", "RUNPGM self\n")]);
    let p = parse_program("RUNPGM self\n").unwrap();
    let e = plan_with(&p, register(), &src).expect_err("self-cycle");
    assert!(matches!(e, LangError::RunPgmCycle { .. }));
}

/// QUIT and EXIT end the plan at their line as a Note.
#[test]
fn quit_and_exit_end_the_plan() {
    let a = actions("READ 'f':t\nQUIT\nFREQ x\n");
    assert_eq!(a.len(), 2, "{a:?}");
    assert!(matches!(&a[1], Action::Note { line: 2, .. }));
    let a = actions("EXIT\nFREQ x\n");
    assert_eq!(a.len(), 1);
    assert!(matches!(&a[0], Action::Note { line: 1, .. }));
    // Statements after QUIT still *parse* — only planning stops.
    let p = parse_program("QUIT\nFREQ x\n").unwrap();
    assert_eq!(p.statements.len(), 2);
}
