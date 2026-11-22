use todo_domain::{Id, View};

/// Transient panels never enter the persisted task document.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Panel {
    #[default]
    None,
    Suggestions,
    Settings,
    ListMenu,
    Sort,
    Themes,
    NewList,
    NewGroup,
    RenameList(Id),
    RenameGroup(Id),
    MoveTask(Id),
    MoveSelection,
    MoveList(Id),
    DueDate(Id),
    Reminder(Id),
    Repeat(Id),
    TaskMenu(Id),
    StepMenu {
        task_id: Id,
        step_id: Id,
    },
    DeleteTask(Id),
    DeleteSelection,
    DeleteList(Id),
    Help,
    Share,
}

pub fn view_title(view: &View) -> &'static str {
    match view {
        View::MyDay => "My Day",
        View::Important => "Important",
        View::Planned => "Planned",
        View::All => "All",
        View::Completed => "Completed",
        View::AssignedToMe => "Assigned to me",
        View::FlaggedEmail => "Flagged email",
        View::List(_) => "Tasks",
        View::Search(_) => "Search",
    }
}
