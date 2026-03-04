use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode};

struct Palette {
    bg: Color,
    fg: Color,
    accent: Color,
    muted: Color,
    highlight_bg: Color,
    border: Color,
    title: Color,
}

impl Palette {
    fn dark() -> Self {
        Self {
            bg: Color::Rgb(18, 18, 24),
            fg: Color::Rgb(220, 215, 200),
            accent: Color::Rgb(130, 190, 255),
            muted: Color::Rgb(110, 110, 130),
            highlight_bg: Color::Rgb(40, 50, 70),
            border: Color::Rgb(60, 60, 80),
            title: Color::Rgb(255, 200, 100),
        }
    }

    fn light() -> Self {
        Self {
            bg: Color::Rgb(248, 244, 232),
            fg: Color::Rgb(40, 35, 30),
            accent: Color::Rgb(30, 100, 200),
            muted: Color::Rgb(140, 130, 120),
            highlight_bg: Color::Rgb(200, 220, 255),
            border: Color::Rgb(190, 185, 170),
            title: Color::Rgb(160, 80, 20),
        }
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let pal = if app.dark_mode {
        Palette::dark()
    } else {
        Palette::light()
    };

    let full = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(pal.bg)),
        full,
    );

    if app.book.is_none() {
        draw_welcome(f, full, &pal);
        return;
    }

    match app.mode {
        AppMode::Help => draw_help(f, full, &pal),
        _ => {
            draw_reader(f, app, full, &pal);
            if app.toc.visible {
                draw_toc(f, app, full, &pal);
            }
            if app.search_mode {
                draw_search_bar(f, app, full, &pal);
            } else if let Some(ref msg) = app.status_message.clone() {
                draw_status_bar(f, full, msg, &pal);
            }
        }
    }
}

fn draw_welcome(f: &mut Frame, area: Rect, pal: &Palette) {
    let text = vec![
        Line::from(vec![Span::styled(
            " 📖  ebook-reader",
            Style::default().fg(pal.title).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(Span::styled(
            "Usage:  ebook-reader <path/to/book.epub>",
            Style::default().fg(pal.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? for help once a book is loaded.",
            Style::default().fg(pal.muted),
        )),
    ];

    let para = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.border))
                .title(" Welcome "),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });

    let centered = centered_rect(60, 40, area);
    f.render_widget(para, centered);
}

fn draw_reader(f: &mut Frame, app: &mut App, area: Rect, pal: &Palette) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    draw_header(f, app, chunks[0], pal);
    draw_body(f, app, chunks[1], pal);
    draw_footer(f, app, chunks[2], pal);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect, pal: &Palette) {
    let book = match &app.book {
        Some(b) => b,
        None => return,
    };

    let chapter_title = book
        .chapters
        .get(app.chapter_index)
        .map(|c| c.title.clone())
        .unwrap_or_default();

    let title_line = Line::from(vec![
        Span::styled(
            &book.title,
            Style::default().fg(pal.title).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  —  ", Style::default().fg(pal.muted)),
        Span::styled(&book.author, Style::default().fg(pal.muted)),
    ]);

    let chapter_line = Line::from(Span::styled(
        format!(
            "§ {} / {}  ·  {}",
            app.chapter_index + 1,
            book.chapters.len(),
            chapter_title
        ),
        Style::default().fg(pal.accent),
    ));

    let header = Paragraph::new(vec![title_line, chapter_line])
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(pal.border)),
        )
        .style(Style::default().bg(pal.bg));

    f.render_widget(header, area);
}

fn draw_body(f: &mut Frame, app: &mut App, area: Rect, pal: &Palette) {
    let width = area.width;
    let height = area.height as usize;

    let lines = match app.current_wrapped_lines(width) {
        Some(l) => l,
        None => vec!["No content".to_string()],
    };

    let max_scroll = lines.len().saturating_sub(height);
    if app.scroll_offset > max_scroll {
        app.scroll_offset = max_scroll;
    }

    let query = app.search_query.clone();
    let line_number_mode = app.line_number_mode;
    let fg = pal.fg;

    let visible: Vec<Line> = lines
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(height)
        .map(|(idx, line)| {
            let line_no = if line_number_mode {
                format!("{:4} │ ", idx + 1)
            } else {
                String::new()
            };

            let full_line = format!("{}{}", line_no, line);

            if !query.is_empty()
                && full_line.to_lowercase().contains(&query.to_lowercase())
            {
                // Build owned spans to avoid borrowing full_line
                let mut spans: Vec<Span> = Vec::new();
                let lower = full_line.to_lowercase();
                let lower_q = query.to_lowercase();
                let mut last = 0;
                let mut i = 0;
                while i + lower_q.len() <= full_line.len() {
                    if lower[i..].starts_with(&lower_q) {
                        if i > last {
                            spans.push(Span::styled(
                                full_line[last..i].to_string(),
                                Style::default().fg(fg),
                            ));
                        }
                        spans.push(Span::styled(
                            full_line[i..i + query.len()].to_string(),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ));
                        last = i + query.len();
                        i = last;
                    } else {
                        i += 1;
                    }
                }
                if last < full_line.len() {
                    spans.push(Span::styled(
                        full_line[last..].to_string(),
                        Style::default().fg(fg),
                    ));
                }
                Line::from(spans)
            } else {
                Line::from(Span::styled(full_line, Style::default().fg(fg)))
            }
        })
        .collect();

    let body = Paragraph::new(visible)
        .block(Block::default().style(Style::default().bg(pal.bg)))
        .style(Style::default().bg(pal.bg).fg(pal.fg));

    f.render_widget(body, area);
}

