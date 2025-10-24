//! A simple markdown renderer widget for Ratatui. by Joshka
//!
//! This module provides a simple markdown renderer widget for Ratatui. It uses the `pulldown-cmark`
//! crate to parse markdown and convert it to a `Text` widget. The `Text` widget can then be
//! rendered to the terminal using the 'Ratatui' library.

use std::sync::LazyLock;
use std::vec;

use ansi_to_tui::IntoText;
use itertools::{Itertools, Position};
use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span, Text};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

pub fn from_str<'a>(input: &'a str) -> Text<'a> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(input, options);
    let mut writer = TextWriter::new(parser);
    writer.run();
    writer.text
}

struct TextWriter<'a, I> {
    /// Iterator supplying events.
    iter: I,

    /// Text to write to.
    text: Text<'a>,

    /// Current style.
    ///
    /// This is a stack of styles, with the top style being the current style.
    inline_styles: Vec<Style>,

    /// Prefix to add to the start of the each line.
    line_prefixes: Vec<Span<'a>>,

    /// Stack of line styles.
    line_styles: Vec<Style>,

    /// Used to highlight code blocks, set when  a codeblock is encountered
    code_highlighter: Option<HighlightLines<'a>>,

    /// Current list index as a stack of indices.
    list_indices: Vec<Option<u64>>,

    /// A link which will be appended to the current line when the link tag is closed.
    link: Option<CowStr<'a>>,

    needs_newline: bool,
}

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

