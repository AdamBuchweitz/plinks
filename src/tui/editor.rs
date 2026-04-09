use anyhow::Result;

use crate::config::CandidateLink;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Primary,
    Url,
    Aliases,
    Tags,
    Note,
}

impl Field {
    pub const ALL: [Field; 5] = [
        Field::Primary,
        Field::Url,
        Field::Aliases,
        Field::Tags,
        Field::Note,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Field::Primary => "Primary",
            Field::Url => "URL",
            Field::Aliases => "Aliases",
            Field::Tags => "Tags",
            Field::Note => "Note",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorState {
    pub original_primary: Option<String>,
    pub original_url: Option<String>,
    pub original_aliases: Option<String>,
    pub original_tags: Option<String>,
    pub original_note: Option<String>,
    pub primary: String,
    pub url: String,
    pub aliases: String,
    pub tags: String,
    pub note: String,
    pub active_field: Field,
    pub error: Option<String>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            original_primary: None,
            original_url: None,
            original_aliases: None,
            original_tags: None,
            original_note: None,
            primary: String::new(),
            url: String::new(),
            aliases: String::new(),
            tags: String::new(),
            note: String::new(),
            active_field: Field::Primary,
            error: None,
        }
    }

    pub fn from_existing(primary: &str, entry: &crate::config::LinkEntry) -> Self {
        Self {
            original_primary: Some(primary.to_string()),
            original_url: Some(entry.url.clone()),
            original_aliases: Some(entry.aliases.join(", ")),
            original_tags: Some(entry.tags.join(", ")),
            original_note: Some(entry.note.clone().unwrap_or_default()),
            primary: primary.to_string(),
            url: entry.url.clone(),
            aliases: entry.aliases.join(", "),
            tags: entry.tags.join(", "),
            note: entry.note.clone().unwrap_or_default(),
            active_field: Field::Primary,
            error: None,
        }
    }

    pub fn next_field(&mut self) {
        self.active_field = match self.active_field {
            Field::Primary => Field::Url,
            Field::Url => Field::Aliases,
            Field::Aliases => Field::Tags,
            Field::Tags => Field::Note,
            Field::Note => Field::Primary,
        };
    }

    pub fn previous_field(&mut self) {
        self.active_field = match self.active_field {
            Field::Primary => Field::Note,
            Field::Url => Field::Primary,
            Field::Aliases => Field::Url,
            Field::Tags => Field::Aliases,
            Field::Note => Field::Tags,
        };
    }

    pub fn insert_char(&mut self, ch: char) {
        self.active_string().push(ch);
    }

    pub fn backspace(&mut self) {
        self.active_string().pop();
    }

    pub fn is_dirty(&self) -> bool {
        match self.original_primary.as_ref() {
            Some(original_primary) => {
                self.primary != *original_primary
                    || self.url != self.original_url.as_deref().unwrap_or_default()
                    || self.aliases != self.original_aliases.as_deref().unwrap_or_default()
                    || self.tags != self.original_tags.as_deref().unwrap_or_default()
                    || self.note != self.original_note.as_deref().unwrap_or_default()
            }
            None => {
                !self.primary.is_empty()
                    || !self.url.is_empty()
                    || !self.aliases.is_empty()
                    || !self.tags.is_empty()
                    || !self.note.is_empty()
            }
        }
    }

    pub fn build_candidate(&self) -> Result<CandidateLink> {
        CandidateLink::new(
            self.primary.clone(),
            self.url.clone(),
            parse_csv(&self.aliases),
            parse_csv(&self.tags),
            Some(self.note.clone()),
        )
    }

    fn active_string(&mut self) -> &mut String {
        match self.active_field {
            Field::Primary => &mut self.primary,
            Field::Url => &mut self.url,
            Field::Aliases => &mut self.aliases,
            Field::Tags => &mut self.tags,
            Field::Note => &mut self.note,
        }
    }
}

fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_candidate_from_editor() {
        let editor = EditorState {
            original_primary: None,
            original_url: None,
            original_aliases: None,
            original_tags: None,
            original_note: None,
            primary: "Docs".into(),
            url: "https://docs.rs".into(),
            aliases: "Rust, api".into(),
            tags: "guide, rust".into(),
            note: " docs ".into(),
            active_field: Field::Primary,
            error: None,
        };

        let candidate = editor.build_candidate().unwrap();
        assert_eq!(candidate.primary, "docs");
        assert_eq!(candidate.entry.aliases, vec!["api", "rust"]);
        assert_eq!(candidate.entry.tags, vec!["guide", "rust"]);
        assert_eq!(candidate.entry.note.as_deref(), Some("docs"));
    }

    #[test]
    fn dirty_tracking_only_flags_actual_changes() {
        let mut editor = EditorState::new();
        assert!(!editor.is_dirty());

        editor.primary = "docs".into();
        assert!(editor.is_dirty());

        let entry = crate::config::LinkEntry {
            url: "https://docs.rs".into(),
            aliases: vec!["api".into()],
            tags: vec!["rust".into()],
            note: Some("Reference".into()),
        };
        let mut existing = EditorState::from_existing("docs", &entry);
        assert!(!existing.is_dirty());

        existing.note = "Changed".into();
        assert!(existing.is_dirty());
    }
}
