use super::Row;
use ferroui_base::{collections::FerroList, BoxedValue};
use std::collections::HashSet;

pub(super) fn reconcile(
    items: &FerroList<Option<BoxedValue>>,
    mut previous: Vec<Row>,
    next: &[Row],
) {
    // Most notifications change a row's fields, not its identity/order. Keep that
    // path free of identity maps and leave native selection/virtualization intact.
    if previous.len() == next.len()
        && previous
            .iter()
            .zip(next)
            .all(|(old, row)| old.same_key(row))
    {
        for (index, (old, row)) in previous.iter().zip(next).enumerate() {
            if old != row {
                items.set(index, row.boxed());
            }
        }
        return;
    }

    // Remove obsolete identities before moving surviving rows. Moving all remaining
    // rows ahead of a deleted selection temporarily carries its native anchor to
    // the tail, causing ListBox to scroll there before the final removal arrives.
    // Reverse contiguous ranges preserve source indexes and batch each deletion.
    let retained: HashSet<_> = next.iter().map(Row::key).collect();
    let mut end = previous.len();
    while end > 0 {
        if retained.contains(&previous[end - 1].key()) {
            end -= 1;
            continue;
        }
        let mut start = end - 1;
        while start > 0 && !retained.contains(&previous[start - 1].key()) {
            start -= 1;
        }
        previous.drain(start..end);
        items.remove_range(start, end - start);
        end = start;
    }

    // A wholesale sort/filter must not perform quadratic searches and Vec moves.
    let displaced = previous
        .iter()
        .zip(next)
        .filter(|(a, b)| !a.same_key(b))
        .take(65)
        .count();
    if next.len().max(previous.len()) > 256 && displaced > 64 {
        items.clear();
        items.add_range(next.iter().map(Row::boxed));
        return;
    }
    for (index, row) in next.iter().enumerate() {
        if previous.get(index).is_some_and(|old| old.same_key(row)) {
            if previous[index] != *row {
                previous[index] = row.clone();
                items.set(index, row.boxed());
            }
        } else if let Some(position) = previous
            .iter()
            .enumerate()
            .skip(index)
            .find_map(|(i, old)| old.same_key(row).then_some(i))
        {
            let old = previous.remove(position);
            previous.insert(index, old);
            items.move_item(position, index);
            if previous[index] != *row {
                previous[index] = row.clone();
                items.set(index, row.boxed());
            }
        } else {
            previous.insert(index, row.clone());
            items.insert(index, row.boxed());
        }
    }
    if previous.len() > next.len() {
        items.remove_range(next.len(), previous.len() - next.len());
        previous.truncate(next.len());
    }
}
