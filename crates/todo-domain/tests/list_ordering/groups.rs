use super::{group, list, today};
use todo_domain::*;

fn groups(state: &AppState) -> Vec<Id> {
    let mut values: Vec<_> = state.groups.iter().map(|g| (g.position, g.id)).collect();
    values.sort_unstable();
    values.into_iter().map(|(_, id)| id).collect()
}

#[test]
fn group_reorder_is_deterministic_and_persists_without_changing_children() {
    let mut state = AppState::new();
    let a = group(&mut state, "A");
    let b = group(&mut state, "B");
    let c = group(&mut state, "C");
    let child = list(&mut state, "Child");
    state
        .apply(
            Action::MoveList {
                id: child,
                group_id: Some(c),
            },
            today(),
        )
        .unwrap();
    // Equal imported positions have the same ID tie-break in the model and the UI.
    for group in &mut state.groups {
        group.position = 10;
    }
    let children = state.lists.clone();
    let initial = state.clone();
    for (id, before) in [(a, Some(a)), (a, Some(b)), (c, None)] {
        assert!(
            !state
                .apply(Action::ReorderGroup { id, before }, today())
                .unwrap()
                .changed
        );
        assert_eq!(state, initial);
    }
    let changes = state
        .apply(
            Action::ReorderGroup {
                id: c,
                before: Some(a),
            },
            today(),
        )
        .unwrap();
    assert!(changes.lists_changed && changes.changed_tasks.is_empty());
    assert_eq!(groups(&state), vec![c, a, b]);
    state
        .apply(
            Action::ReorderGroup {
                id: c,
                before: None,
            },
            today(),
        )
        .unwrap();
    assert_eq!(groups(&state), vec![a, b, c]);
    assert_eq!(state.lists, children);
    assert_eq!(
        AppState::from_json(&state.to_json().unwrap()).unwrap(),
        state
    );
    let before = state.clone();
    for action in [
        Action::ReorderGroup {
            id: 9999,
            before: None,
        },
        Action::ReorderGroup {
            id: a,
            before: Some(child),
        },
    ] {
        assert!(state.apply(action, today()).is_err());
        assert_eq!(state, before);
    }
}
