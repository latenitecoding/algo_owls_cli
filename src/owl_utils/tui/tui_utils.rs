use crate::common::{OwlError, Result};
use crate::owl_utils::{PromptMode, fs_utils, llm_utils};
use ansi_to_tui::IntoText;
use anthropic_sdk::Anthropic;
use crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    crossterm,
    crossterm::event::{Event, KeyCode, read},
    prelude::*,
    widgets::*,
};
use ratatui_explorer::{FileExplorer, Theme};
use std::ffi::OsStr;
use std::io::stdout;
use std::path::Path;
use std::time::{Duration, Instant};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use tui_textarea::TextArea;

pub fn enter_raw_mode() -> Result<()> {
    enable_raw_mode()
        .map_err(|e| OwlError::TuiError("Failed to enter raw mode".into(), e.to_string()))?;
    stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| OwlError::TuiError("Failed to enable alt screen".into(), e.to_string()))?;

    Ok(())
}

pub fn exit_raw_mode() -> Result<()> {
    disable_raw_mode()
        .map_err(|e| OwlError::TuiError("Failed to disable raw mode".into(), e.to_string()))?;
    stdout()
        .execute(LeaveAlternateScreen)
        .map_err(|e| OwlError::TuiError("Failed to leave alt screen".into(), e.to_string()))?;

    Ok(())
}

pub fn get_tui_theme() -> Theme {
    Theme::default()
        .with_block(Block::default().borders(Borders::ALL))
        .with_dir_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .with_highlight_dir_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .with_scroll_padding(1)
}

pub fn highlight_content(path: &Path, content: String, ps: &SyntaxSet, ts: &ThemeSet) -> String {
    if path.is_file()
        && let Some(prog_ext) = path.extension().and_then(OsStr::to_str)
        && prog_ext != "md"
        && let Some(syntax) = ps.find_syntax_by_extension(prog_ext)
    {
        let theme = &ts.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        let mut buffer = String::new();
        for line in LinesWithEndings::from(&content) {
            if let Ok(ranges) = h.highlight_line(line, ps) {
                buffer.push_str(&syntect::util::as_24_bit_terminal_escaped(&ranges, true));
            } else {
                buffer.push_str(line);
            }
        }

        buffer
    } else {
        content
    }
}

