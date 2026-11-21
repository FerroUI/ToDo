import { collection, graph, listPath, taskPath, readCloud } from "./client.js";
import { fields, remoteFields, remoteSnapshot, mergeFields, patchFor, equal } from "./codec.js";
import { checkSteps, syncSteps, adoptSteps } from "./steps.js";
import { assertDocument, checkpoint, commit, identity, loadMetadata } from "./metadata.js";

const nameFields = list => ({ name: list.name });
const remoteName = list => ({ name: list.displayName });
const entityKey = (listId, taskId) => `${listId}\0${taskId}`;
const localStepFields = step => ({ title: step.title, completed: !!step.completed });
const localOnly = task => ({ my_day: task.my_day, my_day_excluded: task.my_day_excluded, attachments: task.attachments, assigned_to: task.assigned_to, position: task.position });

function localTaskChanged(task, record) {
  return !equal(fields(task), record.baseLocal) || (record.localOnly && !equal(localOnly(task), record.localOnly)) || task.list_id !== record.localListId
    || task.steps.some(step => !record.steps?.[step.id] || !equal(localStepFields(step), record.steps[step.id].baseLocal))
    || Object.keys(record.steps || {}).some(id => !task.steps.some(step => step.id === Number(id)));
}

function mapping(task, remote, listId) {
  return { remoteId: remote.id, listId, localListId: task.list_id, baseLocal: fields(task), baseRemote: remoteFields(remote),
    snapshot: remoteSnapshot(remote), etag: remote["@odata.etag"], localOnly: localOnly(task), steps: {} };
}

function newTask(id, list, remote, today) {
  const projected = remoteFields(remote);
  return { id, list_id: list.id, ...projected, my_day: null, my_day_excluded: null,
    reminder: projected.reminder ? { ...projected.reminder, notified: false } : null, recurrence_generated: remote.status === "completed",
    steps: [], attachments: [], assigned_to: null, flagged_email: list.flagged,
    created_on: remote.createdDateTime?.slice(0, 10) || today, updated_on: today,
    completed_on: remote.status === "completed" ? remote.completedDateTime?.dateTime?.slice(0, 10) || today : null,
    position: id };
}

function applyFields(task, merged, remote, today) {
  const before = fields(task);
  const notified = equal(before.reminder, merged.reminder) && !!task.reminder?.notified;
  Object.assign(task, merged);
  if (task.reminder) task.reminder = { ...task.reminder, notified };
  if (!equal(before, merged)) task.updated_on = today;
  if (before.completed !== merged.completed) {
    task.completed_on = merged.completed ? remote.completedDateTime?.dateTime?.slice(0, 10) || today : null;
    task.recurrence_generated = merged.completed && !!merged.recurrence;
  }
}

async function readTask(record) {
  const path = taskPath(record.listId, record.remoteId);
  const remote = await graph(`${path}?$expand=checklistItems,linkedResources`);
  if (!Array.isArray(remote.checklistItems) || remote["checklistItems@odata.nextLink"]) remote.checklistItems = await collection(`${path}/checklistItems`);
  return remote;
}

