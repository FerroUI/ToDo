use todo_domain::*;
fn date(value: &str) -> Date {
    value.parse().unwrap()
}
fn dates(rule: &Recurrence, count: u32) -> Vec<Date> {
    (0..count)
        .filter_map(|index| rule.occurrence(index, date("2026-11-23")))
        .collect()
}

#[test]
fn weekly_sets_keep_the_anchor_week_and_skip_missed_cycles() {
    let rule = Recurrence {
        anchor: Some(date("2026-11-25")),
        days_of_week: vec![RepeatDay::Monday, RepeatDay::Friday],
        week_start: RepeatDay::Monday,
        ..Recurrence::new(RepeatUnit::Week, 2)
    };
    assert_eq!(
        dates(&rule, 5),
        [
            "2026-11-27",
            "2026-12-07",
            "2026-12-11",
            "2026-12-21",
            "2026-12-25"
        ]
        .map(date)
    );
    assert_eq!(
        rule.next_after(date("2026-11-27"), date("2026-12-12")),
        Some(date("2026-12-21"))
    );
    assert_eq!(
        rule.next_after(date("2026-11-01"), date("2026-11-01")),
        Some(date("2026-11-27"))
    );
    let sunday = Recurrence {
        anchor: Some(date("2026-11-28")),
        days_of_week: vec![RepeatDay::Sunday],
        ..Recurrence::new(RepeatUnit::Week, 2)
    };
    let monday = Recurrence {
        week_start: RepeatDay::Monday,
        ..sunday.clone()
    };
    assert_eq!(
        sunday.first_occurrence(date("2026-11-28")),
        Some(date("2026-12-06"))
    );
    assert_eq!(
        monday.first_occurrence(date("2026-11-28")),
        Some(date("2026-11-29"))
    );
}

#[test]
fn monthly_selectors_preserve_phase_and_choose_the_first_matching_relative_date() {
    let absolute = Recurrence {
        anchor: Some(date("2026-01-20")),
        day_of_month: Some(15),
        ..Recurrence::new(RepeatUnit::Month, 2)
    };
    assert_eq!(
        dates(&absolute, 3),
        ["2026-03-15", "2026-05-15", "2026-07-15"].map(date)
    );
    let relative = Recurrence {
        anchor: Some(date("2026-11-01")),
        days_of_week: vec![RepeatDay::Thursday, RepeatDay::Friday],
        ordinal: Some(WeekIndex::Second),
        ..Recurrence::monthly()
    };
    assert_eq!(
        dates(&relative, 3),
        ["2026-11-12", "2026-12-10", "2027-01-08"].map(date)
    );
    let last = Recurrence {
        days_of_week: vec![RepeatDay::Monday],
        ordinal: Some(WeekIndex::Last),
        ..relative
    };
    assert_eq!(
        dates(&last, 3),
        ["2026-11-30", "2026-12-28", "2027-01-25"].map(date)
    );
}

#[test]
fn yearly_month_selectors_leap_days_and_relative_dates_are_anchored() {
    let relative = Recurrence {
        anchor: Some(date("2026-01-01")),
        month: Some(11),
        days_of_week: vec![RepeatDay::Thursday],
        ordinal: Some(WeekIndex::Fourth),
        ..Recurrence::yearly()
    };
    assert_eq!(
        dates(&relative, 3),
        ["2026-11-26", "2027-11-25", "2028-11-23"].map(date)
    );
    let leap = Recurrence {
        anchor: Some(date("2024-01-01")),
        month: Some(2),
        day_of_month: Some(29),
        ..Recurrence::yearly()
    };
    assert_eq!(
        dates(&leap, 5),
        [
            "2024-02-29",
            "2025-02-28",
            "2026-02-28",
            "2027-02-28",
            "2028-02-29"
        ]
        .map(date)
    );
}

#[test]
fn finite_ranges_count_scheduled_occurrences_and_end_dates_are_inclusive() {
    let daily = Recurrence {
        anchor: Some(date("2026-11-23")),
        end: RepeatEnd::After(3),
        ..Recurrence::daily()
    };
    assert_eq!(
        dates(&daily, 5),
        ["2026-11-23", "2026-11-24", "2026-11-25"].map(date)
    );
    assert_eq!(
        daily.last_occurrence(date("2026-11-23")),
        Some(date("2026-11-25"))
    );
    assert_eq!(
        daily.next_after(date("2026-11-23"), date("2026-11-25")),
        None
    );
    let weekly = Recurrence {
        anchor: Some(date("2026-11-23")),
        days_of_week: vec![RepeatDay::Monday, RepeatDay::Wednesday, RepeatDay::Friday],
        end: RepeatEnd::On(date("2026-11-27")),
        ..Recurrence::weekly()
    };
    assert_eq!(
        dates(&weekly, 6),
        ["2026-11-23", "2026-11-25", "2026-11-27"].map(date)
    );
    assert_eq!(
        weekly.last_occurrence(date("2026-11-23")),
        Some(date("2026-11-27"))
    );
    assert_eq!(
        weekly.next_after(date("2026-11-27"), date("2026-11-27")),
        None
    );
}

