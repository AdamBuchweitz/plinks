use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::editor::EditorState;
use super::state::{App, Mode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResult {
    None,
    Quit,
    Reload,
    OpenSelected,
    SaveEditor(Box<EditorState>),
    ConfirmDelete,
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<EventResult> {
    if matches!(key.kind, KeyEventKind::Release) {
        return Ok(EventResult::None);
    }

    match app.mode.clone() {
        Mode::Normal => handle_normal_mode(app, key),
        Mode::Filter => Ok(handle_filter_mode(app, key)),
        Mode::Editor(_) => Ok(handle_editor_mode(app, key)),
        Mode::DeleteConfirm => Ok(handle_delete_confirm_mode(app, key)),
        Mode::DiscardConfirm(_) => Ok(handle_discard_confirm_mode(app, key)),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Result<EventResult> {
    match key.code {
        KeyCode::Char('q') => Ok(EventResult::Quit),
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_selection(1);
            Ok(EventResult::None)
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_selection(-1);
            Ok(EventResult::None)
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Filter;
            Ok(EventResult::None)
        }
        KeyCode::Char('n') => {
            app.begin_new();
            Ok(EventResult::None)
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            if let Err(err) = app.begin_edit() {
                app.set_error(err.to_string());
            }
            Ok(EventResult::None)
        }
        KeyCode::Char('d') => {
            if let Err(err) = app.begin_delete() {
                app.set_error(err.to_string());
            }
            Ok(EventResult::None)
        }
        KeyCode::Char('r') => Ok(EventResult::Reload),
        KeyCode::Char('o') => Ok(EventResult::OpenSelected),
        _ => Ok(EventResult::None),
    }
}

fn handle_filter_mode(app: &mut App, key: KeyEvent) -> EventResult {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            EventResult::None
        }
        KeyCode::Backspace => {
            app.filter.pop();
            app.ensure_selection();
            EventResult::None
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            EventResult::None
        }
        KeyCode::Char(ch) => {
            app.filter.push(ch);
            app.ensure_selection();
            EventResult::None
        }
        _ => EventResult::None,
    }
}

fn handle_editor_mode(app: &mut App, key: KeyEvent) -> EventResult {
    let Some(editor) = (match &mut app.mode {
        Mode::Editor(editor) => Some(editor),
        _ => None,
    }) else {
        return EventResult::None;
    };

    match key.code {
        KeyCode::Esc => {
            if editor.is_dirty() {
                app.mode = Mode::DiscardConfirm(editor.clone());
            } else {
                app.mode = Mode::Normal;
            }
            EventResult::None
        }
        KeyCode::Tab => {
            editor.next_field();
            EventResult::None
        }
        KeyCode::BackTab => {
            editor.previous_field();
            EventResult::None
        }
        KeyCode::Backspace => {
            editor.backspace();
            EventResult::None
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            EventResult::SaveEditor(Box::new(editor.clone()))
        }
        KeyCode::Char(ch) => {
            editor.insert_char(ch);
            EventResult::None
        }
        _ => EventResult::None,
    }
}

fn handle_delete_confirm_mode(app: &mut App, key: KeyEvent) -> EventResult {
    match key.code {
        KeyCode::Char('y') => EventResult::ConfirmDelete,
        KeyCode::Char('n') | KeyCode::Esc => {
            app.mode = Mode::Normal;
            EventResult::None
        }
        _ => EventResult::None,
    }
}

fn handle_discard_confirm_mode(app: &mut App, key: KeyEvent) -> EventResult {
    let Some(editor) = (match &mut app.mode {
        Mode::DiscardConfirm(editor) => Some(editor),
        _ => None,
    }) else {
        return EventResult::None;
    };

    match key.code {
        KeyCode::Char('y') => {
            app.mode = Mode::Normal;
            EventResult::None
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.mode = Mode::Editor(editor.clone());
            EventResult::None
        }
        _ => EventResult::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CandidateLink, Config};
    use crate::tui::editor::Field;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn release_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new_with_kind(code, KeyModifiers::empty(), KeyEventKind::Release)
    }

    fn sample_app() -> App {
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
        App::new(config)
    }

    #[test]
    fn ignores_release_events_in_filter_mode() {
        let mut app = sample_app();
        app.mode = Mode::Filter;

        assert_eq!(
            handle_key(&mut app, key(KeyCode::Char('a'))).unwrap(),
            EventResult::None
        );
        assert_eq!(app.filter, "a");

        assert_eq!(
            handle_key(&mut app, release_key(KeyCode::Char('a'))).unwrap(),
            EventResult::None
        );
        assert_eq!(app.filter, "a");
    }

    #[test]
    fn ignores_release_events_in_editor_mode() {
        let mut app = sample_app();
        app.begin_new();

        assert_eq!(
            handle_key(&mut app, key(KeyCode::Char('a'))).unwrap(),
            EventResult::None
        );
        let Mode::Editor(editor) = &app.mode else {
            panic!("expected editor mode");
        };
        assert_eq!(editor.primary, "a");

        assert_eq!(
            handle_key(&mut app, release_key(KeyCode::Char('a'))).unwrap(),
            EventResult::None
        );
        let Mode::Editor(editor) = &app.mode else {
            panic!("expected editor mode");
        };
        assert_eq!(editor.primary, "a");
    }

    #[test]
    fn ignores_release_events_for_tab_navigation() {
        let mut app = sample_app();
        app.begin_new();

        assert_eq!(
            handle_key(&mut app, key(KeyCode::Tab)).unwrap(),
            EventResult::None
        );
        let Mode::Editor(editor) = &app.mode else {
            panic!("expected editor mode");
        };
        assert_eq!(editor.active_field, Field::Url);

        assert_eq!(
            handle_key(&mut app, release_key(KeyCode::Tab)).unwrap(),
            EventResult::None
        );
        let Mode::Editor(editor) = &app.mode else {
            panic!("expected editor mode");
        };
        assert_eq!(editor.active_field, Field::Url);
    }
}