#[derive(Debug, Default)]
pub struct FileApp {
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

impl FileApp {
    pub fn run(mut self, path: &Path) -> Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))
            .map_err(|e| OwlError::TuiError("Failed to setup terminal".into(), e.to_string()))?;

        let layout = Layout::vertical([
            Constraint::Min(1),
            Constraint::Percentage(100),
            Constraint::Min(1),
        ]);

        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();

        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        loop {
            terminal
                .draw(|f| {
                    let chunks = layout.split(f.area());

                    let (file_content, num_lines) = match fs_utils::read_contents(path) {
                        Ok(file_content) => {
                            // let (content, n) = format_content(&file_content, cols);
                            let content = highlight_content(path, file_content, &ps, &ts);
                            let n = content.split('\n').count();

                            (content, n)
                        }
                        _ => ("Failed to load file.".into(), 1),
                    };

                    self.vertical_scroll_state =
                        self.vertical_scroll_state.content_length(num_lines);

                    let filename = path
                        .to_str()
                        .map(|s| s.to_string())
                        .unwrap_or(path.to_string_lossy().to_string());

                    let title = Block::new()
                        .title_alignment(Alignment::Center)
                        .title(filename.italic());
                    f.render_widget(title, chunks[0]);

                    let paragraph = if let Some(ext) = path.extension().and_then(OsStr::to_str)
                        && ext == "md"
                    {
                        Paragraph::new(tui_markdown::from_str(&file_content))
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_type(BorderType::Double),
                            )
                            .wrap(Wrap { trim: true })
                            .scroll((self.vertical_scroll as u16, 0))
                    } else if let Ok(text) = file_content.into_text() {
                        Paragraph::new(text)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_type(BorderType::Double),
                            )
                            .wrap(Wrap { trim: true })
                            .scroll((self.vertical_scroll as u16, 0))
                    } else {
                        Paragraph::new(file_content)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_type(BorderType::Double),
                            )
                            .scroll((self.vertical_scroll as u16, 0))
                    };

                    f.render_widget(Clear, chunks[1]);
                    f.render_widget(paragraph, chunks[1]);
                    f.render_stateful_widget(
                        Scrollbar::new(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(Some("↑"))
                            .end_symbol(Some("↓")),
                        chunks[1],
                        &mut self.vertical_scroll_state,
                    );

                    let helpbar = Block::new()
                        .title_alignment(Alignment::Center)
                        .title("Use ▲ ▼ to scroll ".bold());
                    f.render_widget(helpbar, chunks[2]);
                })
                .map_err(|e| OwlError::TuiError("Failed to draw frame".into(), e.to_string()))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if crossterm::event::poll(timeout).map_err(|e| {
                OwlError::TuiError("Failed to compute timeout".into(), e.to_string())
            })? {
                let event = read().map_err(|e| {
                    OwlError::TuiError("Failed to read event".into(), e.to_string())
                })?;

                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Down => {
                            self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::Up => {
                            self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        _ => {
                            self.vertical_scroll = 0;
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                    };
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct FileExplorerApp {
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

impl FileExplorerApp {
    pub fn run(mut self, cwd: &Path) -> Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))
            .map_err(|e| OwlError::TuiError("Failed to setup terminal".into(), e.to_string()))?;

        let layout = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]);

        let theme = get_tui_theme();

        let mut file_explorer = FileExplorer::with_theme(theme).map_err(|e| {
            OwlError::TuiError("Failed to start file explorer".into(), e.to_string())
        })?;

        file_explorer.set_cwd(cwd).map_err(|e| {
            OwlError::TuiError(
                "Failed to change current working directory".into(),
                e.to_string(),
            )
        })?;

        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();

        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        loop {
            let file_cursor = file_explorer.current();

            terminal
                .draw(|f| {
                    let h_chunks = layout.split(f.area());
                    let l_chunks =
                        Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                            .split(h_chunks[0]);
                    let r_chunks =
                        Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)])
                            .split(h_chunks[1]);

                    let (file_content, num_lines) =
                        match fs_utils::read_contents(file_cursor.path()) {
                            Ok(file_content) => {
                                // let (content, n) = format_content(&file_content, cols);
                                let content =
                                    highlight_content(file_cursor.path(), file_content, &ps, &ts);
                                let n = content.split('\n').count();

                                (content, n)
                            }
                            _ => ("Failed to load file.".into(), 1),
                        };

                    self.vertical_scroll_state =
                        self.vertical_scroll_state.content_length(num_lines);

                    f.render_widget(&file_explorer.widget(), l_chunks[0]);

                    let l_helpbar = Block::new()
                        .title_alignment(Alignment::Center)
                        .title("Use h j k l to scroll ".bold());
                    f.render_widget(l_helpbar, l_chunks[1]);

                    let paragraph = if let Some(ext) =
                        file_cursor.path().extension().and_then(OsStr::to_str)
                        && ext == "md"
                    {
                        Paragraph::new(tui_markdown::from_str(&file_content))
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_type(BorderType::Double),
                            )
                            .wrap(Wrap { trim: true })
                            .scroll((self.vertical_scroll as u16, 0))
                    } else if let Ok(text) = file_content.into_text() {
                        Paragraph::new(text)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_type(BorderType::Double),
                            )
                            .wrap(Wrap { trim: true })
                            .scroll((self.vertical_scroll as u16, 0))
                    } else {
                        Paragraph::new(file_content)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_type(BorderType::Double),
                            )
                            .scroll((self.vertical_scroll as u16, 0))
                    };

                    f.render_widget(Clear, r_chunks[0]);
                    f.render_widget(paragraph, r_chunks[0]);
                    f.render_stateful_widget(
                        Scrollbar::new(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(Some("↑"))
                            .end_symbol(Some("↓")),
                        r_chunks[0],
                        &mut self.vertical_scroll_state,
                    );

                    let r_helpbar = Block::new()
                        .title_alignment(Alignment::Center)
                        .title("Use ▲ ▼ to scroll ".bold());
                    f.render_widget(r_helpbar, r_chunks[1]);
                })
                .map_err(|e| OwlError::TuiError("Failed to draw frame".into(), e.to_string()))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if crossterm::event::poll(timeout).map_err(|e| {
                OwlError::TuiError("Failed to compute timeout".into(), e.to_string())
            })? {
                let event = read().map_err(|e| {
                    OwlError::TuiError("Failed to read event".into(), e.to_string())
                })?;

                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Down => {
                            self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::Up => {
                            self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        _ => {
                            self.vertical_scroll = 0;
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);

                            file_explorer.handle(&event).map_err(|e| {
                                OwlError::TuiError(
                                    "Failed to handle key event".into(),
                                    e.to_string(),
                                )
                            })?;
                        }
                    };
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct LlmApp {
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

impl LlmApp {
    pub fn draw(
        &mut self,
        ai_sdk: &str,
        layout: &Layout,
        markdown_content: &[String],
        textarea: &TextArea,
        f: &mut Frame,
    ) {
        let chunks = layout.split(f.area());

        let markdown_str = markdown_content.join("\n");
        let markdown_text = tui_markdown::from_str(&markdown_str);

        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(markdown_content.len());

        let title = Block::new()
            .title_alignment(Alignment::Center)
            .title(ai_sdk.bold());
        f.render_widget(title, chunks[0]);

        f.render_widget(Clear, chunks[1]);
        f.render_widget(
            Paragraph::new(markdown_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double),
                )
                .wrap(Wrap { trim: true })
                .scroll((self.vertical_scroll as u16, 0)),
            chunks[1],
        );
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            chunks[1],
            &mut self.vertical_scroll_state,
        );

        let helpbar = Block::new()
            .title_alignment(Alignment::Center)
            .title("Use ▲ ▼ to scroll ".bold());
        f.render_widget(helpbar, chunks[2]);

        f.render_widget(textarea, chunks[3]);
    }

    pub async fn run(
        mut self,
        ai_sdk: &str,
        client: &Anthropic,
        check_prog: Option<&str>,
        check_prompt: Option<&str>,
        mode: PromptMode,
    ) -> Result<String> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))
            .map_err(|e| OwlError::TuiError("Failed to setup terminal".into(), e.to_string()))?;

        let layout = Layout::vertical([
            Constraint::Min(1),
            Constraint::Percentage(75),
            Constraint::Min(1),
            Constraint::Percentage(25),
        ]);

        let tick_rate = Duration::from_millis(250);
        let mut last_tick = Instant::now();

        let mut textarea = TextArea::default();
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double),
        );

        let mut markdown_content = vec![format!("**>>> {}**: Thinking...\n", ai_sdk)];

        terminal
            .draw(|f| self.draw(ai_sdk, &layout, &markdown_content, &textarea, f))
            .map_err(|e| OwlError::TuiError("Failed to draw frame".into(), e.to_string()))?;

        llm_utils::llm_review_with_client(ai_sdk, client, check_prog, check_prompt, mode)
            .await?
            .split('\n')
            .for_each(|line| markdown_content.push(line.to_string()));

        let mut user_has_reply = false;

        loop {
            terminal
                .draw(|f| self.draw(ai_sdk, &layout, &markdown_content, &textarea, f))
                .map_err(|e| OwlError::TuiError("Failed to draw frame".into(), e.to_string()))?;

            if user_has_reply {
                llm_utils::llm_reply_with_client(
                    ai_sdk,
                    client,
                    &textarea.yank_text(),
                    markdown_content.join("\n").trim(),
                )
                .await?
                .split('\n')
                .for_each(|line| markdown_content.push(line.to_string()));

                user_has_reply = false;

                terminal
                    .draw(|f| self.draw(ai_sdk, &layout, &markdown_content, &textarea, f))
                    .map_err(|e| {
                        OwlError::TuiError("Failed to draw frame".into(), e.to_string())
                    })?;
            }

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());

            if crossterm::event::poll(timeout).map_err(|e| {
                OwlError::TuiError("Failed to compute timeout".into(), e.to_string())
            })? {
                let event = read().map_err(|e| {
                    OwlError::TuiError("Failed to read event".into(), e.to_string())
                })?;

                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Down => {
                            self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::Up => {
                            self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                            self.vertical_scroll_state =
                                self.vertical_scroll_state.position(self.vertical_scroll);
                        }
                        KeyCode::Enter => {
                            self.vertical_scroll = markdown_content.len();
                            self.vertical_scroll_state.last();

                            markdown_content.push("\n**>>> user**:".to_string());

                            textarea.select_all();
                            textarea.cut();

                            textarea
                                .yank_text()
                                .split('\n')
                                .for_each(|line| markdown_content.push(line.to_string()));

                            markdown_content.push(format!("\n**>>> {}**: Thinking...\n", ai_sdk));

                            user_has_reply = true;
                        }
                        _ => {
                            textarea.input(key);
                        }
                    };
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        Ok(markdown_content.join("\n"))
    }
}
