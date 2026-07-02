use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::config::Theme;

#[derive(Clone, Debug)]
pub struct PreviewCopyTarget {
    pub row: usize,
    pub x_start: u16,
    pub x_end: u16,
    pub text: String,
}

pub struct RenderedMarkdown {
    pub lines: Vec<Line<'static>>,
    pub copy_targets: Vec<PreviewCopyTarget>,
}

pub struct MarkdownPreview<'a> {
    pub content: &'a str,
    pub scroll: u16,
    pub theme: &'a Theme,
}

impl<'a> Widget for MarkdownPreview<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rendered = render_markdown_with_targets(self.content, area.width as usize, self.theme);
        let bg = Theme::parse_color(&self.theme.bg);

        let lines = rendered.lines;
        let start = self.scroll as usize;
        let visible = area.height as usize;

        for (row, line) in lines.iter().skip(start).take(visible).enumerate() {
            buf.set_line(area.x, area.y + row as u16, line, area.width);
        }
        let rendered = lines.len().saturating_sub(start).min(visible);
        for row in rendered..visible {
            for x in area.x..area.right() {
                buf[(x, area.y + row as u16)]
                    .set_char(' ')
                    .set_style(Style::default().bg(bg));
            }
        }
    }
}

pub fn render_markdown_with_targets(content: &str, width: usize, theme: &Theme) -> RenderedMarkdown {
    let fg = Theme::parse_color(&theme.fg);
    let header_color = Theme::parse_color(&theme.header);
    let link_color = Theme::parse_color(&theme.link);
    let accent = Theme::parse_color(&theme.accent);

    let raw = render_markdown(content, fg, header_color, link_color, accent);
    wrap_lines(raw.lines, raw.copy_targets, width)
}

struct RenderCtx {
    lines: Vec<Line<'static>>,
    copy_targets: Vec<PreviewCopyTarget>,
    spans: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    list_depth: usize,
    list_ordered: Vec<bool>,
    ordered_counter: Vec<u64>,
    in_code_block: bool,
    code_block_header_line: Option<usize>,
    code_block_copy_end: Option<u16>,
    code_block_text: String,
    heading_level: Option<HeadingLevel>,
    fg: Color,
}

impl RenderCtx {
    fn new(fg: Color) -> Self {
        Self {
            lines: Vec::new(),
            copy_targets: Vec::new(),
            spans: Vec::new(),
            style_stack: vec![Style::default().fg(fg)],
            list_depth: 0,
            list_ordered: Vec::new(),
            ordered_counter: Vec::new(),
            in_code_block: false,
            code_block_header_line: None,
            code_block_copy_end: None,
            code_block_text: String::new(),
            heading_level: None,
            fg,
        }
    }

    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_else(|| Style::default().fg(self.fg))
    }

    fn push_style(&mut self, s: Style) {
        self.style_stack.push(s);
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn flush_line(&mut self) {
        if !self.spans.is_empty() {
            self.lines.push(Line::from(std::mem::take(&mut self.spans)));
        }
    }

    fn flush_line_always(&mut self) {
        self.lines.push(Line::from(std::mem::take(&mut self.spans)));
    }

    fn blank(&mut self) {
        // avoid double blanks
        let already_blank = self.lines.last().map(|l| l.spans.is_empty()).unwrap_or(false);
        if !already_blank {
            self.lines.push(Line::from(""));
        }
    }

    fn in_list(&self) -> bool {
        self.list_depth > 0
    }

    fn bullet_prefix(&self) -> String {
        let indent = "  ".repeat(self.list_depth.saturating_sub(1));
        let is_ordered = self.list_ordered.last().copied().unwrap_or(false);
        if is_ordered {
            let n = self.ordered_counter.last().copied().unwrap_or(1);
            format!("{}{}. ", indent, n)
        } else {
            let bullet = match self.list_depth {
                1 => "• ",
                2 => "◦ ",
                _ => "▪ ",
            };
            format!("{}{}", indent, bullet)
        }
    }
}

fn heading_line(text: &str, level: HeadingLevel, color: Color) -> Line<'static> {
    let text = text.trim().to_string();
    match level {
        HeadingLevel::H1 => Line::from(vec![
            Span::styled(
                "# ",
                Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::DIM),
            ),
            Span::styled(
                text,
                Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ),
        ]),
        HeadingLevel::H2 => Line::from(vec![
            Span::styled(
                "## ",
                Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::DIM),
            ),
            Span::styled(text, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]),
        HeadingLevel::H3 => Line::from(vec![
            Span::styled(
                "### ",
                Style::default().fg(color).add_modifier(Modifier::DIM),
            ),
            Span::styled(
                text,
                Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ),
        ]),
        HeadingLevel::H4 => Line::from(vec![
            Span::styled("#### ", Style::default().fg(color).add_modifier(Modifier::DIM)),
            Span::styled(text, Style::default().fg(color).add_modifier(Modifier::ITALIC)),
        ]),
        _ => Line::from(Span::styled(text, Style::default().fg(color))),
    }
}

