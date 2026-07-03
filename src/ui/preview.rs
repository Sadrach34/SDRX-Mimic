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

#[derive(Clone, Debug)]
pub struct ImageSpec {
    pub path: String,
    pub alt: String,
    /// Row index (into `RenderedMarkdown::lines`) of the first interior row
    /// of the placeholder box, i.e. where the image content itself starts.
    pub row: usize,
    pub col_start: u16,
    pub inner_width: u16,
    pub inner_height: u16,
}

pub struct RenderedMarkdown {
    pub lines: Vec<Line<'static>>,
    pub copy_targets: Vec<PreviewCopyTarget>,
    pub images: Vec<ImageSpec>,
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

    let raw = render_markdown(content, fg, header_color, link_color, accent, width);
    wrap_lines(raw.lines, raw.copy_targets, raw.images, width)
}

struct RenderCtx {
    lines: Vec<Line<'static>>,
    copy_targets: Vec<PreviewCopyTarget>,
    images: Vec<ImageSpec>,
    spans: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    list_depth: usize,
    list_ordered: Vec<bool>,
    ordered_counter: Vec<u64>,
    in_code_block: bool,
    code_block_header_line: Option<usize>,
    code_block_text: String,
    heading_level: Option<HeadingLevel>,
    in_image: bool,
    image_alt: String,
    image_url: String,
    fg: Color,
}

impl RenderCtx {
    fn new(fg: Color) -> Self {
        Self {
            lines: Vec::new(),
            copy_targets: Vec::new(),
            images: Vec::new(),
            spans: Vec::new(),
            style_stack: vec![Style::default().fg(fg)],
            list_depth: 0,
            list_ordered: Vec::new(),
            ordered_counter: Vec::new(),
            in_code_block: false,
            code_block_header_line: None,
            code_block_text: String::new(),
            heading_level: None,
            in_image: false,
            image_alt: String::new(),
            image_url: String::new(),
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

// Paleta fija de mditor (charm.land/uict), independiente del Theme del
// usuario: cada nivel de heading es un "badge" con fondo sólido en vez de
// prefijo "#" + regla. Colores tomados 1:1 de internal/uict/colors.go (dark).
fn heading_badge_colors(level: HeadingLevel) -> (Color, Color) {
    match level {
        HeadingLevel::H1 => (Color::Rgb(0xF7, 0xF6, 0xFB), Color::Rgb(0xC2, 0x59, 0xFF)), // Salt / Violet
        HeadingLevel::H2 => (Color::Rgb(0x20, 0x1F, 0x26), Color::Rgb(0x00, 0xA4, 0xFF)), // Pepper / Malibu
        HeadingLevel::H3 => (Color::Rgb(0x20, 0x1F, 0x26), Color::Rgb(0x00, 0xFF, 0xB2)), // Pepper / Julep
        HeadingLevel::H4 => (Color::Rgb(0xF7, 0xF6, 0xFB), Color::Rgb(0x6B, 0x50, 0xFF)), // Salt / Charple
        HeadingLevel::H5 => (Color::Rgb(0xA2, 0xA0, 0xAD), Color::Rgb(0x3A, 0x39, 0x43)), // Steam / Char
        _ => (Color::Rgb(0x85, 0x83, 0x92), Color::Rgb(0x2D, 0x2C, 0x36)),                // Squid / BBQ
    }
}

fn heading_line(text: &str, level: HeadingLevel, _color: Color) -> Line<'static> {
    let mut text = text.trim().to_string();
    if level == HeadingLevel::H1 {
        text = text.to_uppercase();
    }
    let (fg, bg) = heading_badge_colors(level);
    let mut style = Style::default().fg(fg).bg(bg);
    style = match level {
        HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3 | HeadingLevel::H4 => {
            style.add_modifier(Modifier::BOLD)
        }
        HeadingLevel::H6 => style.add_modifier(Modifier::DIM),
        _ => style,
    };
    Line::from(Span::styled(format!(" {} ", text), style))
}

fn heading_rule(level: HeadingLevel, color: Color) -> Option<Line<'static>> {
    let (ch, len) = match level {
        HeadingLevel::H1 => ('═', 48),
        HeadingLevel::H2 => ('─', 40),
        HeadingLevel::H3 => ('╌', 34),
        HeadingLevel::H4 => ('┄', 28),
        HeadingLevel::H5 => ('┈', 22),
        HeadingLevel::H6 => ('·', 16),
    };
    Some(Line::from(Span::styled(
        ch.to_string().repeat(len),
        Style::default().fg(color).add_modifier(Modifier::DIM),
    )))
}

fn render_markdown(
    content: &str,
    fg: Color,
    header_color: Color,
    link_color: Color,
    accent: Color,
    width: usize,
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
                if let Some(rule) = heading_rule(level, header_color) {
                    ctx.lines.push(rule);
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
                    CodeBlockKind::Fenced(l) if !l.is_empty() => l.to_string(),
                    _ => String::new(),
                };
                let copy_label = "[copy]";
                let copy_style =
                    Style::default().fg(link_color).add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

                let border = "┌".to_string();
                let border_style = Style::default().fg(accent).add_modifier(Modifier::DIM);

                let mut spans: Vec<Span<'static>> = vec![Span::styled(border.clone(), border_style)];
                let mut left_len = border.chars().count();

                if !lang.is_empty() {
                    let badge_text = format!(" {} ", lang.to_uppercase());
                    let badge_len = badge_text.chars().count();
                    spans.push(Span::styled(
                        badge_text,
                        Style::default().fg(fg).bg(accent).add_modifier(Modifier::BOLD),
                    ));
                    left_len += badge_len;
                }

                let copy_len = copy_label.chars().count();
                let inner_width = width.max(left_len + copy_len + 1);
                let gap = inner_width.saturating_sub(left_len + copy_len).max(1);
                spans.push(Span::styled(
                    "─".repeat(gap.saturating_sub(1)),
                    border_style,
                ));
                spans.push(Span::styled(" ", Style::default()));
                let x_start = (left_len + gap) as u16;
                let x_end = x_start + copy_len as u16;
                spans.push(Span::styled(copy_label, copy_style));

                let line_idx = ctx.lines.len();
                ctx.lines.push(Line::from(spans));
                ctx.code_block_header_line = Some(line_idx);
                ctx.copy_targets.push(PreviewCopyTarget {
                    row: line_idx,
                    x_start,
                    x_end,
                    text: String::new(),
                });
                ctx.code_block_text.clear();
                ctx.push_style(Style::default().fg(accent).add_modifier(Modifier::DIM));
            }
            Event::End(TagEnd::CodeBlock) => {
                ctx.in_code_block = false;
                ctx.flush_line();
                ctx.pop_style();
                ctx.lines.push(Line::from(Span::styled(
                    format!("└{}", "─".repeat(width.saturating_sub(1))),
                    Style::default().fg(accent).add_modifier(Modifier::DIM),
                )));
                if let Some(row) = ctx.code_block_header_line.take() {
                    if let Some(target) = ctx.copy_targets.iter_mut().rev().find(|t| t.row == row) {
                        target.text = ctx.code_block_text.clone();
                    }
                }
                ctx.code_block_header_line = None;
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

            // ── Images ─────────────────────────────────────────────────────
            Event::Start(Tag::Image { dest_url, .. }) => {
                ctx.flush_line();
                ctx.in_image = true;
                ctx.image_url = dest_url.to_string();
                ctx.image_alt.clear();
            }
            Event::End(TagEnd::Image) => {
                ctx.in_image = false;
                // A tall/wide canvas here matters: the half-block renderer
                // gets 2 vertical color samples per row, so a cramped box
                // (previously a fixed 8 interior rows = 16px tall) looks
                // like a smear no matter how good the source image is.
                let box_w = width.clamp(10, 78);
                let box_h: usize = 30;
                let inner_w = box_w.saturating_sub(2);
                let inner_h = box_h.saturating_sub(2);

                let title = if ctx.image_alt.trim().is_empty() {
                    std::path::Path::new(&ctx.image_url)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| ctx.image_url.clone())
                } else {
                    ctx.image_alt.clone()
                };
                let border_style = Style::default().fg(accent).add_modifier(Modifier::DIM);

                ctx.lines.push(Line::from(Span::styled(
                    format!("┌{}┐", pad_center(&format!(" {} ", title), inner_w, '─')),
                    border_style,
                )));
                let interior_row = ctx.lines.len();
                for _ in 0..inner_h {
                    ctx.lines.push(Line::from(Span::styled(
                        format!("│{}│", " ".repeat(inner_w)),
                        border_style,
                    )));
                }
                ctx.lines.push(Line::from(Span::styled(
                    format!("└{}┘", "─".repeat(inner_w)),
                    border_style,
                )));

                ctx.images.push(ImageSpec {
                    path: ctx.image_url.clone(),
                    alt: ctx.image_alt.clone(),
                    row: interior_row,
                    col_start: 1,
                    inner_width: inner_w as u16,
                    inner_height: inner_h as u16,
                });
                ctx.blank();
            }

