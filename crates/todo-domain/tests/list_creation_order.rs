use todo_domain::*;

fn today() -> Date {
    "2026-11-22".parse().unwrap()
}

#[test]
fn creation_and_duplication_append_after_sparse_sidebar_positions() {
    let mut state = AppState::new();
    let first = state
        .apply(
            Action::AddList {
                name: "First".into(),
            },
            today(),
        )
        .unwrap()
        .created_list
        .unwrap();
    state
        .lists
        .iter_mut()
        .find(|list| list.id == first)
        .unwrap()
        .position = 1000;
    let second = state
        .apply(
            Action::AddList {
                name: "Second".into(),
            },
            today(),
        )
        .unwrap()
        .created_list
        .unwrap();
    let copy = state
        .apply(Action::DuplicateList(first), today())
        .unwrap()
        .created_list
        .unwrap();
    assert!(state.list(first).unwrap().position < state.list(second).unwrap().position);
    assert!(state.list(second).unwrap().position < state.list(copy).unwrap().position);
    let group = state
        .apply(
            Action::AddGroup {
                name: "Group".into(),
            },
            today(),
        )
        .unwrap()
        .created_group
        .unwrap();
    state.groups[0].position = 2000;
    let other = state
        .apply(
            Action::AddGroup {
                name: "Other".into(),
            },
            today(),
        )
        .unwrap()
        .created_group
        .unwrap();
    assert!(state.group(group).unwrap().position < state.group(other).unwrap().position);
    state.validate().unwrap();
}

#[test]
fn exhausted_sidebar_order_rejects_creation_before_allocating_an_identifier() {
    let mut state = AppState::new();
    let list = state
        .apply(
            Action::AddList {
                name: "Last".into(),
            },
            today(),
        )
        .unwrap()
        .created_list
        .unwrap();
    state
        .lists
        .iter_mut()
        .find(|item| item.id == list)
        .unwrap()
        .position = i64::MAX;
    state
        .apply(
            Action::AddGroup {
                name: "Last group".into(),
            },
            today(),
        )
        .unwrap();
    state.groups[0].position = i64::MAX;
    for action in [
        Action::AddList {
            name: "Overflow".into(),
        },
        Action::DuplicateList(list),
        Action::AddGroup {
            name: "Overflow".into(),
        },
    ] {
        let before = state.clone();
        assert!(state.apply(action, today()).is_err());
        assert_eq!(
            state, before,
            "a rejected creation must preserve the document and ID cursor"
        );
    }
}