export async function synchronize(runtime, accountId, today) {
  const expected = runtime.exportData();
  const original = JSON.parse(expected);
  const draft = structuredClone(original);
  const { key, meta } = loadMetadata(accountId);
  meta.pendingTasks ||= {};
  meta.deletedTasks ||= [];
  const guard = () => assertDocument(runtime, expected);
  const save = () => checkpoint(key, meta, original);
  const allocate = () => draft.next_id++;
  guard();
  let cloud = await readCloud();
  // Validate all projected data before a write, including time zones we cannot represent.
  for (const list of cloud) for (const task of list.tasks) remoteFields(task);
  const initialCloud = new Map(cloud.map(list => [list.id, list]));
  const remoteTasks = new Map(cloud.flatMap(list => list.tasks.map(task => [entityKey(list.id, task.id), task])));

  // Interrupted list creation is adopted only when there is one unambiguous new server list.
  for (const [id, pending] of Object.entries(meta.pendingLists || {})) {
    if (meta.lists[id]) { delete meta.pendingLists[id]; continue; }
    const found = cloud.filter(list => !pending.before.includes(list.id) && list.displayName === pending.name);
    const local = draft.lists.find(list => list.id === Number(id));
    if (!found.length && local && local.name !== pending.name) { delete meta.pendingLists[id]; save(); continue; }
    if (found.length !== 1) throw new Error(`Creating “${pending.name}” was interrupted. Check Microsoft To Do; if the list was not created, rename it here and sync again.`);
    if (!local) throw new Error("A list was removed locally while its cloud creation was interrupted.");
    meta.lists[id] = { remoteId: found[0].id, baseLocal: nameFields(local), baseRemote: remoteName(found[0]) };
    delete meta.pendingLists[id]; save();
  }

  const defaultList = cloud.find(list => list.wellknownListName === "defaultList");
  if (!defaultList) throw new Error("Microsoft did not return the default Tasks list.");
  meta.lists[1] ||= { remoteId: defaultList.id, baseLocal: { name: "Tasks" }, baseRemote: remoteName(defaultList) };
  for (const [id, pending] of Object.entries(meta.pendingTasks)) {
    if (meta.tasks[id]?.listId === pending.listId) { delete meta.pendingTasks[id]; continue; }
    const local = draft.tasks.find(task => task.id === Number(id));
    const candidates = cloud.filter(list => list.id === pending.listId).flatMap(list => list.tasks.filter(task => task.linkedResources?.some(link => link.applicationName === "FerroUI To Do" && link.externalId === `${identity()}:${id}`)).map(task => ({ list, task })));
    if (!candidates.length && local && local.title !== pending.fields.title) { delete meta.pendingTasks[id]; save(); continue; }
    if (!local || candidates.length !== 1) throw new Error(`Uploading “${pending.fields.title}” was interrupted. Check Microsoft To Do; if it was not created, rename it here and sync again.`);
    const { list, task } = candidates[0];
    const previous = meta.tasks[id];
    if (previous && previous.listId !== list.id) meta.pendingDeletes.push({ path: taskPath(previous.listId, previous.remoteId), etag: previous.etag });
    meta.tasks[id] = { ...mapping(local, task, list.id), baseLocal: pending.fields, baseRemote: pending.fields };
    delete meta.pendingTasks[id]; save();
  }

  // Detect conflicts before applying any remote mutation.
  for (const [id, record] of Object.entries(meta.tasks)) {
    const local = draft.tasks.find(task => task.id === Number(id));
    const remote = remoteTasks.get(entityKey(record.listId, record.remoteId));
    if (!local && remote && (!equal(remoteSnapshot(remote), record.snapshot) || (record.etag && remote["@odata.etag"] !== record.etag))) throw new Error(`“${remote.title}” was deleted here and changed in Microsoft. Neither copy was overwritten.`);
    if (local && !remote && localTaskChanged(local, record)) throw new Error(`“${local.title}” changed here and was deleted in Microsoft. Keep a local copy or resolve that change before syncing.`);
    if (local && remote) {
      mergeFields(fields(local), remoteFields(remote), record.baseLocal, record.baseRemote, local.title);
      checkSteps(local, remote, record);
      if (local.list_id !== record.localListId) {
        if (remote.hasAttachments) throw new Error(`“${local.title}” has Microsoft file attachments. Move it in Microsoft To Do to keep those files together.`);
        if (initialCloud.get(record.listId)?.wellknownListName === "flaggedEmails") throw new Error("Flagged email tasks cannot be moved to another Microsoft list here.");
      }
    }
  }
  const deletingLists = new Set();
  for (const [id, record] of Object.entries(meta.lists)) {
    const local = draft.lists.find(list => list.id === Number(id));
    const remote = initialCloud.get(record.remoteId);
    if (!local && remote) {
      if (remote.wellknownListName !== "none" || remote.isOwner === false) throw new Error(`The Microsoft list “${remote.displayName}” cannot be deleted here.`);
      const mapped = new Set(Object.values(meta.tasks).filter(task => task.listId === remote.id).map(task => task.remoteId));
      if (!equal(remoteName(remote), record.baseRemote) || remote.tasks.some(task => !mapped.has(task.id))) throw new Error(`List “${remote.displayName}” has new changes in Microsoft. Its local deletion was not sent.`);
      deletingLists.add(Number(id));
    } else if (local && !remote) {
      if (!equal(nameFields(local), record.baseLocal) || draft.tasks.some(task => task.list_id === local.id && (!meta.tasks[task.id] || localTaskChanged(task, meta.tasks[task.id])))) throw new Error(`List “${local.name}” changed locally and was deleted in Microsoft.`);
    } else if (local && remote && Number(id) !== 1) mergeFields(nameFields(local), remoteName(remote), record.baseLocal, record.baseRemote, local.name);
  }

  for (const [id, record] of Object.entries(meta.lists)) {
    const local = draft.lists.find(list => list.id === Number(id));
    const remote = initialCloud.get(record.remoteId);
    if (local && !remote) {
      draft.lists = draft.lists.filter(list => list.id !== local.id);
      draft.tasks = draft.tasks.filter(task => task.list_id !== local.id);
      delete meta.lists[id];
    } else if (!local && !remote) {
      delete meta.lists[id];
    } else if (local && remote && Number(id) !== 1) {
      const before = nameFields(local);
      const merged = mergeFields(before, remoteName(remote), record.baseLocal, record.baseRemote, local.name);
      if (merged.name !== remote.displayName) { guard(); await graph(listPath(remote.id), "PATCH", { displayName: merged.name }, remote["@odata.etag"]); }
      local.name = merged.name; record.baseLocal = before; record.baseRemote = merged; save();
    }
  }
  // Only user-created local lists are posted. Microsoft built-in lists are retained as built-ins.
  for (const local of draft.lists) {
    if (meta.lists[local.id]) continue;
    guard();
    meta.pendingLists[local.id] = { name: local.name, before: cloud.map(list => list.id) }; save();
    const remote = await graph("me/todo/lists", "POST", { displayName: local.name });
    remote.tasks = []; cloud.push(remote); initialCloud.set(remote.id, remote);
    meta.lists[local.id] = { remoteId: remote.id, baseLocal: nameFields(local), baseRemote: remoteName(remote) };
    delete meta.pendingLists[local.id]; save();
  }

  for (const [id, record] of Object.entries(meta.tasks)) {
    if (deletingLists.has(record.localListId)) continue;
    const local = draft.tasks.find(task => task.id === Number(id));
    let remote = remoteTasks.get(entityKey(record.listId, record.remoteId));
    if (!local) {
      if (remote) { guard(); await graph(taskPath(record.listId, record.remoteId), "DELETE", undefined, remote["@odata.etag"]); }
      meta.deletedTasks.push(entityKey(record.listId, record.remoteId));
      delete meta.tasks[id]; save(); continue;
    }
    if (!remote) { draft.tasks = draft.tasks.filter(task => task.id !== local.id); delete meta.tasks[id]; continue; }
    const before = fields(local);
    const merged = mergeFields(before, remoteFields(remote), record.baseLocal, record.baseRemote, local.title);
    applyFields(local, merged, remote, today);
    if (local.list_id !== record.localListId) {
      // Graph has no task move operation: create first, checkpoint, then delete the old task.
      const old = { path: taskPath(record.listId, record.remoteId), etag: remote["@odata.etag"] };
      remote = await createTask(local, meta, today, guard, save, remote);
      meta.pendingDeletes.push(old);
      meta.tasks[id] = mapping(local, remote, meta.lists[local.list_id].remoteId);
      delete meta.pendingTasks[id];
      save();
    } else {
      const patch = patchFor(merged, remoteFields(remote), today);
      if (Object.keys(patch).length) { guard(); await graph(taskPath(record.listId, record.remoteId), "PATCH", patch, remote["@odata.etag"]); }
      record.baseLocal = before; record.baseRemote = merged; save();
    }
    const current = meta.tasks[id];
    await syncSteps(local, remote, current, { guard, save, allocate });
    remote = await readTask(current);
    current.snapshot = remoteSnapshot(remote); current.etag = remote["@odata.etag"];
    current.baseRemote = remoteFields(remote); current.baseLocal = before; save();
  }

  for (const pending of [...meta.pendingDeletes]) {
    guard(); await graph(pending.path, "DELETE", undefined, pending.etag);
    meta.pendingDeletes = meta.pendingDeletes.filter(item => item !== pending); save();
  }
  // A completion can cause Microsoft to create the next recurring task, so refresh before imports.
  cloud = await readCloud();
  const remainingRemoteKeys = new Set(cloud.flatMap(list => list.tasks.map(task => entityKey(list.id, task.id))));
  meta.deletedTasks = meta.deletedTasks.filter(key => remainingRemoteKeys.has(key));
  for (const remoteList of cloud) {
    if ([...deletingLists].some(id => meta.lists[id]?.remoteId === remoteList.id)) continue;
    let local = draft.lists.find(list => meta.lists[list.id]?.remoteId === remoteList.id);
    const flagged = remoteList.wellknownListName === "flaggedEmails";
    if (flagged) local = draft.lists.find(list => list.id === 1);
    if (!local) {
      local = { id: allocate(), name: remoteList.displayName || "Untitled list", color: "#3E69E4", background: null, group_id: null, position: draft.next_id, sort: "Manual", show_completed: true };
      draft.lists.push(local);
      meta.lists[local.id] = { remoteId: remoteList.id, baseLocal: nameFields(local), baseRemote: remoteName(remoteList) };
    }
    for (const remote of remoteList.tasks) {
      if (meta.deletedTasks.includes(entityKey(remoteList.id, remote.id))) continue;
      if (Object.values(meta.tasks).some(record => record.listId === remoteList.id && record.remoteId === remote.id)) continue;
      let task;
      const candidates = draft.tasks.filter(item => !meta.tasks[item.id] && item.list_id === local.id && equal(fields(item), remoteFields(remote)) && equal(item.steps.map(localStepFields), (remote.checklistItems || []).map(step => ({ title: step.displayName, completed: !!step.isChecked }))) && original.tasks.some(parent => parent.completed && parent.recurrence_generated && parent.title === item.title && parent.recurrence));
      if (candidates.length === 1) task = candidates[0];
      if (!task) { task = newTask(allocate(), { ...local, flagged }, remote, today); draft.tasks.push(task); }
      const record = mapping(task, remote, remoteList.id);
      adoptSteps(task, remote, record, allocate);
      meta.tasks[task.id] = record;
    }
  }
  for (const task of draft.tasks) {
    if (meta.tasks[task.id]) continue;
    const remote = await createTask(task, meta, today, guard, save);
    const record = mapping(task, remote, meta.lists[task.list_id].remoteId);
    meta.tasks[task.id] = record; delete meta.pendingTasks[task.id]; save();
    await syncSteps(task, { ...remote, checklistItems: [] }, record, { guard, save, allocate });
    const updated = await readTask(record);
    record.snapshot = remoteSnapshot(updated); record.baseRemote = remoteFields(updated); save();
  }
  for (const id of deletingLists) {
    const record = meta.lists[id]; guard();
    const current = cloud.find(list => list.id === record.remoteId);
    const previous = initialCloud.get(record.remoteId);
    if (current) {
      const snapshot = list => ({ name: remoteName(list), tasks: list.tasks.map(task => ({ id: task.id, ...remoteSnapshot(task), etag: task["@odata.etag"] })).sort((a, b) => a.id.localeCompare(b.id)) });
      if (!equal(snapshot(current), snapshot(previous))) throw new Error(`List “${current.displayName}” changed in Microsoft while syncing. Its deletion was not sent.`);
      await graph(listPath(record.remoteId), "DELETE", undefined, current["@odata.etag"]);
    }
    delete meta.lists[id];
    for (const [taskId, task] of Object.entries(meta.tasks)) if (task.localListId === id) delete meta.tasks[taskId];
    save();
  }
  for (const task of draft.tasks) {
    const record = meta.tasks[task.id]; if (!record) continue;
    record.baseLocal = fields(task); record.localListId = task.list_id; record.localOnly = localOnly(task);
    for (const step of task.steps) if (record.steps[step.id]) record.steps[step.id].baseLocal = localStepFields(step);
  }
  for (const list of draft.lists) if (meta.lists[list.id]) meta.lists[list.id].baseLocal = nameFields(list);
  guard(); commit(runtime, expected, draft, key, meta);
}

async function createTask(task, metadata, today, guard, save, source) {
  const list = metadata.lists[task.list_id];
  if (!list) throw new Error("The task's Microsoft list is not available yet.");
  guard();
  metadata.pendingTasks[task.id] = { fields: fields(task), listId: list.remoteId }; save();
  const payload = source ? Object.fromEntries(["body", "categories", "completedDateTime", "dueDateTime", "importance", "isReminderOn", "recurrence", "reminderDateTime", "startDateTime", "status", "title"].filter(key => source[key] !== undefined).map(key => [key, source[key]])) : {};
  Object.assign(payload, patchFor(fields(task), source ? remoteFields(source) : null, today));
  const links = (source?.linkedResources || []).filter(link => link.applicationName !== "FerroUI To Do").map(({ webUrl, applicationName, displayName, externalId }) => ({ webUrl, applicationName, displayName, externalId }));
  payload.linkedResources = [...links, { webUrl: `${location.origin}${location.pathname}`, applicationName: "FerroUI To Do", displayName: "To Do", externalId: `${identity()}:${task.id}` }];
  return graph(`${listPath(list.remoteId)}/tasks`, "POST", payload);
}
