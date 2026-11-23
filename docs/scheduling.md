# Scheduling, recurrence, and English task entry

The domain owns civil-date calculation and validation. The state layer turns an editor save into one undoable transaction; the native FerroUI controls and browser codecs use that same persisted rule. Completion creates the next scheduled task, keeps the series anchor, and resets the copied checklist/reminder delivery state.

## Calendar rules

The custom repeat editor supports daily intervals, weekday intervals, selected weekly weekdays and week boundaries, absolute monthly/yearly dates, and first/second/third/fourth/last weekday patterns. A range can continue indefinitely, end on an inclusive date, or contain a fixed total number of scheduled occurrences. The optional time zone preserves Microsoft’s range identity; calendar dates are never shifted through UTC.

An interval belongs to the original anchor. A January 31 monthly rule produces February’s last day and then March 31. A February 29 yearly rule returns to February 29 in leap years. A weekly interval keeps the anchor week and its chosen first day of week, including when the first selected weekday occurs later than the range start.

Numbered ranges count scheduled dates, including dates missed while a task was overdue. Completing an overdue occurrence finds the next scheduled date strictly after both the current due date and the completion date. It never creates a backlog of missed copies. Indexed arithmetic and binary search bound the work even when the anchor is decades old.

Changing a due date explicitly reanchors the rule. Saving an unchanged editor preserves a progressed series and its original month-end anchor. Duplicating a task or pasting an independent task copy starts an independent numbered series at its current due date. Moving a task and copying an entire list retain their existing series history. Promoting a step creates an ordinary independent task without inheriting its parent’s schedule.

Task date ranges have separate optional start and due dates. A start after the due date is rejected before mutation. Recurring successors shift the start date and reminder by their original offset from the due date. A single date-range or repeat save is one undo step, including bulk repeat changes.

## Relative weekdays: Microsoft To Do and Outlook

Two public Microsoft formats describe different multi-weekday ordinals:

| Mode | Example: second Thursday or Friday in January 2027 |
| --- | --- |
| Microsoft Graph, first matching weekday | January 8: the earlier of the second Thursday and second Friday |
| Outlook, count matching days | January 7: the second date in the combined Thursday/Friday set |

The model and editor preserve this distinction. “Last weekday” in English entry uses Outlook’s combined-set meaning. A single selected weekday and the first selected matching date have the same meaning in both formats.

Graph supports six recurrence patterns and three range types. Arbitrary business-day intervals and combined-set ordinals that Graph cannot express remain available locally and in backups. Attempting an incompatible Graph upload returns an explicit error rather than changing the schedule.

Sources: [Graph recurrencePattern](https://learn.microsoft.com/en-us/graph/api/resources/recurrencepattern?view=graph-rest-1.0), [Graph recurrenceRange](https://learn.microsoft.com/en-us/graph/api/resources/recurrencerange?view=graph-rest-1.0), [Outlook MonthNth](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxocal/6ac538f5-8ba0-4dfe-a383-09fab61136db).

## Outlook PST preservation

Supported Gregorian schedules use Outlook’s native `PidLidTaskRecurrence` binary, `PidLidTaskFRecurring`, and `PidLidTaskDeadOccurrence`. The writer follows the native interval limits, minute-based date fields, weekday mask, finite last occurrence, and required no-end sentinel. A progressed numbered task exports its remaining count. Completed historical occurrences are marked dead so Outlook does not generate another copy.

Version 3 FerroUI metadata retains the original range start/end, time zone, richer rule, and generated-occurrence state. The importer restores that richer metadata only when Outlook’s native projection is unchanged. An Outlook repeat edit or explicit removal wins. Optional task start dates also survive when a standalone native Outlook task cannot represent them without a due date.

Unrepresentable recurrence formats are retained in metadata with an export warning. Unsupported incoming calendars, sliding completion-relative rules, malformed/future binary structures, and task exceptions stay in the preserved original PST properties; they are not silently converted into a different schedule.

Sources: [TaskRecurrence](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxotask/e3900985-4858-4e13-b785-3005c5d2fc7f), [RecurrencePattern wire format](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxocal/cf7153b4-f8b5-4cb6-bf14-e78d21f94814), [FirstDateTime calculation](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxocal/4cd5abdc-7b06-48f4-afd4-b00b1067e6b1).

## English entry grammar

Recognition is previewed before submission and is controlled by Settings. The default retains the complete title. Optional removal deletes only the recognized span and never produces an empty title. Renaming an existing task does not parse its title.

| Input | Meaning |
| --- | --- |
| `File taxes January 15, 2027` | Due on the explicit named date |
| `Review 15th January 2027` | Equivalent day-first named date |
| `Review in 2 weeks` | Due 14 civil days from today |
| `Call on January 1 at 3 PM` | Reminder on the upcoming January 1 at 15:00 |
| `Review due Dec 5 with Ana` | Explicit embedded due clause; remaining title is preserved |
| `Holiday from Dec 30 through Jan 2` | Start and due dates spanning the year boundary |
| `Work between December 1 and December 5 at noon` | Start, due date, and due-day reminder |
| `Submit report every Mon and Thu until Dec 31` | Weekly weekday set and inclusive end date |
| `Pay rent every month on the 1st at 09:30 for 4 occurrences` | Monthly date, clock reminder, and numbered range |
| `Standup every Mon-Fri at 9am` | Monday–Friday weekly set |
| `Close books every 2 months on the last weekday starting Dec 1` | Anchored relative monthly schedule |
| `Host dinner every year on the fourth Thursday of November` | Relative yearly date |
| `Call on December 5 at 3pm every week for 2 occurrences` | Explicit first date, clock, and finite weekly rule |
| `Every Monday call Mom` | Explicit embedded repeat clause |

Month names accept standard English abbreviations. Without a year, a named date means its next valid occurrence on or after the supplied local date; February 29 finds the next leap year. Explicit years are honored, including past dates. A time without a date remains on the supplied local day, even if already passed. Only explicit relative time expressions such as `in 90 minutes` cross midnight automatically.

Unintroduced date words remain suffix-only. Ambiguous numeric slash dates, alternatives, negations, invalid dates, incomplete ranges, and malformed repeat rules stay literal. Examples include `Read daily news`, `Keep tomorrow in the title`, `Meet every Monday or Tuesday`, `Call on February 30 at 3pm`, and `Pay every 0 days`. Recognition is bounded to 8 KiB and 512 whitespace-delimited tokens; larger titles remain unchanged.

## Regression coverage

Domain tests exercise finite counts, overdue skips, arbitrary weekday sets, interval phase, leap/month ends, both ordinal modes, Date bounds, exact legacy serialization, explicit seed removal, creation/duplicate/completion, English grammar, and invalid-input atomicity. State tests cover complete editor validation, one-step undo/redo, unchanged progressed-series saves, and bulk validation. JavaScript tests cover all Graph pattern/range combinations, time-zone preservation, archive migration, native PST fields, resumed counts, and deterministic PST fixtures parsed independently by the Rust Microsoft PST reader. `scripts/ui/recurrence.mjs` exercises the compiled native controls, parsed ranges, finite validation, undo/redo, and reload persistence.
