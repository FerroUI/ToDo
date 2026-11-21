use crate::{Action, AppState, Date, Recurrence, TASKS_LIST};
use chrono::Duration;

impl AppState {
    /// Optional sample data, used only when the user explicitly requests a demonstration.
    pub fn sample(today: Date) -> Self {
        let mut state = Self::new();
        let groceries = state
            .apply(
                Action::AddList {
                    name: "Groceries".into(),
                },
                today,
            )
            .unwrap()
            .created_list
            .unwrap();
        let personal = state
            .apply(
                Action::AddList {
                    name: "Personal".into(),
                },
                today,
            )
            .unwrap()
            .created_list
            .unwrap();
        let work = state
            .apply(
                Action::AddList {
                    name: "Work".into(),
                },
                today,
            )
            .unwrap()
            .created_list
            .unwrap();
        let tasks = [
            ("Plan the week ahead", TASKS_LIST, true, true),
            ("Go for a walk", personal, false, true),
            ("Read a chapter", personal, false, true),
            ("Review the new designs", work, true, true),
            ("Send the project update", work, false, false),
            ("Fresh vegetables", groceries, false, false),
            ("Coffee beans", groceries, false, false),
            ("Book a weekend getaway", personal, true, false),
        ];
        for (title, list_id, important, my_day) in tasks {
            let id = state
                .apply(
                    Action::AddTask {
                        title: title.into(),
                        list_id,
                    },
                    today,
                )
                .unwrap()
                .created_task
                .unwrap();
            if important {
                state
                    .apply(Action::SetImportant { id, important }, today)
                    .unwrap();
            }
            if my_day {
                state
                    .apply(
                        Action::SetMyDay {
                            id,
                            day: Some(today),
                        },
                        today,
                    )
                    .unwrap();
            }
        }
        let review = state
            .tasks
            .iter()
            .find(|task| task.title == "Review the new designs")
            .unwrap()
            .id;
        state.apply(Action::Batch(vec![
            Action::SetDueDate { id: review, date: Some(today) },
            Action::SetNotes { id: review, notes: "A little progress every day adds up.\nReview the latest screens and share feedback with the team.".into() },
            Action::AddStep { task_id: review, title: "Review the desktop layout".into() },
            Action::AddStep { task_id: review, title: "Check the mobile experience".into() },
            Action::AddStep { task_id: review, title: "Share feedback".into() },
        ]), today).unwrap();
        let update = state
            .tasks
            .iter()
            .find(|task| task.title == "Send the project update")
            .unwrap()
            .id;
        state
            .apply(
                Action::SetDueDate {
                    id: update,
                    date: today.checked_add_signed(Duration::days(2)),
                },
                today,
            )
            .unwrap();
        let walk = state
            .tasks
            .iter()
            .find(|task| task.title == "Go for a walk")
            .unwrap()
            .id;
        state
            .apply(
                Action::SetRecurrence {
                    id: walk,
                    recurrence: Some(Recurrence::daily()),
                },
                today,
            )
            .unwrap();
        state
    }
}
