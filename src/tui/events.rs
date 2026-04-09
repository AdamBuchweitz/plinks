use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::editor::EditorState;
use super::state::{App, Mode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResult {
    None,
    Quit,
    Reload,
    OpenSelected,
    SaveEditor(EditorState),
    ConfirmDelete,
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<EventResult> {
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
            EventResult::SaveEditor(editor.clone())
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
