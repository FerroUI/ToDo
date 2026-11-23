//! Clipboard authorization is tied to one successfully written offer and document lifetime.
//! Serialized payloads are portable copies; only an exact local offer can move source IDs
//! or reuse attachment keys from this document's attachment store.
use crate::{AppViewModel, Panel};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use todo_domain::{Action, Id, Task, TaskEntry, View};

pub const MAX_TASK_CLIPBOARD_BYTES: usize = 4 * 1024 * 1024;
const MAX_TASKS: usize = 1000;
thread_local! { static NEXT_OFFER: Cell<u64> = const { Cell::new(1) }; }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskClipboard {
    kind: String,
    version: u32,
    token: u64,
    cut: bool,
    tasks: Vec<Task>,
}

impl TaskClipboard {
    pub fn from_json(value: &str) -> Result<Self, String> {
        if value.len() > MAX_TASK_CLIPBOARD_BYTES {
            return Err("Task clipboard exceeds 4 MiB".into());
        }
        let mut data: Self = serde_json::from_str(value)
            .map_err(|_| "The clipboard task data is invalid".to_owned())?;
        if data.kind != "ferroui.todo.tasks"
            || data.version != 1
            || data.tasks.is_empty()
            || data.tasks.len() > MAX_TASKS
        {
            return Err("The clipboard task format is unsupported".into());
        }
        // Never trust file-store references or remote identities supplied by another process.
        for task in &mut data.tasks {
            portable(task);
        }
        Ok(data)
    }
    pub fn to_json(&self) -> Result<String, String> {
        let json = serde_json::to_string(self).map_err(|error| error.to_string())?;
        if json.len() > MAX_TASK_CLIPBOARD_BYTES {
            return Err("Task clipboard exceeds 4 MiB".into());
        }
        Ok(json)
    }
    pub fn text(&self) -> String {
        self.tasks
            .iter()
            .map(|task| task.title.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn count(&self) -> usize {
        self.tasks.len()
    }
    pub fn is_cut(&self) -> bool {
        self.cut
    }

    /// Standard HTML also supports browsers that reject application-specific media types.
    pub fn to_html(&self) -> Result<String, String> {
        use std::fmt::Write;
        let json = self.to_json()?;
        let mut html = String::with_capacity(json.len() * 2 + 128);
        html.push_str("<ul data-ferroui-todo-v1=\"");
        for byte in json.bytes() {
            let _ = write!(html, "{byte:02x}");
        }
        html.push_str("\">");
        for task in &self.tasks {
            html.push_str("<li>");
            html.push_str(
                &task
                    .title
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            );
            html.push_str("</li>");
        }
        html.push_str("</ul>");
        Ok(html)
    }

    pub fn from_html(html: &str) -> Result<Option<Self>, String> {
        const MARKER: &str = "data-ferroui-todo-v1=\"";
        let Some(start) = html.find(MARKER) else {
            return Ok(None);
        };
        let hex = html[start + MARKER.len()..]
            .split('"')
            .next()
            .unwrap_or_default();
        if hex.len() > MAX_TASK_CLIPBOARD_BYTES * 2 || hex.len() % 2 != 0 {
            return Err("The clipboard task data is invalid or too large".into());
        }
        let mut bytes = Vec::with_capacity(hex.len() / 2);
        for pair in hex.as_bytes().chunks_exact(2) {
            let digit = |b: u8| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            };
            bytes.push(
                digit(pair[0])
                    .zip(digit(pair[1]))
                    .map(|(a, b)| a * 16 + b)
                    .ok_or("The clipboard task data is invalid")?,
            );
        }
        let json = String::from_utf8(bytes).map_err(|_| "The clipboard task data is invalid")?;
        Self::from_json(&json).map(Some)
    }
}

fn portable(task: &mut Task) {
    task.attachments.clear();
    task.external = None;
    task.flagged_email = false;
    task.assigned_to = None;
    task.microsoft_data = None;
    for resource in &mut task.linked_resources {
        resource.id.clear();
        resource.external_id.clear();
    }
}

#[derive(Clone)]
pub struct TaskClipboardOffer {
    pub data: TaskClipboard,
    tasks: Vec<Task>,
    epoch: u64,
}

pub(crate) struct ClipboardSnapshot {
    offer: TaskClipboardOffer,
    cut_pending: bool,
}

pub struct TaskPasteTarget {
    view: View,
    route: View,
    panel: Panel,
    epoch: u64,
}

impl AppViewModel {
    pub fn prepare_task_clipboard(
        &self,
        ids: &[Id],
        cut: bool,
    ) -> Result<TaskClipboardOffer, String> {
        if self.refreshing() {
            return Err("Wait for synchronization before copying tasks".into());
        }
        if ids.is_empty() {
            return Err("Select a task to copy".into());
        }
        if ids.len() > MAX_TASKS {
            return Err("Copy at most 1,000 tasks at a time".into());
        }
        let state = self.state();
        let mut seen = std::collections::HashSet::new();
        let mut tasks = Vec::with_capacity(ids.len());
        for id in ids {
            if !seen.insert(*id) {
                return Err("Select each task only once".into());
            }
            let task = state.task(*id).ok_or("A selected task no longer exists")?;
            if cut && !task.can_move() {
                return Err(
                    "This task stays in its source list. Copy it to create an independent task."
                        .into(),
                );
            }
            tasks.push(task.clone());
        }
        let mut payload = tasks.clone();
        for task in &mut payload {
            portable(task);
        }
        let token = NEXT_OFFER.with(|next| {
            let token = next.get();
            next.set(token.wrapping_add(1));
            token
        });
        let offer = TaskClipboardOffer {
            data: TaskClipboard {
                kind: "ferroui.todo.tasks".into(),
                version: 1,
                token,
                cut,
                tasks: payload,
            },
            tasks,
            epoch: self.clipboard_epoch.get(),
        };
        offer.data.to_json()?;
        Ok(offer)
    }