            // ── Text ───────────────────────────────────────────────────────
            Event::Text(text) => {
                if ctx.in_image {
                    ctx.image_alt.push_str(text.as_ref());
                } else if ctx.in_code_block {
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
        images: ctx.images,
    }
}

fn pad_center(s: &str, width: usize, fill: char) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.chars().take(width).collect();
    }
    let total_pad = width - len;
    let left = total_pad / 2;
    let right = total_pad - left;
    format!(
        "{}{}{}",
        fill.to_string().repeat(left),
        s,
        fill.to_string().repeat(right)
    )
}

fn wrap_lines(
    lines: Vec<Line<'static>>,
    copy_targets: Vec<PreviewCopyTarget>,
    images: Vec<ImageSpec>,
    width: usize,
) -> RenderedMarkdown {
    if width == 0 {
        return RenderedMarkdown { lines, copy_targets, images };
    }
    let mut target_by_row = std::collections::HashMap::new();
    for target in copy_targets {
        target_by_row.insert(target.row, target);
    }

    // Image placeholder boxes (border + interior) must never be word-wrapped —
    // every row they occupy is pushed through verbatim.
    let mut protected_rows: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for img in &images {
        let top = img.row.saturating_sub(1);
        let bottom = img.row + img.inner_height as usize;
        for r in top..=bottom {
            protected_rows.insert(r);
        }
    }
    let mut image_row_remap: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

    let mut result = Vec::new();
    let mut wrapped_targets = Vec::new();
    for (src_row, line) in lines.into_iter().enumerate() {
        let mut row_target = target_by_row.remove(&src_row);
        if row_target.is_some() || protected_rows.contains(&src_row) {
            let new_row = result.len();
            if let Some(mut t) = row_target.take() {
                t.row = new_row;
                wrapped_targets.push(t);
            }
            image_row_remap.insert(src_row, new_row);
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
    let wrapped_images = images
        .into_iter()
        .map(|mut img| {
            if let Some(&new_row) = image_row_remap.get(&img.row) {
                img.row = new_row;
            }
            img
        })
        .collect();

    RenderedMarkdown {
        lines: result,
        copy_targets: wrapped_targets,
        images: wrapped_images,
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
