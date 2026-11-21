use super::Row;
use ferroui_base::{collections::FerroList, BoxedValue};

pub(super) fn reconcile(
    items: &FerroList<Option<BoxedValue>>,
    mut previous: Vec<Row>,
    next: &[Row],
) {
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
