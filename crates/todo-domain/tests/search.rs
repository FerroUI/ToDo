use todo_domain::*;

fn today() -> Date {
    "2026-11-23".parse().unwrap()
}

fn reference_match(task: &Task, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return false;
    }
    // The existing user-visible contract: ANDed substrings of the combined fields,
    // with one synthetic hashtag before every literal category name.
    let fields = [
        task.title.clone(),
        task.notes.clone(),
        task.steps
            .iter()
            .map(|step| step.title.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        task.tags
            .iter()
            .map(|tag| format!("#{tag}"))
            .collect::<Vec<_>>()
            .join(" "),
    ];
    let text = fields.join(" ").to_lowercase();
    query.split_whitespace().all(|word| text.contains(word))
}

#[test]
fn query_projection_preserves_unicode_cross_field_and_literal_category_matching() {
    let mut task = Task::new(2, TASKS_LIST, "Plan THE ÜBER release".into(), today());
    task.notes = "Café notes with İSTANBUL and ΟΣ".into();
    task.steps = vec![
        Step {
            id: 3,
            title: "Review API".into(),
            completed: false,
        },
        Step {
            id: 4,
            title: "Żółw Unicode".into(),
            completed: true,
        },
    ];
    task.tags = ["Work Projects", "#Client", "R&D, Inc.", "Σ"]
        .map(str::to_owned)
        .to_vec();
    let mut partial = Task::new(5, TASKS_LIST, "ÜBER only".into(), today());
    partial.completed = true;
    let plain = Task::new(6, TASKS_LIST, "Unrelated".into(), today());
    let mut state = AppState::new();
    state.tasks = vec![task, partial, plain];
    state.next_id = 7;
    for query in [
        "",
        "  \t\n",
        "über",
        "ÜBER API #WORK",
        "#Work Café",
        "#Work Projects",
        "#projects",
        "#workprojects",
        "##CLIENT",
        "#Client",
        "#",
        "R&D,",
        "Inc.",
        "İstanbul",
        "i\u{307}stanbul",
        "ΟΣ",
        "ος",
        "οσ",
        "Σ",
        "#Σ",
        "żÓŁW API",
        "Plan release plan",
        "release missing",
        "\u{2003}api\u{a0}café\n",
    ] {
        let expected = state
            .tasks
            .iter()
            .filter(|task| reference_match(task, query))
            .map(|task| task.id)
            .collect::<Vec<_>>();
        let view = View::Search(query.into());
        let actual = state
            .visible_tasks(&view, SortOrder::Manual, true, today())
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "query {query:?}");
        assert_eq!(
            state.count(&view, today()).total(),
            expected.len(),
            "query {query:?}"
        );
        for task in &state.tasks {
            assert_eq!(
                view.matches(task, today()),
                reference_match(task, query),
                "task {}, query {query:?}",
                task.id
            );
        }
    }
}

#[test]
fn repeated_projections_reset_matched_terms_between_tasks_and_retain_completion_filters() {
    let mut state = AppState::new();
    for (id, title, notes, completed) in [
        (2, "First needle", "second", false),
        (3, "Only first", "", false),
        (4, "Only second", "", false),
        (5, "First", "Second needle", true),
    ] {
        let mut task = Task::new(id, TASKS_LIST, title.into(), today());
        task.notes = notes.into();
        task.completed = completed;
        state.tasks.push(task);
    }
    let view = View::Search("FIRST second needle".into());
    assert_eq!(
        state.count(&view, today()),
        TaskCount {
            active: 1,
            completed: 1
        }
    );
    assert_eq!(
        state
            .visible_tasks(&view, SortOrder::Manual, false, today())
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>(),
        [2]
    );
    assert_eq!(
        state
            .visible_tasks(&view, SortOrder::Manual, true, today())
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>(),
        [2, 5]
    );
}