#[test]
fn date_boundaries_and_huge_skips_are_bounded_without_overflow() {
    for unit in [
        RepeatUnit::Day,
        RepeatUnit::Weekday,
        RepeatUnit::Week,
        RepeatUnit::Month,
        RepeatUnit::Year,
    ] {
        let mut rule = Recurrence::new(unit, u16::MAX);
        rule.anchor = Some(Date::MIN);
        assert!(rule.next_after(Date::MIN, Date::MAX).is_none());
        assert!(rule.occurrence(u32::MAX, Date::MIN).is_none());
        rule.anchor = Some(Date::MAX);
        assert_eq!(rule.first_occurrence(Date::MAX), Some(Date::MAX));
        assert!(rule.next_after(Date::MAX, Date::MAX).is_none());
    }
}

#[test]
fn legacy_json_retains_its_shape_and_invalid_range_or_selectors_are_rejected() {
    let legacy = r#"{"unit":"Month","every":1,"anchor":"2027-01-31"}"#;
    let mut rule: Recurrence = serde_json::from_str(legacy).unwrap();
    assert_eq!(serde_json::to_string(&rule).unwrap(), legacy);
    rule.ordinal = Some(WeekIndex::First);
    assert!(rule.validate().is_err());
    rule.ordinal = None;
    rule.end = RepeatEnd::After(0);
    assert!(rule.validate().is_err());
    rule.end = RepeatEnd::On(date("2027-01-30"));
    assert!(rule.validate().is_err());
    rule.end = RepeatEnd::Never;
    rule.month = Some(13);
    assert!(rule.validate().is_err());
}

#[test]
fn outlook_combined_weekday_ordinals_and_graph_ordinals_remain_distinct() {
    let rule = Recurrence {
        anchor: Some(date("2027-01-01")),
        days_of_week: vec![RepeatDay::Thursday, RepeatDay::Friday],
        ordinal: Some(WeekIndex::Second),
        ..Recurrence::monthly()
    };
    assert_eq!(
        rule.first_occurrence(date("2027-01-01")),
        Some(date("2027-01-08"))
    );
    let outlook = Recurrence {
        relative_mode: RelativeMode::NthMatchingDay,
        ..rule
    };
    assert_eq!(
        outlook.first_occurrence(date("2027-01-01")),
        Some(date("2027-01-07"))
    );
    let weekdays = Recurrence {
        anchor: Some(date("2026-11-28")),
        end: RepeatEnd::After(2),
        ..Recurrence::weekdays()
    };
    assert_eq!(dates(&weekdays, 3), ["2026-11-30", "2026-12-01"].map(date));
}

#[test]
fn finite_completion_duplicate_and_explicit_seed_removal_are_durable() {
    let mut state = AppState::new();
    let today = date("2027-01-31");
    let id = state
        .apply(
            Action::AddTask {
                title: "Finite".into(),
                list_id: TASKS_LIST,
            },
            today,
        )
        .unwrap()
        .created_task
        .unwrap();
    state
        .apply(
            Action::SetDueDate {
                id,
                date: Some(today),
            },
            today,
        )
        .unwrap();
    state
        .apply(
            Action::SetRecurrence {
                id,
                recurrence: Some(Recurrence {
                    end: RepeatEnd::After(2),
                    ..Recurrence::monthly()
                }),
            },
            today,
        )
        .unwrap();
    let next = state
        .apply(
            Action::SetCompleted {
                id,
                completed: true,
            },
            today,
        )
        .unwrap()
        .created_task
        .unwrap();
    let copied = state
        .apply(Action::DuplicateTask(next), today)
        .unwrap()
        .created_task
        .unwrap();
    let rule = state.task(copied).unwrap().recurrence.as_ref().unwrap();
    assert_eq!(rule.anchor, Some(date("2027-02-28")));
    assert_eq!(rule.day_of_month, Some(31));
    assert_eq!(rule.last_occurrence(today), Some(date("2027-03-31")));
    assert!(
        state
            .apply(
                Action::SetCompleted {
                    id: next,
                    completed: true
                },
                date("2027-02-28")
            )
            .unwrap()
            .created_task
            .is_none()
    );
    let task = state
        .tasks
        .iter_mut()
        .find(|task| task.id == copied)
        .unwrap();
    task.microsoft_data = Some(
        serde_json::json!({"recurrence":{"pattern":{"type":"relativeMonthly"}},"unknown":"preserved"}),
    );
    state
        .apply(
            Action::SetRecurrence {
                id: copied,
                recurrence: None,
            },
            today,
        )
        .unwrap();
    assert_eq!(
        state.task(copied).unwrap().microsoft_data.as_ref().unwrap()["recurrence"],
        serde_json::Value::Null
    );
    assert_eq!(
        state.task(copied).unwrap().microsoft_data.as_ref().unwrap()["unknown"],
        "preserved"
    );
    assert_eq!(
        AppState::from_json(&state.to_json().unwrap()).unwrap(),
        state
    );
}
