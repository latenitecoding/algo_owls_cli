use crate::common::{OwlError, Result};
use ansi_to_tui::IntoText as _;
use crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    crossterm::event::{Event, KeyCode, read},
    prelude::*,
    widgets::*,
};
use ratatui_explorer::{FileExplorer, Theme};
use std::ffi::OsStr;
use std::fs;
use std::io::stdout;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub fn tui_file_explorer(stash_dir: &Path) -> Result<()> {
    enable_raw_mode()
        .map_err(|e| OwlError::TuiError("Failed to enter raw mode".into(), e.to_string()))?;
    stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| OwlError::TuiError("Failed to enable alt screen".into(), e.to_string()))?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))
        .map_err(|e| OwlError::TuiError("Failed to setup terminal".into(), e.to_string()))?;
    let layout = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]);

    let theme = Theme::default()
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
        .with_scroll_padding(1);

    let mut file_explorer = FileExplorer::with_theme(theme)
        .map_err(|e| OwlError::TuiError("Failed to start file explorer".into(), e.to_string()))?;
    file_explorer.set_cwd(stash_dir).map_err(|e| {
        OwlError::TuiError(
            "Failed to change current working directory".into(),
            e.to_string(),
        )
    })?;

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let mut skip_lines = 0;

    loop {
        let file_cursor = file_explorer.current();
        let file_content = if file_cursor.is_file() {
            fs::read_to_string(file_cursor.path()).map_err(|e| {
                OwlError::FileError(
                    format!(
                        "Failed to read from '{}'",
                        file_cursor.path().to_string_lossy()
                    ),
                    e.to_string(),
                )
            })
        } else if file_cursor.is_dir() {
            fs::read_dir(file_cursor.path())
                .map(|dir_read| {
                    dir_read
                        .into_iter()
                        .map(|try_entry| match try_entry {
                            Ok(dir_entry) => {
                                let dir_name = dir_entry
                                    .file_name()
                                    .into_string()
                                    .unwrap_or_else(|e| e.to_string_lossy().to_string());
                                if let Ok(ft) = dir_entry.file_type()
                                    && ft.is_dir()
                                {
                                    format!("{}/", dir_name)
                                } else {
                                    dir_name
                                }
                            }
                            Err(_) => "<could not read entry name>".into(),
                        })
                        .collect::<Vec<String>>()
                        .join("\n")
                })
                .map_err(|e| {
                    OwlError::FileError(
                        format!(
                            "Failed to read entries in dir '{}'",
                            file_cursor.path().to_string_lossy()
                        ),
                        e.to_string(),
                    )
                })
        } else {
            Ok("<not a regular file>".into())
        };

        let file_content = match file_content {
            Ok(file_content) => file_content,
            _ => "Failed to load file.".into(),
        };

        terminal
            .draw(|f| {
                let chunks = layout.split(f.area());

                let rows = chunks[1].height as usize - 1;
                let cols = chunks[1].width as usize - 1;

                let file_lines = file_content
                    .split('\n')
                    .skip(skip_lines)
                    .take(rows)
                    .map(|line| {
                        if line.len() <= cols {
                            return line.trim().to_string();
                        }

                        let mut buffer = String::new();

                        for chunk in line.trim().split(' ') {
                            if chunk.len() >= cols
                                || (buffer.len() + chunk.len()) % cols <= buffer.len() % cols
                            {
                                buffer.push('\n');
                            } else if !buffer.is_empty() {
                                buffer.push(' ');
                            }
                            buffer.push_str(chunk);
                        }

                        buffer
                    })
                    .collect::<Vec<String>>();

                if file_lines.len() + 1 < rows && skip_lines > 0 {
                    skip_lines -= 1;
                }

                let file_content = file_lines.join("\n");

                let try_text = if file_cursor.is_file()
                    && let Some(prog_ext) = file_cursor.path().extension().and_then(OsStr::to_str)
                    && let Some(syntax) = ps.find_syntax_by_extension(prog_ext)
                {
                    let theme = &ts.themes["base16-ocean.dark"];
                    let mut h = HighlightLines::new(syntax, theme);

                    let mut buffer = String::new();
                    for line in LinesWithEndings::from(&file_content) {
                        if let Ok(ranges) = h.highlight_line(line, &ps) {
                            let ansi_str = syntect::util::as_24_bit_terminal_escaped(&ranges, true);
                            buffer.push_str(&ansi_str);
                        }
                    }

                    buffer.into_text()
                } else {
                    file_content.into_text()
                };

                f.render_widget(&file_explorer.widget(), chunks[0]);
                f.render_widget(Clear, chunks[1]);
                f.render_widget(
                    match try_text {
                        Ok(text) => Paragraph::new(text).block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_type(BorderType::Double),
                        ),
                        Err(_) => Paragraph::new(file_content).block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_type(BorderType::Double),
                        ),
                    },
                    chunks[1],
                );
            })
            .map_err(|e| OwlError::TuiError("Failed to draw frame".into(), e.to_string()))?;

        let event =
            read().map_err(|e| OwlError::TuiError("Failed to read event".into(), e.to_string()))?;

        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('f') => skip_lines += 1,
                KeyCode::Char('e') => skip_lines = if skip_lines > 0 { skip_lines - 1 } else { 0 },
                _ => skip_lines = 0,
            };
        }

        file_explorer
            .handle(&event)
            .map_err(|e| OwlError::TuiError("Failed to handle key event".into(), e.to_string()))?;
    }

    disable_raw_mode()
        .map_err(|e| OwlError::TuiError("Failed to disable raw mode".into(), e.to_string()))?;
    stdout()
        .execute(LeaveAlternateScreen)
        .map_err(|e| OwlError::TuiError("Failed to leave alt screen".into(), e.to_string()))?;

    Ok(())
}
