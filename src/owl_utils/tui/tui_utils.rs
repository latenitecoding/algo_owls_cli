use crate::common::{OwlError, Result};
use crate::owl_utils::fs_utils;
use ansi_to_tui::IntoText;
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

pub fn format_content(
    path: &Path,
    content: &str,
    ps: &SyntaxSet,
    ts: &ThemeSet,
    cols: usize,
) -> (String, usize) {
    let content_block = content
        .split('\n')
        .map(|line| {
            if line.len() <= cols {
                return line.to_string();
            }

            let mut buffer = String::new();

            for chunk in line.trim().split_inclusive(' ') {
                if chunk.len() >= cols || (buffer.len() + chunk.len()) % cols <= buffer.len() % cols
                {
                    buffer.push('\n');
                }
                buffer.push_str(chunk);
            }

            buffer
        })
        .collect::<Vec<String>>()
        .join("\n");

    let n: usize = content_block.split('\n').count();

    if path.is_file()
        && let Some(prog_ext) = path.extension().and_then(OsStr::to_str)
        && let Some(syntax) = ps.find_syntax_by_extension(prog_ext)
    {
        let theme = &ts.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        let mut buffer = String::new();
        for line in LinesWithEndings::from(&content_block) {
            if let Ok(ranges) = h.highlight_line(line, ps) {
                buffer.push_str(&syntect::util::as_24_bit_terminal_escaped(&ranges, true));
            } else {
                buffer.push_str(line);
            }
        }

        (buffer, n)
    } else {
        (content_block, n)
    }
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

#[derive(Debug, Default)]
pub struct FileExplorerApp {
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

impl FileExplorerApp {
    pub fn run(mut self, cwd: &Path) -> Result<()> {
        enable_raw_mode()
            .map_err(|e| OwlError::TuiError("Failed to enter raw mode".into(), e.to_string()))?;
        stdout()
            .execute(EnterAlternateScreen)
            .map_err(|e| OwlError::TuiError("Failed to enable alt screen".into(), e.to_string()))?;

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

                    let cols = r_chunks[1].width as usize - 2;

                    let (file_content, num_lines) =
                        match fs_utils::read_contents(file_cursor.path()) {
                            Ok(file_content) => {
                                format_content(file_cursor.path(), &file_content, &ps, &ts, cols)
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

                    let paragraph = if let Ok(text) = file_content.into_text() {
                        Paragraph::new(text)
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .border_type(BorderType::Double),
                            )
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
                        KeyCode::Char('q') => break,
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

        disable_raw_mode()
            .map_err(|e| OwlError::TuiError("Failed to disable raw mode".into(), e.to_string()))?;
        stdout()
            .execute(LeaveAlternateScreen)
            .map_err(|e| OwlError::TuiError("Failed to leave alt screen".into(), e.to_string()))?;

        Ok(())
    }
}
