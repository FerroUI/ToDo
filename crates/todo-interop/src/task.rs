use crate::{
    named::{Names, COMMON, PUBLIC, TASK},
    properties::{boolean, decode, integer, raw, text, timestamp},
    InteropError, Result, UtcDate, UtcReminder,
};
use chrono::{SecondsFormat, Timelike};
use outlook_pst::ltp::prop_context::PropertyValue;
use outlook_pst::messaging::message::MessageProperties;
use serde_json::{json, Map};
use todo_domain::{Date, Reminder, Task};

pub(crate) struct Projection {
    pub task: Task,
    pub utc_reminder: Option<UtcReminder>,
    pub utc_dates: Vec<UtcDate>,
    pub warnings: Vec<String>,
}

pub(crate) fn project(
    props: &MessageProperties,
    names: &Names,
    id: u64,
    list: u64,
    today: Date,
) -> Result<Projection> {
    let codepage = integer(props.get(0x3ffd))
        .or_else(|| integer(props.get(0x3fde)))
        .unwrap_or(1252);
    let codepage = u32::try_from(codepage)
        .map_err(|_| InteropError::new("The PST text code page is invalid."))?;
    let get = |guid, lid| names.id(guid, lid).and_then(|id| props.get(id));
    let mut warnings = Vec::new();
    let mut title = text(props.get(0x0037), codepage)?.unwrap_or_default();
    if title.starts_with('\u{0001}') {
        title = title.chars().skip(2).collect();
    }
    if title.trim().is_empty() {
        title = "Untitled task".into();
        warnings.push("An untitled Outlook task was named “Untitled task”.".into());
    }
    let created_time = timestamp(props.get(0x3007))?;
    let updated_time = timestamp(props.get(0x3008))?;
    let created = created_time.map(|date| date.date_naive()).unwrap_or(today);
    let updated = updated_time
        .map(|date| date.date_naive())
        .unwrap_or(created);
    let completed_time = timestamp(get(TASK, 0x810f))?;
    let completed_on = completed_time.map(|date| date.date_naive());
    let status = integer(get(TASK, 0x8101)).unwrap_or(0);
    let completed = boolean(get(TASK, 0x811c)).unwrap_or(status == 2);
    let due = timestamp(get(TASK, 0x8105))?;
    let start = timestamp(get(TASK, 0x8104))?;
    let importance = integer(props.get(0x0017)).unwrap_or(1);
    let plain = text(props.get(0x1000), codepage)?;
    let html = match props.get(0x1013) {
        Some(PropertyValue::Binary(value)) => Some(decode(
            value.buffer(),
            integer(props.get(0x3fde)).unwrap_or(65001) as u32,
        )?),
        value => text(value, codepage)?,
    };
    let rich_note = html
        .as_deref()
        .filter(|html| !html.is_empty())
        .map(todo_domain::notes::RichNote::from_html)
        .transpose()
        .map_err(InteropError::new)?;
    // PidTagNativeBody=1 makes an Outlook plain-body edit authoritative even
    // when the producer retained an older HTML alternative in the message.
    let use_html =
        rich_note.is_some() && (integer(props.get(0x1016)) != Some(1) || plain.is_none());
    let notes = if use_html {
        rich_note.as_ref().unwrap().text()
    } else {
        plain.clone().unwrap_or_default()
    };
    if plain.is_none() && props.get(0x1009).is_some() && html.is_none() {
        warnings.push("An RTF-only task body is preserved in the original task data; its plain-text note is empty.".into());
    }
    let categories = names.find(PUBLIC, "Keywords").and_then(|id| props.get(id));
    let tags = match categories {
        Some(PropertyValue::MultipleUnicode(values)) => values
            .iter()
            .map(|v| {
                text(Some(&PropertyValue::Unicode(v.clone())), codepage)
                    .map(Option::unwrap_or_default)
            })
            .collect::<Result<Vec<_>>>()?,
        Some(PropertyValue::MultipleString8(values)) => values
            .iter()
            .map(|v| decode(v.buffer(), codepage))
            .collect::<Result<Vec<_>>>()?,
        None => Vec::new(),
        _ => {
            return Err(InteropError::new(
                "A PST task has an invalid category collection.",
            ))
        }
    };
    let mut task = Task::new(id, list, title, today);
    task.notes = notes;
    task.notes_html = rich_note
        .as_ref()
        .filter(|note| use_html && note.has_formatting())
        .map(|note| note.to_html());
    task.important = importance == 2;
    task.completed = completed;
    task.created_on = created;
    task.updated_on = updated;
    task.completed_on = completed.then_some(completed_on.unwrap_or(updated));
    task.due_date = due.map(|date| date.date_naive());
    task.start_date = start.map(|date| date.date_naive());
    task.tags = tags;
    let reminder_set = boolean(get(COMMON, 0x8503)).unwrap_or(false);
    let reminder_time = if reminder_set {
        timestamp(get(COMMON, 0x8560))?.or(timestamp(get(COMMON, 0x8502))?)
    } else {
        None
    };
    let utc_reminder = reminder_time.map(|time| {
        task.reminder = Some(Reminder {
            date: time.date_naive(),
            minutes_after_midnight: (time.hour() * 60 + time.minute()) as u16,
            notified: false,
        });
        UtcReminder {
            task_id: id,
            date_time: time.to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    });
    if reminder_set && reminder_time.is_none() {
        warnings.push("A task reminder had no timestamp and was not enabled.".into());
    }
    if boolean(get(TASK, 0x8126)).unwrap_or(get(TASK, 0x8116).is_some()) {
        match get(TASK, 0x8116) {
            Some(PropertyValue::Binary(value)) => match crate::recurrence::decode(value.buffer()) {
                Ok(rule) => {
                    task.recurrence = Some(rule);
                    task.recurrence_generated = boolean(get(TASK, 0x8109)).unwrap_or(task.completed);
                }
                Err(reason) => warnings.push(format!("An Outlook task contains {reason}. Its original binary pattern is preserved; review its repeat schedule before syncing.")),
            },
            _ => warnings.push("An Outlook recurring task has no valid recurrence binary; its original properties are preserved.".into()),
        }
    }
    let mut properties = Map::new();
    let mut named_properties = Map::new();
    for (id, value) in props.iter() {
        if Some(*id) == names.find(PUBLIC, "FerroUI.ToDo.Task") {
            continue;
        }
        properties.insert(format!("{id:04x}"), raw(value, codepage)?);
        if let Some(name) = names.0.get(id) {
            named_properties.insert(format!("{id:04x}"), json!(name));
        }
    }
    let date_value = |date: chrono::DateTime<chrono::Utc>| json!({"dateTime":date.format("%Y-%m-%dT%H:%M:%S").to_string(),"timeZone":"UTC"});
    let body = if let Some(html) = html {
        json!({"contentType":"html","content":html})
    } else {
        json!({"contentType":"text","content":task.notes})
    };
    let mut data = json!({"title":task.title,"body":body,"importance":match importance {0=>"low",2=>"high",_=>"normal"},
        "status":match status {1=>"inProgress",2=>"completed",3=>"waitingOnOthers",4=>"deferred",_=>if completed {"completed"}else{"notStarted"}},
        "categories":task.tags,"isReminderOn":reminder_time.is_some(),
        "pst":{"properties":properties,"namedProperties":named_properties}});
    if let Some(due) = due {
        data["dueDateTime"] = date_value(due);
    }
    if let Some(start) = start {
        data["startDateTime"] = date_value(start);
    }
    if let Some(completed) = completed_time {
        data["completedDateTime"] = date_value(completed);
    }
    if let Some(created) = created_time {
        data["createdDateTime"] = json!(created.to_rfc3339_opts(SecondsFormat::AutoSi, true));
    }
    if let Some(updated) = updated_time {
        data["lastModifiedDateTime"] = json!(updated.to_rfc3339_opts(SecondsFormat::AutoSi, true));
    }
    if let Some(property) = names
        .find(
            "6f305800-7b8b-41af-b73b-84de53a69d3d",
            "BodyLastModifiedTime",
        )
        .and_then(|id| props.get(id))
    {
        if let Some(time) = timestamp(Some(property))? {
            data["bodyLastModifiedDateTime"] =
                json!(time.to_rfc3339_opts(SecondsFormat::AutoSi, true));
        }
    }
    if let Some(reminder) = reminder_time {
        data["reminderDateTime"] = date_value(reminder);
    }
    // PST properties are retained for inspection and future format-specific
    // exporters. Existing Graph upload seeding whitelists writable Graph fields.
    task.microsoft_data = Some(data);
    let utc_dates = if completed {
        completed_time
            .map(|time| UtcDate {
                task_id: id,
                field: "completed_on",
                date_time: time.to_rfc3339_opts(SecondsFormat::Secs, true),
            })
            .into_iter()
            .collect()
    } else {
        Vec::new()
    };
    Ok(Projection {
        task,
        utc_reminder,
        utc_dates,
        warnings,
    })
}
