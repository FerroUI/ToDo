use crate::Task;

/// Compile once per projection and reuse field storage while scanning its tasks.
/// Terms are ANDed across fields; each term remains a case-insensitive substring.
pub(crate) struct SearchQuery {
    terms: Vec<String>,
    unmatched: Vec<usize>,
    normalized: String,
}

impl SearchQuery {
    pub fn new(query: &str) -> Self {
        let terms: Vec<_> = query
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        Self {
            unmatched: Vec::with_capacity(terms.len()),
            terms,
            normalized: String::new(),
        }
    }

    pub fn matches(&mut self, task: &Task) -> bool {
        if self.terms.is_empty() {
            return false;
        }
        self.unmatched.clear();
        self.unmatched.extend(0..self.terms.len());
        if self.match_field(&task.title, false) || self.match_field(&task.notes, false) {
            return true;
        }
        for step in &task.steps {
            if self.match_field(&step.title, false) {
                return true;
            }
        }
        for tag in &task.tags {
            if self.match_field(tag, true) {
                return true;
            }
        }
        false
    }

    fn match_field(&mut self, field: &str, tag: bool) -> bool {
        if field.is_ascii() {
            self.normalized.clear();
            self.normalized.push_str(field);
            self.normalized.make_ascii_lowercase();
        } else {
            // Use str's full Unicode lowercase rules, including contextual Greek sigma.
            self.normalized = field.to_lowercase();
        }
        let field = &self.normalized;
        let terms = &self.terms;
        self.unmatched.retain(|index| {
            let term = &terms[*index];
            !field.contains(term)
                && !(tag
                    && term
                        .strip_prefix('#')
                        .is_some_and(|suffix| field.starts_with(suffix)))
        });
        self.unmatched.is_empty()
    }
}
