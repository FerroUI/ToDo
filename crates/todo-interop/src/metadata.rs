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
    #[serde(default)]
    notes_html: Option<String>,
    #[serde(default)]
    rendered_html: Option<String>,
    steps: Vec<MetadataStep>,
    my_day: Option<Date>,
    my_day_excluded: Option<Date>,
    recurrence: Option<Recurrence>,
    #[serde(default)]
    recurrence_generated: bool,
    #[serde(default)]
    native_recurrence: bool,
    #[serde(default)]
    rendered_recurrence: Option<Recurrence>,
    #[serde(default)]
    start_date: Option<Date>,
    #[serde(default)]
    rendered_start_date: Option<Date>,
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
    if metadata.kind != "ferroui.todo.pst" || !(1..=3).contains(&metadata.version) {
        return Err(InteropError::new(
            "This PST contains a newer unsupported FerroUI task format.",
        ));
    }
    if metadata.steps.len() > crate::MAX_ITEMS {
        return Err(InteropError::new("The PST task contains too many steps."));
    }
    // Both the text and its formatting belong to the body revision. A change
    // made in Outlook, including formatting-only edits, wins over metadata.
    let rich_unchanged = match metadata.rendered_html.as_deref() {
        Some(expected) => task
            .microsoft_data
            .as_ref()
            .and_then(|data| data.get("body"))
            .and_then(|body| body.get("content"))
            .and_then(|value| value.as_str())
            .and_then(|html| todo_domain::notes::RichNote::from_html(html).ok())
            .zip(todo_domain::notes::RichNote::from_html(expected).ok())
            .is_some_and(|(actual, expected)| actual == expected),
        None => task.notes_html.is_none(),
    };
    if task.notes == metadata.rendered_body && rich_unchanged {
        task.notes = metadata.notes;
        task.notes_html = metadata
            .notes_html
            .as_deref()
            .map(todo_domain::notes::RichNote::from_html)
            .transpose()
            .map_err(InteropError::new)?
            .filter(|note| note.text() == task.notes && note.has_formatting())
            .map(|note| note.to_html());
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
    // Preserve Outlook edits (including disabling repeat); only restore the
    // richer range anchor/time zone when its native projection is unchanged.
    if (metadata.native_recurrence && task.recurrence == metadata.rendered_recurrence)
        || (!metadata.native_recurrence && task.recurrence.is_none())
    {
        task.recurrence = metadata.recurrence;
        task.recurrence_generated |= metadata.recurrence_generated;
    }
    if metadata.start_date.is_some() && task.start_date == metadata.rendered_start_date {
        task.start_date = metadata.start_date;
    }
    task.linked_resources = metadata.linked_resources;
    Ok(())
}
