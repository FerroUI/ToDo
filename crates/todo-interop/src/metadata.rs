use crate::{
    named::{Names, PUBLIC},
    properties::text,
    InteropError, Result,
};
use outlook_pst::messaging::message::MessageProperties;
use serde::Deserialize;
use todo_domain::{AppState, Date, LinkedResource, Recurrence, Step, Task};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    kind: String,
    version: u32,
    notes: String,
    rendered_body: String,
    steps: Vec<MetadataStep>,
    my_day: Option<Date>,
    my_day_excluded: Option<Date>,
    recurrence: Option<Recurrence>,
    linked_resources: Vec<LinkedResource>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataStep {
    title: String,
    completed: bool,
}

pub(crate) fn apply(
    props: &MessageProperties,
    names: &Names,
    task: &mut Task,
    document: &mut AppState,
) -> Result<()> {
    let Some(id) = names.find(PUBLIC, "FerroUI.ToDo.Task") else {
        return Ok(());
    };
    let Some(value) = text(props.get(id), 65001)? else {
        return Ok(());
    };
    if value.len() > crate::MAX_DOCUMENT_BYTES {
        return Err(InteropError::new(
            "PST task application data exceeds the import limit.",
        ));
    }
    let metadata: Metadata = serde_json::from_str(&value)
        .map_err(|_| InteropError::new("The PST contains invalid FerroUI task metadata."))?;
    if metadata.kind != "ferroui.todo.pst" || metadata.version != 1 {
        return Err(InteropError::new(
            "This PST contains a newer unsupported FerroUI task format.",
        ));
    }
    if metadata.steps.len() > crate::MAX_ITEMS {
        return Err(InteropError::new("The PST task contains too many steps."));
    }
    // If Outlook changed the note, its edited text wins over the saved note.
    if task.notes == metadata.rendered_body {
        task.notes = metadata.notes;
    }
    for step in metadata.steps {
        let id = document.next_id;
        document.next_id += 1;
        task.steps.push(Step {
            id,
            title: step.title,
            completed: step.completed,
        });
    }
    task.my_day = metadata.my_day;
    task.my_day_excluded = metadata.my_day_excluded;
    task.recurrence = metadata.recurrence;
    task.linked_resources = metadata.linked_resources;
    Ok(())
}