fn highlight_matches<'a>(line: &'a str, query: &str, pal: &Palette) -> Vec<Span<'a>> {
    let lower_line = line.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut spans = Vec::new();
    let mut last = 0;

    let mut i = 0;
    while i + lower_query.len() <= line.len() {
        if lower_line[i..].starts_with(&lower_query) {
            if i > last {
                spans.push(Span::styled(&line[last..i], Style::default().fg(pal.fg)));
            }
            spans.push(Span::styled(
                &line[i..i + query.len()],
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            last = i + query.len();
            i = last;
        } else {
            i += 1;
        }
    }
    if last < line.len() {
        spans.push(Span::styled(&line[last..], Style::default().fg(pal.fg)));
    }
    spans
}

fn draw_footer(f: &mut Frame, app: &mut App, area: Rect, pal: &Palette) {
    let width = area.width;
    let percent = app.reading_progress_percent(width);

    let label = format!(
        " {:.1}%  ·  j/k scroll  ·  n/p chapter  ·  t TOC  ·  / search  ·  ? help  ·  q quit ",
        percent
    );

    let gauge = Gauge::default()
        .block(Block::default().style(Style::default().bg(pal.bg)))
        .gauge_style(Style::default().fg(pal.accent).bg(pal.border))
        .ratio(percent / 100.0)
        .label(Span::styled(label, Style::default().fg(pal.muted)));

    f.render_widget(gauge, area);
}

fn draw_toc(f: &mut Frame, app: &App, area: Rect, pal: &Palette) {
    let book = match &app.book {
        Some(b) => b,
        None => return,
    };

    let popup = centered_rect(60, 70, area);
    f.render_widget(Clear, popup);

    let items: Vec<ListItem> = book
        .chapters
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let marker = if i == app.chapter_index { "▶ " } else { "  " };
            ListItem::new(format!("{}{}", marker, ch.title))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.toc.selected));

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Table of Contents  [Enter] jump · [Esc] close ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.accent)),
        )
        .style(Style::default().bg(pal.bg).fg(pal.fg))
        .highlight_style(
            Style::default()
                .bg(pal.highlight_bg)
                .fg(pal.accent)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, popup, &mut state);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect, pal: &Palette) {
    let bar_rect = Rect {
        x: area.x,
        y: area.height.saturating_sub(3),
        width: area.width,
        height: 3,
    };
    f.render_widget(Clear, bar_rect);

    let para = Paragraph::new(format!(" /{}", app.search_query))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.accent))
                .title(" Search  [Enter] confirm · [Esc] cancel "),
        )
        .style(Style::default().bg(pal.bg).fg(pal.fg));

    f.render_widget(para, bar_rect);
}

fn draw_status_bar(f: &mut Frame, area: Rect, msg: &str, pal: &Palette) {
    let bar_rect = Rect {
        x: area.x,
        y: area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    let para = Paragraph::new(format!(" {}", msg))
        .style(Style::default().bg(pal.highlight_bg).fg(pal.accent));
    f.render_widget(para, bar_rect);
}

fn draw_help(f: &mut Frame, area: Rect, pal: &Palette) {
    let popup = centered_rect(65, 80, area);
    f.render_widget(Clear, popup);

    let rows: Vec<(&str, &str)> = vec![
        ("j / ↓",       "Scroll down one line"),
        ("k / ↑",       "Scroll up one line"),
        ("f / PgDn",    "Page down"),
        ("b / PgUp",    "Page up"),
        ("g",           "Jump to top of chapter"),
        ("G",           "Jump to bottom of chapter"),
        ("n / →",       "Next chapter"),
        ("p / ←",       "Previous chapter"),
        ("t",           "Toggle Table of Contents"),
        ("/",           "Search within book"),
        ("m",           "Next search result"),
        ("N",           "Previous search result"),
        ("L",           "Toggle line numbers"),
        ("D",           "Toggle dark/light mode"),
        ("?",           "Toggle this help"),
        ("q / Ctrl-C",  "Quit  (progress saved automatically)"),
    ];

    let lines: Vec<Line> = std::iter::once(Line::from(vec![
        Span::styled(
            "  Key",
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw("                  "),
        Span::styled(
            "Action",
            Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
        ),
    ]))
    .chain(std::iter::once(Line::from(Span::styled(
        "─".repeat(50),
        Style::default().fg(pal.border),
    ))))
    .chain(rows.iter().map(|(key, desc)| {
        Line::from(vec![
            Span::styled(
                format!("  {:18}", key),
                Style::default().fg(pal.title).add_modifier(Modifier::BOLD),
            ),
            Span::styled(*desc, Style::default().fg(pal.fg)),
        ])
    }))
    .chain(std::iter::once(Line::from("")))
    .chain(std::iter::once(Line::from(Span::styled(
        "  Reading progress is saved on exit.",
        Style::default().fg(pal.muted),
    ))))
    .collect();

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Keyboard Shortcuts  [?] or [Esc] close ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(pal.accent)),
        )
        .style(Style::default().bg(pal.bg))
        .wrap(Wrap { trim: false });

    f.render_widget(para, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