impl<'a, I> TextWriter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    fn new(iter: I) -> Self {
        Self {
            iter,
            text: Text::default(),
            inline_styles: vec![],
            line_styles: vec![],
            line_prefixes: vec![],
            list_indices: vec![],
            needs_newline: false,
            code_highlighter: None,
            link: None,
        }
    }

    fn run(&mut self) {
        while let Some(event) = self.iter.next() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: Event<'a>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.text(text),
            Event::Code(code) => self.code(code),
            Event::SoftBreak => self.soft_break(),
            Event::HardBreak => self.hard_break(),
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: Tag<'a>) {
        match tag {
            Tag::Paragraph => self.start_paragraph(),
            Tag::Heading { level, .. } => self.start_heading(level),
            Tag::BlockQuote(kind) => self.start_blockquote(kind),
            Tag::CodeBlock(kind) => self.start_codeblock(kind),
            Tag::List(start_index) => self.start_list(start_index),
            Tag::Item => self.start_item(),
            Tag::Emphasis => self.push_inline_style(Style::new().italic()),
            Tag::Strong => self.push_inline_style(Style::new().bold()),
            Tag::Strikethrough => self.push_inline_style(Style::new().crossed_out()),
            Tag::Link { dest_url, .. } => self.push_link(dest_url),
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.end_paragraph(),
            TagEnd::Heading(_) => self.end_heading(),
            TagEnd::BlockQuote(_) => self.end_blockquote(),
            TagEnd::CodeBlock => self.end_codeblock(),
            TagEnd::HtmlBlock => {}
            TagEnd::List(_is_ordered) => self.end_list(),
            TagEnd::Item => {}
            TagEnd::FootnoteDefinition => {}
            TagEnd::Table => {}
            TagEnd::TableHead => {}
            TagEnd::TableRow => {}
            TagEnd::TableCell => {}
            TagEnd::Emphasis => self.pop_inline_style(),
            TagEnd::Strong => self.pop_inline_style(),
            TagEnd::Strikethrough => self.pop_inline_style(),
            TagEnd::Subscript => {}
            TagEnd::Superscript => {}
            TagEnd::Link => self.pop_link(),
            TagEnd::Image => {}
            TagEnd::MetadataBlock(_) => {}
            TagEnd::DefinitionList => {}
            TagEnd::DefinitionListTitle => {}
            TagEnd::DefinitionListDefinition => {}
        }
    }

    fn start_paragraph(&mut self) {
        // Insert an empty line between paragraphs if there is at least one line of text already.
        if self.needs_newline {
            self.push_line(Line::default());
        }
        self.push_line(Line::default());
        self.needs_newline = false;
    }

    fn end_paragraph(&mut self) {
        self.needs_newline = true
    }

    fn start_heading(&mut self, level: HeadingLevel) {
        if self.needs_newline {
            self.push_line(Line::default());
        }
        let style = match level {
            HeadingLevel::H1 => styles::H1,
            HeadingLevel::H2 => styles::H2,
            HeadingLevel::H3 => styles::H3,
            HeadingLevel::H4 => styles::H4,
            HeadingLevel::H5 => styles::H5,
            HeadingLevel::H6 => styles::H6,
        };
        let content = format!("{} ", "#".repeat(level as usize));
        self.push_line(Line::styled(content, style));
        self.needs_newline = false;
    }

    fn end_heading(&mut self) {
        self.needs_newline = true
    }

    fn start_blockquote(&mut self, _kind: Option<BlockQuoteKind>) {
        if self.needs_newline {
            self.push_line(Line::default());
            self.needs_newline = false;
        }
        self.line_prefixes.push(Span::from(">"));
        self.line_styles.push(styles::BLOCKQUOTE);
    }

    fn end_blockquote(&mut self) {
        self.line_prefixes.pop();
        self.line_styles.pop();
        self.needs_newline = true;
    }

    fn text(&mut self, text: CowStr<'a>) {
        if let Some(highlighter) = &mut self.code_highlighter {
            let text: Text = LinesWithEndings::from(&text)
                .filter_map(|line| highlighter.highlight_line(line, &SYNTAX_SET).ok())
                .filter_map(|part| as_24_bit_terminal_escaped(&part, false).into_text().ok())
                .flatten()
                .collect();

            for line in text.lines {
                self.text.push_line(line);
            }
            self.needs_newline = false;
            return;
        }

        for (position, line) in text.lines().with_position() {
            if self.needs_newline {
                self.push_line(Line::default());
                self.needs_newline = false;
            }
            if matches!(position, Position::Middle | Position::Last) {
                self.push_line(Line::default());
            }

            let style = self.inline_styles.last().copied().unwrap_or_default();

            let span = Span::styled(line.to_owned(), style);

            self.push_span(span);
        }
        self.needs_newline = false;
    }

    fn code(&mut self, code: CowStr<'a>) {
        let span = Span::styled(code, styles::CODE);
        self.push_span(span);
    }

    fn hard_break(&mut self) {
        self.push_line(Line::default());
    }

    fn start_list(&mut self, index: Option<u64>) {
        if self.list_indices.is_empty() && self.needs_newline {
            self.push_line(Line::default());
        }
        self.list_indices.push(index);
    }

    fn end_list(&mut self) {
        self.list_indices.pop();
        self.needs_newline = true;
    }

    fn start_item(&mut self) {
        self.push_line(Line::default());
        let width = self.list_indices.len() * 4 - 3;
        if let Some(last_index) = self.list_indices.last_mut() {
            let span = match last_index {
                None => Span::from(" ".repeat(width - 1) + "- "),
                Some(index) => {
                    *index += 1;
                    format!("{:width$}. ", *index - 1).light_blue()
                }
            };
            self.push_span(span);
        }
        self.needs_newline = false;
    }

    fn soft_break(&mut self) {
        self.push_line(Line::default());
    }

    fn start_codeblock(&mut self, kind: CodeBlockKind<'_>) {
        if !self.text.lines.is_empty() {
            self.push_line(Line::default());
        }
        let lang = match kind {
            CodeBlockKind::Fenced(ref lang) => lang.as_ref(),
            CodeBlockKind::Indented => "",
        };

        self.line_styles.push(styles::CODE);

        self.set_code_highlighter(lang);

        let span = Span::from(format!("```{lang}"));
        self.push_line(span.into());
        self.needs_newline = true;
    }

    fn end_codeblock(&mut self) {
        let span = Span::from("```");
        self.push_line(span.into());
        self.needs_newline = true;

        self.line_styles.pop();

        self.clear_code_highlighter();
    }

    fn set_code_highlighter(&mut self, lang: &str) {
        if let Some(syntax) = SYNTAX_SET.find_syntax_by_token(lang) {
            let theme = &THEME_SET.themes["base16-ocean.dark"];
            let highlighter = HighlightLines::new(syntax, theme);
            self.code_highlighter = Some(highlighter);
        }
    }

    fn clear_code_highlighter(&mut self) {
        self.code_highlighter = None;
    }

    fn push_inline_style(&mut self, style: Style) {
        let current_style = self.inline_styles.last().copied().unwrap_or_default();
        let style = current_style.patch(style);
        self.inline_styles.push(style);
    }

    fn pop_inline_style(&mut self) {
        self.inline_styles.pop();
    }

    fn push_line(&mut self, line: Line<'a>) {
        let style = self.line_styles.last().copied().unwrap_or_default();
        let mut line = line.patch_style(style);

        // Add line prefixes to the start of the line.
        let line_prefixes = self.line_prefixes.iter().cloned().collect_vec();
        let has_prefixes = !line_prefixes.is_empty();
        if has_prefixes {
            line.spans.insert(0, " ".into());
        }
        for prefix in line_prefixes.iter().rev().cloned() {
            line.spans.insert(0, prefix);
        }
        self.text.lines.push(line);
    }

    fn push_span(&mut self, span: Span<'a>) {
        if let Some(line) = self.text.lines.last_mut() {
            line.push_span(span);
        } else {
            self.push_line(Line::from(vec![span]));
        }
    }

    /// Store the link to be appended to the link text
    fn push_link(&mut self, dest_url: CowStr<'a>) {
        self.link = Some(dest_url);
    }

    /// Append the link to the current line
    fn pop_link(&mut self) {
        if let Some(link) = self.link.take() {
            self.push_span(" (".into());
            self.push_span(Span::styled(link, styles::LINK));
            self.push_span(")".into());
        }
    }
}

mod styles {
    use ratatui::style::{Color, Modifier, Style};

    pub const H1: Style = Style::new()
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::UNDERLINED);
    pub const H2: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    pub const H3: Style = Style::new()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::ITALIC);
    pub const H4: Style = Style::new()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::ITALIC);
    pub const H5: Style = Style::new()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::ITALIC);
    pub const H6: Style = Style::new()
        .fg(Color::LightCyan)
        .add_modifier(Modifier::ITALIC);
    pub const BLOCKQUOTE: Style = Style::new().fg(Color::Green);
    pub const CODE: Style = Style::new().fg(Color::White).bg(Color::Black);
    pub const LINK: Style = Style::new()
        .fg(Color::Blue)
        .add_modifier(Modifier::UNDERLINED);
}
