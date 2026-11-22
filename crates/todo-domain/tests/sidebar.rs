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
fn drop(state: &mut AppState, source: SidebarItem, target: SidebarDrop) {
    let action = state
        .sidebar_drop_action(source, target)
        .expect("valid navigation drop");
    state.apply(action, today()).unwrap();
    state.validate().unwrap();
}

#[test]
fn old_documents_keep_their_legacy_display_and_repair_only_order_metadata() {
    let mut state = AppState::new();
    let a = list(&mut state, "A");
    let g = group(&mut state, "Group");
    let b = list(&mut state, "B");
    let child = list(&mut state, "Child");
    state
        .apply(
            Action::MoveList {
                id: child,
                group_id: Some(g),
            },
            today(),
        )
        .unwrap();
    let mut json = serde_json::to_value(&state).unwrap();
    json.as_object_mut().unwrap().remove("sidebar_order");
    let legacy = AppState::from_json(&json.to_string()).unwrap();
    assert!(legacy.sidebar_order.is_empty());
    assert_eq!(legacy.sidebar_ids(), vec![a, b, g]);
    assert_eq!(legacy.lists, state.lists);
    json["sidebar_order"] = serde_json::json!([g, g, TASKS_LIST, child, 999, a]);
    let repaired = AppState::from_json(&json.to_string()).unwrap();
    assert_eq!(repaired.sidebar_ids(), vec![g, a, b]);
    assert_eq!(repaired.sidebar_order, vec![g, a, b]);
    assert_eq!(repaired.lists, state.lists);
    assert_eq!(repaired.groups, state.groups);
}

#[test]
fn lists_and_groups_interleave_and_new_roots_append_without_disturbing_children() {
    let mut state = AppState::new();
    let a = list(&mut state, "A");
    let g = group(&mut state, "Group");
    let b = list(&mut state, "B");
    assert_eq!(state.sidebar_ids(), vec![a, g, b]);
    drop(
        &mut state,
        SidebarItem::Group(g),
        SidebarDrop::Before(SidebarItem::List(a)),
    );
    assert_eq!(state.sidebar_ids(), vec![g, a, b]);
    drop(
        &mut state,
        SidebarItem::List(b),
        SidebarDrop::After(SidebarItem::Group(g)),
    );
    assert_eq!(state.sidebar_ids(), vec![g, b, a]);
    let new = list(&mut state, "New");
    assert_eq!(state.sidebar_ids(), vec![g, b, a, new]);
    assert_eq!(
        AppState::from_json(&state.to_json().unwrap()).unwrap(),
        state
    );
    assert!(
        state
            .sidebar_drop_action(SidebarItem::List(new), SidebarDrop::RootEnd)
            .is_none()
    );
}

#[test]
fn cross_group_edges_are_atomic_and_collapsed_destinations_are_revealed() {
    let mut state = AppState::new();
    let a = list(&mut state, "A");
    let b = list(&mut state, "B");
    let g = group(&mut state, "Group");
    drop(&mut state, SidebarItem::List(b), SidebarDrop::IntoGroup(g));
    state.apply(Action::ToggleGroup(g), today()).unwrap();
    drop(
        &mut state,
        SidebarItem::List(a),
        SidebarDrop::Before(SidebarItem::List(b)),
    );
    assert_eq!(state.sidebar_siblings(Some(g)), vec![a, b]);
    assert!(!state.group(g).unwrap().collapsed);
    assert_eq!(state.sidebar_ids(), vec![g]);
    drop(
        &mut state,
        SidebarItem::List(a),
        SidebarDrop::Before(SidebarItem::Group(g)),
    );
    assert_eq!(state.sidebar_ids(), vec![a, g]);
    assert_eq!(state.sidebar_siblings(Some(g)), vec![b]);
}

#[test]
fn ungroup_replaces_the_group_in_place_with_ordered_members_and_keeps_tasks() {
    let mut state = AppState::new();
    let first = list(&mut state, "Before");
    let g = group(&mut state, "Group");
    let last = list(&mut state, "After");
    let a = list(&mut state, "A");
    let b = list(&mut state, "B");
    drop(&mut state, SidebarItem::List(b), SidebarDrop::IntoGroup(g));
    drop(&mut state, SidebarItem::List(a), SidebarDrop::IntoGroup(g));
    state
        .apply(
            Action::AddTask {
                list_id: a,
                title: "Retained task".into(),
            },
            today(),
        )
        .unwrap();
    let tasks = state.tasks.clone();
    state.apply(Action::DeleteGroup(g), today()).unwrap();
    assert_eq!(state.sidebar_ids(), vec![first, b, a, last]);
    assert_eq!(state.sidebar_siblings(None), vec![first, b, a, last]);
    assert_eq!(state.tasks, tasks);
    assert!(state.lists.iter().all(|list| list.group_id.is_none()));
    state.validate().unwrap();
}

#[test]
fn protected_stale_self_and_nested_group_drops_have_no_action() {
    let mut state = AppState::new();
    let a = list(&mut state, "A");
    let g = group(&mut state, "Group");
    let other = group(&mut state, "Other");
    drop(&mut state, SidebarItem::List(a), SidebarDrop::IntoGroup(g));
    let original = state.clone();
    for (source, target) in [
        (SidebarItem::List(TASKS_LIST), SidebarDrop::IntoGroup(g)),
        (SidebarItem::List(999), SidebarDrop::RootEnd),
        (SidebarItem::Group(g), SidebarDrop::IntoGroup(other)),
        (
            SidebarItem::Group(other),
            SidebarDrop::Before(SidebarItem::List(a)),
        ),
        (
            SidebarItem::List(a),
            SidebarDrop::Before(SidebarItem::List(a)),
        ),
        (
            SidebarItem::List(a),
            SidebarDrop::Before(SidebarItem::List(TASKS_LIST)),
        ),
    ] {
        assert!(state.sidebar_drop_action(source, target).is_none());
    }
    assert_eq!(state, original);
}

#[test]
fn a_target_removed_after_hover_cannot_partially_move_the_source() {
    let mut state = AppState::new();
    let a = list(&mut state, "A");
    let g = group(&mut state, "Group");
    let plan = state
        .sidebar_drop_action(SidebarItem::List(a), SidebarDrop::IntoGroup(g))
        .unwrap();
    state.apply(Action::DeleteGroup(g), today()).unwrap();
    let original = state.clone();
    assert!(state.apply(plan, today()).is_err());
    assert_eq!(state, original);
}
