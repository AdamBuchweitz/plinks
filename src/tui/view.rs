use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui::{Frame, layout::Alignment};

use super::editor::{EditorState, Field};
use super::state::{App, Mode, StatusKind};
use crate::project_root::ResolvedConfigPath;

pub fn render(frame: &mut Frame<'_>, app: &mut App, resolved: &ResolvedConfigPath) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, vertical[0], resolved);
    render_main(frame, vertical[1], app);
    render_footer(frame, vertical[2], app);

    match &app.mode {
        Mode::Editor(editor) => render_editor(frame, frame.area(), editor),
        Mode::DeleteConfirm => render_center_message(
            frame,
            frame.area(),
            "Delete selected link? Press y to confirm.",
            Color::Yellow,
        ),
        Mode::DiscardConfirm(_) => render_center_message(
            frame,
            frame.area(),
            "Discard unsaved edits? Press y to discard.",
            Color::Yellow,
        ),
        _ => {}
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, resolved: &ResolvedConfigPath) {
    let text = Line::from(vec![
        Span::styled("Project ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(resolved.project_dir.display().to_string()),
        Span::raw("  "),
        Span::styled("File ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(resolved.config_path.display().to_string()),
    ]);
    frame.render_widget(Paragraph::new(text), area);
}

fn render_main(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let primaries = app.visible_primaries();
    let rows = primaries
        .iter()
        .map(|primary| {
            let entry = &app.config.links[primary];
            Row::new(vec![
                Cell::from(primary.clone()),
                Cell::from(entry.aliases.join(", ")),
                Cell::from(entry.tags.join(", ")),
                Cell::from(entry.url.clone()),
            ])
        })
        .collect::<Vec<_>>();

    let widths = [
        Constraint::Length(18),
        Constraint::Length(24),
        Constraint::Length(20),
        Constraint::Min(20),
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["PRIMARY", "ALIASES", "TAGS", "URL"]).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(Block::default().title("Links").borders(Borders::ALL))
        .row_highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol(">> ");
    app.ensure_selection();
    frame.render_stateful_widget(table, sections[0], &mut app.table_state);

    let detail = if let Some(primary) = app.selected_primary() {
        let entry = &app.config.links[&primary];
        vec![
            Line::from(vec![
                Span::styled("Primary: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(primary),
            ]),
            Line::from(vec![
                Span::styled("URL: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(entry.url.clone()),
            ]),
            Line::from(vec![
                Span::styled("Aliases: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(if entry.aliases.is_empty() {
                    "-".to_string()
                } else {
                    entry.aliases.join(", ")
                }),
            ]),
            Line::from(vec![
                Span::styled("Tags: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(if entry.tags.is_empty() {
                    "-".to_string()
                } else {
                    entry.tags.join(", ")
                }),
            ]),
            Line::from(vec![
                Span::styled("Note: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(entry.note.clone().unwrap_or_else(|| "-".into())),
            ]),
        ]
    } else {
        vec![Line::raw("No links match the current filter.")]
    };

    let details = Paragraph::new(detail)
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(details, sections[1]);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let keys = match app.mode {
        Mode::Normal => "q quit  j/k move  / filter  n new  e edit  d delete  o open  r reload",
        Mode::Filter => "type to filter  Enter/Esc finish",
        Mode::Editor(_) => "Tab fields  Ctrl-s save  Esc cancel",
        Mode::DeleteConfirm => "y confirm delete  n cancel",
        Mode::DiscardConfirm(_) => "y discard  n continue editing",
    };
    frame.render_widget(
        Paragraph::new(keys).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );

    let status_style = match app.status.as_ref().map(|status| status.kind) {
        Some(StatusKind::Error) => Style::default().fg(Color::Red),
        Some(StatusKind::Info) => Style::default().fg(Color::Green),
        None => Style::default().fg(Color::DarkGray),
    };
    let status_text = app
        .status
        .as_ref()
        .map(|status| status.text.clone())
        .unwrap_or_else(|| match app.mode {
            Mode::Filter => format!("Filter: {}", app.filter),
            _ => String::new(),
        });
    frame.render_widget(
        Paragraph::new(status_text)
            .style(status_style)
            .block(Block::default().title("Status").borders(Borders::ALL)),
        chunks[1],
    );
}

fn render_editor(frame: &mut Frame<'_>, area: Rect, editor: &EditorState) {
    let popup = centered_rect(70, 65, area);
    frame.render_widget(Clear, popup);
    let block = Block::default().title("Edit Link").borders(Borders::ALL);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(2),
        ])
        .split(inner);

    render_field(
        frame,
        rows[0],
        "Primary",
        &editor.primary,
        editor.active_field == Field::Primary,
    );
    render_field(
        frame,
        rows[1],
        "URL",
        &editor.url,
        editor.active_field == Field::Url,
    );
    render_field(
        frame,
        rows[2],
        "Aliases",
        &editor.aliases,
        editor.active_field == Field::Aliases,
    );
    render_field(
        frame,
        rows[3],
        "Tags",
        &editor.tags,
        editor.active_field == Field::Tags,
    );
    render_field(
        frame,
        rows[4],
        "Note",
        &editor.note,
        editor.active_field == Field::Note,
    );

    let error = editor
        .error
        .clone()
        .unwrap_or_else(|| "Ctrl-s saves; Esc cancels.".into());
    frame.render_widget(
        Paragraph::new(error)
            .style(if editor.error.is_some() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Gray)
            })
            .alignment(Alignment::Left),
        rows[5],
    );
}

fn render_field(frame: &mut Frame<'_>, area: Rect, title: &str, value: &str, active: bool) {
    let style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(value.to_string())
            .style(style)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_center_message(frame: &mut Frame<'_>, area: Rect, message: &str, color: Color) {
    let popup = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(message)
            .style(Style::default().fg(color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Confirm")),
        popup,
    );
}

fn centered_rect(horizontal: u16, vertical: u16, area: Rect) -> Rect {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - vertical) / 2),
            Constraint::Percentage(vertical),
            Constraint::Percentage((100 - vertical) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - horizontal) / 2),
            Constraint::Percentage(horizontal),
            Constraint::Percentage((100 - horizontal) / 2),
        ])
        .split(outer[1])[1]
}