    /// Call only after the platform confirms the clipboard write succeeded.
    pub fn accept_task_clipboard(&self, offer: TaskClipboardOffer) -> Result<(), String> {
        if offer.epoch != self.clipboard_epoch.get()
            || self.refreshing()
            || offer.data.cut && !self.clipboard_sources_match(&offer.tasks)
        {
            return Err(
                "The source tasks changed while the clipboard was opening. Copy or cut them again."
                    .into(),
            );
        }
        let count = offer.data.count();
        let cut = offer.data.cut;
        self.task_clipboard.replace(Some(ClipboardSnapshot {
            offer,
            cut_pending: cut,
        }));
        self.set_status(format!(
            "{count} {}. {}",
            if cut { "tasks cut" } else { "tasks copied" },
            if cut {
                "Paste into a list to move them."
            } else {
                "Paste into a list to create copies."
            }
        ));
        Ok(())
    }

    pub fn clear_task_clipboard(&self) {
        self.task_clipboard.borrow_mut().take();
        self.clipboard_epoch
            .set(self.clipboard_epoch.get().wrapping_add(1));
    }

    pub fn task_paste_target(&self, view: View) -> TaskPasteTarget {
        TaskPasteTarget {
            view,
            route: self.view(),
            panel: self.panel(),
            epoch: self.clipboard_epoch.get(),
        }
    }

