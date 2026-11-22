use crate::{
    files::{self, PstMessage},
    named::Names,
    properties::text,
    reader::PstStore,
    task, InteropError, PortableArchive, PstImport, Result, MAX_DOCUMENT_BYTES, MAX_ITEMS,
};
use outlook_pst::messaging::{
    message::{AnsiMessage, UnicodeMessage},
    store::EntryId,
};
use serde_json::to_vec;
use std::collections::{BTreeSet, HashSet};
use todo_domain::{AppState, Date, TaskList};

pub(crate) fn read(pst: PstStore, current: &AppState, today: Date) -> Result<PstImport> {
    let store = pst.store();
    let names = Names::load(store.as_ref())?;
    let root = store.properties().ipm_sub_tree_entry_id()?;
    let mut pending = vec![(root, String::new(), 0usize)];
    let mut folders = HashSet::new();
    let mut messages = HashSet::new();
    let mut warnings = BTreeSet::new();
    let mut document = AppState::new();
    document.settings = current.settings.clone();
    document.settings.auto_sync = false;
    document.settings.planner_enabled = false;
    let mut result = PstImport {
        archive: PortableArchive {
            kind: "ferroui.todo.backup",
            version: 1,
            document,
            files: Vec::new(),
        },
        warnings: Vec::new(),
        skipped_messages: 0,
        utc_reminders: Vec::new(),
        utc_dates: Vec::new(),
    };
    let mut file_bytes = 0usize;
    let mut metadata_bytes = 0usize;
    let mut default_claimed = false;
    let mut list_count = 0usize;
    while let Some((entry, parent, depth)) = pending.pop() {
        let node = u32::from(entry.node_id());
        if !folders.insert(node) {
            return Err(InteropError::new(
                "The PST folder hierarchy contains a cycle or duplicate folder.",
            ));
        }
        if folders.len() > 10_000 || depth > 64 {
            return Err(InteropError::new(
                "The PST folder hierarchy exceeds the supported depth or count.",
            ));
        }
        let folder = store.open_folder(&entry)?;
        let name = text(folder.properties().get(0x3001), 1252)?.unwrap_or_else(|| "Tasks".into());
        let path = if depth == 0 {
            String::new()
        } else if parent.is_empty() {
            name
        } else {
            format!("{parent} / {name}")
        };
        if let Some(table) = folder.hierarchy_table() {
            let mut children = Vec::new();
            for row in table.rows_matrix() {
                let id = store
                    .properties()
                    .make_entry_id(u32::from(row.id()).into())?;
                children.push((id, path.clone(), depth + 1));
                if pending.len() + children.len() > 10_000 {
                    return Err(InteropError::new("The PST contains too many folders."));
                }
            }
            pending.extend(children.into_iter().rev());
        }
        let mut list_id = if text(folder.properties().get(0x3613), 1252)?
            .is_some_and(|value| value.eq_ignore_ascii_case("IPF.Task"))
        {
            list_count += 1;
            let id = allocate_list(&mut result.archive.document, &path, &mut default_claimed);
            Some(id)
        } else {
            None
        };
        if let Some(table) = folder.contents_table() {
            for row in table.rows_matrix() {
                let node = u32::from(row.id());
                if !messages.insert(node) {
                    return Err(InteropError::new("The PST repeats a message identity."));
                }
                if messages.len() > MAX_ITEMS {
                    return Err(InteropError::new(
                        "The PST contains more than 100,000 items; export a task-only PST.",
                    ));
                }
                let entry = store.properties().make_entry_id(node.into())?;
                let probe = store.open_message(&entry, Some(&[0x001a]))?;
                let class = text(probe.properties().get(0x001a), 1252)?.unwrap_or_default();
                if class != "IPM.Task" && !class.starts_with("IPM.Task.") {
                    result.skipped_messages += 1;
                    continue;
                }
                let message = open_message(&pst, &entry)?;
                let document = &mut result.archive.document;
                let list = *list_id.get_or_insert_with(|| {
                    list_count += 1;
                    allocate_list(document, &path, &mut default_claimed)
                });
                let id = document.next_id;
                document.next_id += 1;
                let mut projection =
                    task::project(message.message().properties(), &names, id, list, today)?;
                crate::metadata::apply(
                    message.message().properties(),
                    &names,
                    &mut projection.task,
                    document,
                )?;
                files::append(
                    &message,
                    document,
                    &mut projection.task,
                    &mut result.archive.files,
                    &mut file_bytes,
                )?;
                if let Some(reminder) = projection.utc_reminder {
                    result.utc_reminders.push(reminder);
                }
                result.utc_dates.extend(projection.utc_dates);
                warnings.extend(projection.warnings);
                metadata_bytes = metadata_bytes
                    .checked_add(
                        to_vec(&projection.task)
                            .map_err(|err| InteropError::new(err.to_string()))?
                            .len(),
                    )
                    .ok_or_else(|| InteropError::new("PST task metadata is too large."))?;
                if metadata_bytes > MAX_DOCUMENT_BYTES {
                    return Err(InteropError::new(
                        "PST task metadata exceeds the 32 MiB document limit.",
                    ));
                }
                document.tasks.push(projection.task);
            }
        }
    }
    if result.archive.document.tasks.is_empty() && list_count == 0 {
        return Err(InteropError::new(format!("This PST contains no supported Outlook tasks ({} non-task items found). Your current tasks were kept.",result.skipped_messages)));
    }
    let document = &result.archive.document;
    document
        .validate()
        .map_err(|err| InteropError::new(format!("The imported tasks are invalid: {err}")))?;
    if to_vec(document)
        .map_err(|err| InteropError::new(err.to_string()))?
        .len()
        > MAX_DOCUMENT_BYTES
    {
        return Err(InteropError::new(
            "The imported task document exceeds 32 MiB.",
        ));
    }
    if result.skipped_messages > 0 {
        warnings.insert(format!(
            "Skipped {} mail, calendar, contact, or other non-task items.",
            result.skipped_messages
        ));
    }
    result.warnings = warnings.into_iter().collect();
    Ok(result)
}

fn allocate_list(document: &mut AppState, path: &str, default_claimed: &mut bool) -> u64 {
    if !*default_claimed && path.eq_ignore_ascii_case("Tasks") {
        *default_claimed = true;
        return todo_domain::TASKS_LIST;
    }
    let id = document.next_id;
    document.next_id += 1;
    document.lists.push(TaskList::new(
        id,
        if path.is_empty() {
            "Imported Outlook tasks".into()
        } else {
            path.into()
        },
    ));
    id
}

fn open_message(store: &PstStore, entry: &EntryId) -> Result<PstMessage> {
    Ok(match store {
        PstStore::Ansi(store) => PstMessage::Ansi(AnsiMessage::read(store.clone(), entry, None)?),
        PstStore::Unicode(store) => {
            PstMessage::Unicode(UnicodeMessage::read(store.clone(), entry, None)?)
        }
    })
}
