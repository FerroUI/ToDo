#[path = "list_ordering/groups.rs"]
mod group_tests;

use todo_domain::*;

fn today() -> Date {
    "2026-11-22".parse().unwrap()
}

fn list(state: &mut AppState, name: &str) -> Id {
    state
        .apply(Action::AddList { name: name.into() }, today())
        .unwrap()
        .created_list
        .unwrap()
}

fn group(state: &mut AppState, name: &str) -> Id {
    state
        .apply(Action::AddGroup { name: name.into() }, today())
        .unwrap()
        .created_group
        .unwrap()
}

fn lists(state: &AppState, group_id: Option<Id>) -> Vec<Id> {
    let mut values: Vec<_> = state
        .lists
        .iter()
        .filter(|list| list.id != TASKS_LIST && list.group_id == group_id)
        .map(|list| (list.position, list.id))
        .collect();
    values.sort_unstable();
    values.into_iter().map(|(_, id)| id).collect()
}

#[test]
fn moving_a_list_appends_and_repeated_moves_are_noops() {
    let mut state = AppState::new();
    let a = list(&mut state, "Older source");
    let b = list(&mut state, "Destination child");
    let c = list(&mut state, "Root sibling");
    let destination = Some(group(&mut state, "Destination"));
    state
        .apply(
            Action::MoveList {
                id: b,
                group_id: destination,
            },
            today(),
        )
        .unwrap();
    state
        .apply(
            Action::MoveList {
                id: a,
                group_id: destination,
            },
            today(),
        )
        .unwrap();
    assert_eq!(lists(&state, destination), vec![b, a]);
    let before = state.clone();
    assert!(
        !state
            .apply(
                Action::MoveList {
                    id: a,
                    group_id: destination
                },
                today()
            )
            .unwrap()
            .changed
    );
    assert_eq!(state, before);
    state
        .apply(
            Action::MoveList {
                id: a,
                group_id: None,
            },
            today(),
        )
        .unwrap();
    assert_eq!(lists(&state, None), vec![c, a]);
    state.validate().unwrap();
}

#[test]
fn invalid_or_overflowing_moves_leave_the_entire_document_unchanged() {
    let mut state = AppState::new();
    let source = list(&mut state, "Source");
    let child = list(&mut state, "Destination child");
    let destination = Some(group(&mut state, "Destination"));
    state
        .apply(
            Action::MoveList {
                id: child,
                group_id: destination,
            },
            today(),
        )
        .unwrap();
    state
        .lists
        .iter_mut()
        .find(|l| l.id == child)
        .unwrap()
        .position = i64::MAX;
    let original = state.clone();
    for action in [
        Action::MoveList {
            id: source,
            group_id: destination,
        },
        Action::MoveList {
            id: source,
            group_id: Some(9999),
        },
        Action::MoveList {
            id: TASKS_LIST,
            group_id: destination,
        },
    ] {
        assert!(state.apply(action, today()).is_err());
        assert_eq!(state, original);
    }
}

#[test]
fn list_reorder_preserves_noop_positions_and_rejects_invalid_targets() {
    let mut state = AppState::new();
    let a = list(&mut state, "A");
    let b = list(&mut state, "B");
    let c = list(&mut state, "C");
    let original = state.clone();
    for (id, before) in [(a, Some(a)), (a, Some(b)), (c, None)] {
        assert!(
            !state
                .apply(Action::ReorderList { id, before }, today())
                .unwrap()
                .changed
        );
        assert_eq!(state, original);
    }
    state
        .apply(
            Action::ReorderList {
                id: c,
                before: Some(a),
            },
            today(),
        )
        .unwrap();
    assert_eq!(lists(&state, None), vec![c, a, b]);
    state
        .apply(
            Action::ReorderList {
                id: c,
                before: Some(b),
            },
            today(),
        )
        .unwrap();
    assert_eq!(lists(&state, None), vec![a, c, b]);
    state
        .apply(
            Action::ReorderList {
                id: c,
                before: None,
            },
            today(),
        )
        .unwrap();
    assert_eq!(lists(&state, None), vec![a, b, c]);
    let destination = Some(group(&mut state, "Other"));
    state
        .apply(
            Action::MoveList {
                id: c,
                group_id: destination,
            },
            today(),
        )
        .unwrap();
    let before = state.clone();
    for target in [Some(c), Some(TASKS_LIST), Some(9999)] {
        assert!(
            state
                .apply(
                    Action::ReorderList {
                        id: a,
                        before: target
                    },
                    today()
                )
                .is_err()
        );
        assert_eq!(state, before);
    }
}

#[test]
fn a_cross_group_drop_rolls_back_if_its_final_target_is_invalid() {
    let mut state = AppState::new();
    let source = list(&mut state, "Source");
    let sibling = list(&mut state, "Sibling");
    let destination = Some(group(&mut state, "Destination"));
    let before = state.clone();
    let result = state.apply(
        Action::Batch(vec![
            Action::MoveList {
                id: source,
                group_id: destination,
            },
            Action::ReorderList {
                id: source,
                before: Some(sibling),
            },
        ]),
        today(),
    );
    assert!(result.is_err());
    assert_eq!(state, before);
}
