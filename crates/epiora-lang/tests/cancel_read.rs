//! `CANCEL READ` (v1.1 plan Task 13b): the register-supported command
//! plans as a `Manage` action (no longer `Unsupported`), prints and
//! re-parses as `CANCEL READ`, and replaying it closes the working
//! view — the session returns to its source dataset with an empty
//! recipe, so the command is never stored as a step.

mod common;
use common::ds::*;
use epiora_lang::{
    parse_program, plan, print_command, register, replay, Action, Command, Statement,
};

#[test]
fn cancel_read_plans_as_manage() {
    let program = parse_program("CANCEL READ").expect("CANCEL READ parses");
    assert_eq!(program.statements.len(), 1);
    let actions = plan(&program, register());
    assert_eq!(actions.len(), 1);
    assert!(
        matches!(
            &actions[0],
            Action::Manage {
                line: 1,
                command: Command::CancelRead,
            }
        ),
        "expected Action::Manage{{ CancelRead }}, got {:?}",
        actions[0]
    );
}

#[test]
fn cancel_read_prints_round_trip() {
    let text = print_command(&Command::CancelRead);
    assert_eq!(text, "CANCEL READ");
    let program = parse_program(&text).expect("printed form re-parses");
    assert!(
        matches!(
            &program.statements[0].node,
            Statement::Manage(Command::CancelRead)
        ),
        "expected Statement::Manage(CancelRead), got {:?}",
        program.statements[0].node
    );
}

#[test]
fn cancel_read_discards_recipe_and_view() {
    let src = ds("T", vec![("k", texts(&["1", "2"])), ("v", ints(&[10, 20]))]);
    // A hand-written recipe CAN carry the step (the session stores
    // none): replay resets the view to the source — the SELECT's row
    // filter is discarded.
    let view = replay(
        &src,
        &Links::default(),
        &recipe(vec![Command::Select(expr("v > 15")), Command::CancelRead]),
    )
    .expect("replay");
    assert_eq!(view.materialize().row_count(), 2);
    assert_eq!(
        view.materialize().content_hash_hex(),
        src.content_hash_hex(),
    );
}
