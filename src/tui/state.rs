use anyhow::{Result, anyhow};
use ratatui::widgets::TableState;

use crate::config::Config;

use super::editor::EditorState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusMessage {
    pub kind: StatusKind,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum Mode {
    Normal,
    Filter,
    Editor(EditorState),
    DeleteConfirm,
    DiscardConfirm(EditorState),
}

#[derive(Debug, Clone)]
pub struct App {
    pub config: Config,
    pub filter: String,
    pub mode: Mode,
    pub status: Option<StatusMessage>,
    pub table_state: TableState,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut table_state = TableState::default();
        if !config.links.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            config,
            filter: String::new(),
            mode: Mode::Normal,
            status: None,
            table_state,
        }
    }

    pub fn visible_primaries(&self) -> Vec<String> {
        let query = self.filter.trim().to_ascii_lowercase();
        self.config
            .links
            .iter()
            .filter(|(primary, entry)| {
                if query.is_empty() {
                    return true;
                }
                let haystacks = [
                    primary.as_str(),
                    entry.url.as_str(),
                    &entry.aliases.join(" "),
                    &entry.tags.join(" "),
                    entry.note.as_deref().unwrap_or_default(),
                ];
                haystacks
                    .into_iter()
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .map(|(primary, _)| primary.clone())
            .collect()
    }

    pub fn selected_primary(&self) -> Option<String> {
        let visible = self.visible_primaries();
        let idx = self.table_state.selected().unwrap_or(0);
        visible.get(idx).cloned()
    }

    pub fn ensure_selection(&mut self) {
        let len = self.visible_primaries().len();
        match len {
            0 => self.table_state.select(None),
            _ => {
                let idx = self.table_state.selected().unwrap_or(0).min(len - 1);
                self.table_state.select(Some(idx));
            }
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        let visible = self.visible_primaries();
        if visible.is_empty() {
            self.table_state.select(None);
            return;
        }
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, visible.len() as isize - 1) as usize;
        self.table_state.select(Some(next));
    }

    pub fn begin_new(&mut self) {
        self.mode = Mode::Editor(EditorState::new());
    }

    pub fn begin_edit(&mut self) -> Result<()> {
        let primary = self
            .selected_primary()
            .ok_or_else(|| anyhow!("no link selected"))?;
        let entry = self
            .config
            .links
            .get(&primary)
            .ok_or_else(|| anyhow!("selected link no longer exists"))?;
        self.mode = Mode::Editor(EditorState::from_existing(&primary, entry));
        Ok(())
    }

    pub fn begin_delete(&mut self) -> Result<()> {
        if self.selected_primary().is_none() {
            return Err(anyhow!("no link selected"));
        }
        self.mode = Mode::DeleteConfirm;
        Ok(())
    }

    pub fn set_info(&mut self, text: impl Into<String>) {
        self.status = Some(StatusMessage {
            kind: StatusKind::Info,
            text: text.into(),
        });
    }

    pub fn set_error(&mut self, text: impl Into<String>) {
        self.status = Some(StatusMessage {
            kind: StatusKind::Error,
            text: text.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::config::CandidateLink;

    use super::*;

    #[test]
    fn filter_narrows_visible_rows() {
        let mut config = Config::default();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "docs".into(),
                    "https://docs.rs".into(),
                    vec!["api".into()],
                    vec!["rust".into()],
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "jira".into(),
                    "https://jira.example".into(),
                    vec![],
                    vec!["ops".into()],
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        let mut app = App::new(config);
        app.filter = "rust".into();
        assert_eq!(app.visible_primaries(), vec!["docs"]);
    }

    #[test]
    fn create_edit_and_delete_states_transition() {
        let mut config = Config::default();
        config
            .save_link(
                None,
                CandidateLink::new(
                    "docs".into(),
                    "https://docs.rs".into(),
                    vec![],
                    vec![],
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        let mut app = App::new(config);

        app.begin_new();
        assert!(matches!(app.mode, Mode::Editor(_)));

        app.mode = Mode::Normal;
        app.begin_edit().unwrap();
        assert!(matches!(app.mode, Mode::Editor(_)));

        app.mode = Mode::Normal;
        app.begin_delete().unwrap();
        assert!(matches!(app.mode, Mode::DeleteConfirm));
    }
}