fn render_markdown(
    content: &str,
    fg: Color,
    header_color: Color,
    link_color: Color,
    accent: Color,
) -> RenderedMarkdown {
    let mut ctx = RenderCtx::new(fg);
    let opts = Options::all();
    let parser = Parser::new_ext(content, opts);

    for event in parser {
        match event {
            // ── Headings ───────────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                ctx.flush_line();
                ctx.blank();
                ctx.heading_level = Some(level);
                ctx.push_style(Style::default().fg(header_color).add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Heading(level)) => {
                ctx.pop_style();
                let spans = std::mem::take(&mut ctx.spans);
                let raw_text: String = spans.iter().map(|s| s.content.as_ref()).collect();
                ctx.lines.push(heading_line(&raw_text, level, header_color));
                match level {
                    HeadingLevel::H1 => {
                        ctx.lines.push(Line::from(Span::styled(
                            "═".repeat(48),
                            Style::default().fg(header_color).add_modifier(Modifier::DIM),
                        )));
                    }
                    HeadingLevel::H2 => {
                        ctx.lines.push(Line::from(Span::styled(
                            "─".repeat(36),
                            Style::default().fg(header_color).add_modifier(Modifier::DIM),
                        )));
                    }
                    _ => {}
                }
                ctx.blank();
                ctx.heading_level = None;
            }

            // ── Paragraphs ─────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                ctx.flush_line();
                if !ctx.in_list() {
                    ctx.blank();
                }
            }

            // ── Bold / Italic / Strikethrough ──────────────────────────────
            Event::Start(Tag::Strong) => {
                ctx.push_style(ctx.current_style().add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Strong) => ctx.pop_style(),

            Event::Start(Tag::Emphasis) => {
                ctx.push_style(ctx.current_style().add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => ctx.pop_style(),

            Event::Start(Tag::Strikethrough) => {
                ctx.push_style(ctx.current_style().add_modifier(Modifier::CROSSED_OUT));
            }
            Event::End(TagEnd::Strikethrough) => ctx.pop_style(),

            // ── Links ──────────────────────────────────────────────────────
            Event::Start(Tag::Link { dest_url, .. }) => {
                ctx.push_style(
                    Style::default().fg(link_color).add_modifier(Modifier::UNDERLINED),
                );
                let _ = dest_url;
            }
            Event::End(TagEnd::Link) => ctx.pop_style(),

            // ── Lists ──────────────────────────────────────────────────────
            Event::Start(Tag::List(first_num)) => {
                ctx.list_depth += 1;
                ctx.list_ordered.push(first_num.is_some());
                ctx.ordered_counter.push(first_num.unwrap_or(1));
                if !ctx.spans.is_empty() {
                    ctx.flush_line();
                }
            }
            Event::End(TagEnd::List(_)) => {
                ctx.flush_line();
                ctx.list_depth = ctx.list_depth.saturating_sub(1);
                ctx.list_ordered.pop();
                ctx.ordered_counter.pop();
                if ctx.list_depth == 0 {
                    ctx.blank();
                }
            }
            Event::Start(Tag::Item) => {
                ctx.flush_line();
                let prefix = ctx.bullet_prefix();
                let s = Style::default().fg(accent);
                ctx.spans.push(Span::styled(prefix, s));
                ctx.push_style(Style::default().fg(fg));
                // increment ordered counter
                if let Some(n) = ctx.ordered_counter.last_mut() {
                    *n += 1;
                }
            }
            Event::End(TagEnd::Item) => {
                ctx.pop_style();
                ctx.flush_line();
            }

            // ── Code inline ────────────────────────────────────────────────
            Event::Code(code) => {
                ctx.spans.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(accent).add_modifier(Modifier::DIM),
                ));
            }

            // ── Code block ─────────────────────────────────────────────────
            Event::Start(Tag::CodeBlock(kind)) => {
                ctx.in_code_block = true;
                ctx.flush_line();
                let lang = match &kind {
                    CodeBlockKind::Fenced(l) if !l.is_empty() => format!(" {} ", l),
                    _ => String::new(),
                };
                let prefix = format!("┌─{}─ ", lang);
                let copy_label = "[copy]";
                let line_idx = ctx.lines.len();
                let x_start = prefix.chars().count() as u16;
                let x_end = x_start + copy_label.chars().count() as u16;
                ctx.lines.push(Line::from(vec![
                    Span::styled(
                        prefix,
                        Style::default().fg(accent).add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        copy_label,
                        Style::default().fg(link_color).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    ),
                ]));
                ctx.code_block_header_line = Some(line_idx);
                ctx.code_block_copy_end = Some(x_end);
                ctx.code_block_text.clear();
                ctx.push_style(Style::default().fg(accent).add_modifier(Modifier::DIM));
            }
            Event::End(TagEnd::CodeBlock) => {
                ctx.in_code_block = false;
                ctx.flush_line();
                ctx.pop_style();
                ctx.lines.push(Line::from(Span::styled(
                    "└─",
                    Style::default().fg(accent).add_modifier(Modifier::DIM),
                )));
                if let Some(row) = ctx.code_block_header_line.take() {
                    let x_end = ctx.code_block_copy_end.take().unwrap_or(0);
                    ctx.copy_targets.push(PreviewCopyTarget {
                        row,
                        x_start: {
                            // recompute from rendered header line to stay aligned
                            let mut pos = 0u16;
                            if let Some(line) = ctx.lines.get(row) {
                                for span in &line.spans {
                                    let s = span.content.as_ref();
                                    if s == "[copy]" {
                                        break;
                                    }
                                    pos += s.chars().count() as u16;
                                }
                            }
                            pos
                        },
                        x_end,
                        text: ctx.code_block_text.clone(),
                    });
                }
                ctx.blank();
            }

            // ── Blockquote ─────────────────────────────────────────────────
            Event::Start(Tag::BlockQuote(_)) => {
                ctx.push_style(Style::default().fg(link_color).add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                ctx.flush_line();
                ctx.pop_style();
                ctx.blank();
            }

            // ── Text ───────────────────────────────────────────────────────
            Event::Text(text) => {
                if ctx.in_code_block {
                    if !ctx.code_block_text.is_empty() {
                        ctx.code_block_text.push('\n');
                    }
                    ctx.code_block_text.push_str(text.as_ref());
                    // cada línea del bloque de código como fila separada
                    for (i, l) in text.lines().enumerate() {
                        if i > 0 {
                            ctx.flush_line_always();
                        }
                        ctx.spans.push(Span::styled(
                            format!("│ {}", l),
                            ctx.current_style(),
                        ));
                    }
                } else {
                    ctx.spans.push(Span::styled(text.to_string(), ctx.current_style()));
                }
            }

            // ── Breaks ─────────────────────────────────────────────────────
            Event::SoftBreak => {
                ctx.flush_line_always();
            }
            Event::HardBreak => {
                ctx.flush_line();
            }

            // ── Horizontal rule ────────────────────────────────────────────
            Event::Rule => {
                ctx.flush_line();
                ctx.lines.push(Line::from(Span::styled(
                    "─".repeat(50),
                    Style::default().fg(header_color).add_modifier(Modifier::DIM),
                )));
                ctx.blank();
            }

            // ── Task list checkboxes ────────────────────────────────────────
            Event::TaskListMarker(checked) => {
                let mark = if checked { "[x] " } else { "[ ] " };
                ctx.spans.push(Span::styled(
                    mark,
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ));
            }

            _ => {}
        }
    }

    ctx.flush_line();
    RenderedMarkdown {
        lines: ctx.lines,
        copy_targets: ctx.copy_targets,
    }
}

fn wrap_lines(
    lines: Vec<Line<'static>>,
    copy_targets: Vec<PreviewCopyTarget>,
    width: usize,
) -> RenderedMarkdown {
    if width == 0 {
        return RenderedMarkdown { lines, copy_targets };
    }
    let mut target_by_row = std::collections::HashMap::new();
    for target in copy_targets {
        target_by_row.insert(target.row, target);
    }

    let mut result = Vec::new();
    let mut wrapped_targets = Vec::new();
    for (src_row, line) in lines.into_iter().enumerate() {
        let mut row_target = target_by_row.remove(&src_row);
        if row_target.is_some() {
            let new_row = result.len();
            if let Some(mut t) = row_target.take() {
                t.row = new_row;
                wrapped_targets.push(t);
            }
            result.push(line);
            continue;
        }

        // Flatten spans to (char, Style) for word-boundary wrapping
        let chars: Vec<(char, Style)> = line
            .spans
            .iter()
            .flat_map(|s| s.content.chars().map(move |c| (c, s.style)))
            .collect();

        if chars.len() <= width {
            result.push(line);
            continue;
        }

        let mut start = 0;
        while start < chars.len() {
            let remaining = chars.len() - start;
            if remaining <= width {
                result.push(chars_to_line(&chars[start..]));
                break;
            }

            // Find last space within [start, start+width) to break at a word boundary
            let window = &chars[start..start + width];
            let (line_end, next_start) = match window.iter().rposition(|(c, _)| *c == ' ') {
                Some(p) if p > 0 => (start + p, start + p + 1),
                _ => (start + width, start + width), // no space: hard break
            };

            result.push(chars_to_line(&chars[start..line_end]));
            start = next_start;
        }
    }
    RenderedMarkdown {
        lines: result,
        copy_targets: wrapped_targets,
    }
}

fn chars_to_line(chars: &[(char, Style)]) -> Line<'static> {
    if chars.is_empty() {
        return Line::from("");
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut text = String::new();
    let mut cur_style = chars[0].1;
    for &(ch, style) in chars {
        if style == cur_style {
            text.push(ch);
        } else {
            spans.push(Span::styled(text, cur_style));
            text = String::new();
            cur_style = style;
            text.push(ch);
        }
    }
    if !text.is_empty() {
        spans.push(Span::styled(text, cur_style));
    }
    Line::from(spans)
}