    pub fn paste_task_clipboard(
        &self,
        target: TaskPasteTarget,
        data: Option<TaskClipboard>,
        text: &str,
    ) -> Result<(), String> {
        if target.epoch != self.clipboard_epoch.get()
            || target.route != self.view()
            || target.panel != self.panel()
            || self.refreshing()
        {
            return Err(
                "The destination changed while the clipboard was opening. Paste again.".into(),
            );
        }
        if self.state().list(target.view.destination_list()).is_none() {
            return Err("The destination list no longer exists".into());
        }
        if let Some(data) = data {
            let local = self
                .task_clipboard
                .borrow()
                .as_ref()
                .filter(|saved| saved.offer.data == data && saved.offer.epoch == target.epoch)
                .map(|saved| (saved.offer.tasks.clone(), saved.cut_pending));
            if data.cut && local.is_none() {
                return Err("These cut tasks are no longer available in this document. Copy or cut them again.".into());
            }
            if let Some((tasks, true)) = &local {
                if !self.clipboard_sources_match(&tasks) {
                    return Err("The cut tasks changed. Cut them again before moving them.".into());
                }
                let ids: Vec<_> = tasks.iter().map(|task| task.id).collect();
                let count = ids.len();
                let token = data.token;
                let list_id = target.view.destination_list();
                let mut actions = vec![Action::TransferTasks {
                    ids: ids.clone(),
                    list_id,
                    before: None,
                }];
                for task in tasks {
                    let id = task.id;
                    let action = match &target.view {
                        View::MyDay => Some(Action::SetMyDay {
                            id,
                            day: Some(self.today()),
                        }),
                        View::Important => Some(Action::SetImportant {
                            id,
                            important: true,
                        }),
                        View::Planned if task.due_date.is_none() && task.reminder.is_none() => {
                            Some(Action::SetDueDate {
                                id,
                                date: Some(
                                    task.start_date.unwrap_or(self.today()).max(self.today()),
                                ),
                            })
                        }
                        View::AssignedToMe => Some(Action::AssignTask {
                            id,
                            assignee: Some("me".into()),
                        }),
                        _ => None,
                    };
                    actions.extend(action);
                }
                self.dispatch_after(Action::Batch(actions), move |vm, changes| {
                    if let Some(saved) = vm
                        .task_clipboard
                        .borrow_mut()
                        .as_mut()
                        .filter(|saved| saved.offer.data.token == token)
                    {
                        saved.cut_pending = false;
                    }
                    if vm.selection().iter().all(|id| ids.contains(id)) {
                        vm.end_selection();
                    }
                    if changes.changed {
                        vm.set_undo_status(format!(
                            "{count} tasks moved. You can undo this change."
                        ));
                    } else {
                        vm.set_status("These tasks are already in this list.".into());
                    }
                });
                return Ok(());
            }
            let tasks = local.map(|(tasks, _)| tasks).unwrap_or(data.tasks);
            self.paste_copies(tasks, target.view);
            return Ok(());
        }
        let titles = clipboard_task_lines(text)?;
        let count = titles.len();
        let (recognize, remove) = {
            let state = self.state();
            (
                state.settings.recognize_dates,
                state.settings.remove_recognized_dates,
            )
        };
        let actions = titles
            .into_iter()
            .map(|title| Action::AddTaskEntry {
                entry: if recognize {
                    TaskEntry::parse_english(&title, self.today(), self.current_minutes(), remove)
                } else {
                    TaskEntry::plain(title)
                },
                view: target.view.clone(),
            })
            .collect();
        self.dispatch_after(Action::Batch(actions), move |vm, changes| {
            if changes.changed {
                vm.set_undo_status(format!("{count} tasks pasted. You can undo this change."));
            }
        });
        Ok(())
    }

    fn paste_copies(&self, tasks: Vec<Task>, view: View) {
        let count = tasks.len();
        self.dispatch_after(Action::PasteTasks { tasks, view }, move |vm, changes| {
            if changes.changed {
                vm.set_undo_status(format!("{count} tasks pasted. You can undo this change."));
            }
        });
    }

    fn clipboard_sources_match(&self, tasks: &[Task]) -> bool {
        let state = self.state();
        tasks
            .iter()
            .all(|task| task.can_move() && state.task(task.id) == Some(task))
    }
}

pub fn clipboard_task_lines(text: &str) -> Result<Vec<String>, String> {
    if text.len() > MAX_TASK_CLIPBOARD_BYTES {
        return Err("Task clipboard exceeds 4 MiB".into());
    }
    let mut lines = Vec::new();
    for line in text.split(['\r', '\n']) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Common copied lists become tasks while meaningful leading hyphens remain intact.
        let line = line
            .strip_prefix("- [ ] ")
            .or_else(|| line.strip_prefix("* [ ] "))
            .or_else(|| line.strip_prefix("- "))
            .or_else(|| line.strip_prefix("* "))
            .or_else(|| line.strip_prefix("• "))
            .unwrap_or(line)
            .trim();
        if line.is_empty() {
            continue;
        }
        if line.contains('\0') {
            return Err("The clipboard contains invalid text".into());
        }
        lines.push(line.to_owned());
        if lines.len() > MAX_TASKS {
            return Err("Paste at most 1,000 tasks at a time".into());
        }
    }
    if lines.is_empty() {
        return Err("The clipboard has no task text".into());
    }
    Ok(lines)
}
