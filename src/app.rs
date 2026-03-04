use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

use crate::epub_reader::Book;
use crate::progress::{BookProgress, ProgressManager};
use crate::toc::TocState;
use crate::ui;

#[derive(Debug, PartialEq)]
pub enum AppMode {
    Reading,
    Help,
    Library,
}

pub struct App {
    pub book: Option<Book>,
    pub chapter_index: usize,
    pub scroll_offset: usize,
    pub mode: AppMode,
    pub toc: TocState,
    pub progress: ProgressManager,
    pub status_message: Option<String>,
    pub search_query: String,
    pub search_mode: bool,
    pub search_results: Vec<(usize, usize)>,
    pub search_result_index: usize,
    pub line_number_mode: bool,
    pub dark_mode: bool,
    wrapped_cache: Option<(usize, u16, Vec<String>)>,
}

impl App {
    pub fn new(file_path: Option<&str>) -> Result<Self> {
        let progress = ProgressManager::new();

        let (book, chapter_index, scroll_offset) = if let Some(path) = file_path {
            let book = Book::load(path)?;
            let (ci, so) = progress
                .get(path)
                .map(|p| (p.chapter_index, p.scroll_offset))
                .unwrap_or((0, 0));
            (Some(book), ci, so)
        } else {
            (None, 0, 0)
        };

        Ok(Self {
            book,
            chapter_index,
            scroll_offset,
            mode: AppMode::Reading,
            toc: TocState::default(),
            progress,
            status_message: None,
            search_query: String::new(),
            search_mode: false,
            search_results: Vec::new(),
            search_result_index: 0,
            line_number_mode: false,
            dark_mode: true,
            wrapped_cache: None,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.event_loop(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        self.save_progress();

        result
    }

    fn event_loop<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<()> {
        loop {
            terminal.draw(|f| ui::draw(f, self))?;

            if !event::poll(Duration::from_millis(100))? {
                continue;
            }

            if let Event::Key(key) = event::read()? {
                if self.search_mode {
                    self.handle_search_key(key.code);
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(())
                    }

                    KeyCode::Down | KeyCode::Char('j') => self.scroll_down(1),
                    KeyCode::Up | KeyCode::Char('k') => self.scroll_up(1),
                    KeyCode::PageDown | KeyCode::Char('f') => self.page_down(terminal),
                    KeyCode::PageUp | KeyCode::Char('b') => self.page_up(terminal),
                    KeyCode::Char('g') => self.scroll_to_top(),
                    KeyCode::Char('G') => self.scroll_to_bottom(terminal),

                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('n') => {
                        self.next_chapter()
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('p') => {
                        self.prev_chapter()
                    }

                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        if let Some(book) = &self.book {
                            let len = book.chapters.len();
                            self.toc.toggle();
                            self.toc.selected = self.chapter_index.min(len.saturating_sub(1));
                        }
                    }

                    KeyCode::Enter => {
                        if self.toc.visible {
                            self.chapter_index = self.toc.selected;
                            self.scroll_offset = 0;
                            self.wrapped_cache = None;
                            self.toc.visible = false;
                        }
                    }
                    KeyCode::Esc => {
                        self.toc.visible = false;
                        self.mode = AppMode::Reading;
                    }

                    KeyCode::Char('/') => {
                        self.search_mode = true;
                        self.search_query.clear();
                    }
                    KeyCode::Char('N') => self.search_prev(),
                    KeyCode::Char('m') => self.search_next_result(),

                    KeyCode::Char('L') => self.line_number_mode = !self.line_number_mode,
                    KeyCode::Char('D') => self.dark_mode = !self.dark_mode,

                    KeyCode::Char('?') => {
                        self.mode = if self.mode == AppMode::Help {
                            AppMode::Reading
                        } else {
                            AppMode::Help
                        };
                    }

                    _ => {}
                }

                if self.toc.visible {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.toc.move_up();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let Some(book) = &self.book {
                                self.toc.move_down(book.chapters.len());
                            }
                        }
                        _ => {}
                    }
                }
            }

            if let Event::Resize(_, _) = event::read().unwrap_or(Event::FocusLost) {
                self.wrapped_cache = None;
            }
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn page_down<B: ratatui::backend::Backend>(&mut self, terminal: &Terminal<B>) {
        let height = terminal.size().map(|s| s.height as usize).unwrap_or(40);
        self.scroll_down(height.saturating_sub(3));
    }

    pub fn page_up<B: ratatui::backend::Backend>(&mut self, terminal: &Terminal<B>) {
        let height = terminal.size().map(|s| s.height as usize).unwrap_or(40);
        self.scroll_up(height.saturating_sub(3));
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom<B: ratatui::backend::Backend>(&mut self, terminal: &Terminal<B>) {
        let height = terminal.size().map(|s| s.height as usize).unwrap_or(40);
        if let Some(lines) = self.current_wrapped_lines(
            terminal.size().map(|s| s.width).unwrap_or(80),
        ) {
            self.scroll_offset = lines.len().saturating_sub(height);
        }
    }

    pub fn next_chapter(&mut self) {
        if let Some(book) = &self.book {
            if self.chapter_index + 1 < book.chapters.len() {
                self.chapter_index += 1;
                self.scroll_offset = 0;
                self.wrapped_cache = None;
            }
        }
    }

    pub fn prev_chapter(&mut self) {
        if self.chapter_index > 0 {
            self.chapter_index -= 1;
            self.scroll_offset = 0;
            self.wrapped_cache = None;
        }
    }

    pub fn current_wrapped_lines(&mut self, width: u16) -> Option<Vec<String>> {
        let chapter_index = self.chapter_index;

        if let Some((ci, w, ref lines)) = self.wrapped_cache {
            if ci == chapter_index && w == width {
                return Some(lines.clone());
            }
        }

        let content = self
            .book
            .as_ref()?
            .chapters
            .get(chapter_index)?
            .content
            .clone();

        let text_width = (width as usize).saturating_sub(4).max(20);
        let mut wrapped = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                wrapped.push(String::new());
            } else {
                let options = textwrap::Options::new(text_width);
                for wl in textwrap::wrap(line, options) {
                    wrapped.push(wl.into_owned());
                }
            }
        }

        self.wrapped_cache = Some((chapter_index, width, wrapped.clone()));
        Some(wrapped)
    }

    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.search_mode = false;
            }
            KeyCode::Enter => {
                self.search_mode = false;
                self.perform_search();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            _ => {}
        }
    }

    fn perform_search(&mut self) {
        self.search_results.clear();
        self.search_result_index = 0;
        if self.search_query.is_empty() {
            return;
        }
        let query = self.search_query.to_lowercase();
        if let Some(book) = &self.book {
            for (ci, chapter) in book.chapters.iter().enumerate() {
                for (li, line) in chapter.content.lines().enumerate() {
                    if line.to_lowercase().contains(&query) {
                        self.search_results.push((ci, li));
                    }
                }
            }
        }
        if !self.search_results.is_empty() {
            let (ci, li) = self.search_results[0];
            self.chapter_index = ci;
            self.scroll_offset = li;
            self.wrapped_cache = None;
        }
        self.status_message = Some(format!(
            "{} result(s) for '{}'",
            self.search_results.len(),
            self.search_query
        ));
    }

    fn search_next_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_result_index = (self.search_result_index + 1) % self.search_results.len();
        let (ci, li) = self.search_results[self.search_result_index];
        self.chapter_index = ci;
        self.scroll_offset = li;
        self.wrapped_cache = None;
    }

    fn search_prev(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        if self.search_result_index == 0 {
            self.search_result_index = self.search_results.len() - 1;
        } else {
            self.search_result_index -= 1;
        }
        let (ci, li) = self.search_results[self.search_result_index];
        self.chapter_index = ci;
        self.scroll_offset = li;
        self.wrapped_cache = None;
    }

    pub fn save_progress(&mut self) {
        if let Some(book) = &self.book {
            let path = book.path.clone();
            let _ = self.progress.save(
                &path,
                BookProgress {
                    chapter_index: self.chapter_index,
                    scroll_offset: self.scroll_offset,
                },
            );
        }
    }

    pub fn reading_progress_percent(&mut self, width: u16) -> f64 {
        let book = match &self.book {
            Some(b) => b,
            None => return 0.0,
        };
        let total_chapters = book.chapters.len();
        if total_chapters == 0 {
            return 0.0;
        }
        let chapter_frac = self.chapter_index as f64 / total_chapters as f64;
        let lines = self
            .current_wrapped_lines(width)
            .map(|l| l.len())
            .unwrap_or(1);
        let line_frac = if lines > 0 {
            self.scroll_offset as f64 / lines as f64
        } else {
            0.0
        };
        ((chapter_frac + line_frac / total_chapters as f64) * 100.0).min(100.0)
    }
}
