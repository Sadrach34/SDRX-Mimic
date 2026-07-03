/// Manual soft-wrap for the editor. tui-textarea 0.7 has no native word
/// wrap (each logical line maps 1:1 to a screen row), so long lines get
/// wrapped here purely for display, while the underlying `TextArea` keeps
/// tracking cursor position in logical (line, col) terms.
pub struct WrappedLine {
    /// 0-based index of the logical (buffer) line this segment belongs to.
    pub logical_row: usize,
    /// True for the first wrapped segment of a logical line — only this
    /// segment gets a line-number gutter, matching how every wrap-capable
    /// editor prints the number once per source line, not per screen row.
    pub is_first_segment: bool,
    pub text: String,
    /// Local char range `[start, end)` within `text` that falls inside the
    /// active selection, if any part of this segment is selected.
    pub highlight: Option<(usize, usize)>,
}

pub struct WrapResult {
    pub lines: Vec<WrappedLine>,
    pub cursor_screen_row: usize,
    pub cursor_screen_col: usize,
}

/// `selection` is a normalized `(start, end)` pair of logical `(row, col)`
/// positions, as returned by `TextArea::selection_range` (start <= end).
pub fn wrap_editor_lines(
    logical_lines: &[String],
    width: usize,
    cursor: (usize, usize),
    selection: Option<((usize, usize), (usize, usize))>,
) -> WrapResult {
    let width = width.max(1);
    let mut out: Vec<WrappedLine> = Vec::new();
    let mut cursor_screen_row = 0usize;
    let mut cursor_screen_col = 0usize;

    let seg_highlight = |row_idx: usize, seg_start: usize, seg_end: usize| -> Option<(usize, usize)> {
        let ((sr, sc), (er, ec)) = selection?;
        if row_idx < sr || row_idx > er {
            return None;
        }
        let lo = if row_idx == sr { sc.max(seg_start) } else { seg_start };
        let hi = if row_idx == er { ec.min(seg_end) } else { seg_end };
        if lo < hi {
            Some((lo - seg_start, hi - seg_start))
        } else {
            None
        }
    };

    for (row_idx, line) in logical_lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();

        if chars.is_empty() {
            if row_idx == cursor.0 {
                cursor_screen_row = out.len();
                cursor_screen_col = 0;
            }
            out.push(WrappedLine {
                logical_row: row_idx,
                is_first_segment: true,
                text: String::new(),
                highlight: None,
            });
            continue;
        }

        let mut start = 0usize;
        let mut first = true;
        while start < chars.len() {
            let remaining = chars.len() - start;
            let (seg_end, next_start) = if remaining <= width {
                (chars.len(), chars.len())
            } else {
                let window_end = start + width;
                match chars[start..window_end].iter().rposition(|c| *c == ' ') {
                    Some(p) if p > 0 => (start + p, start + p + 1),
                    _ => (window_end, window_end),
                }
            };

            let seg_row = out.len();
            if row_idx == cursor.0 && cursor.1 >= start && cursor.1 <= seg_end {
                cursor_screen_row = seg_row;
                cursor_screen_col = cursor.1 - start;
            }

            out.push(WrappedLine {
                logical_row: row_idx,
                is_first_segment: first,
                text: chars[start..seg_end].iter().collect(),
                highlight: seg_highlight(row_idx, start, seg_end),
            });
            first = false;
            start = next_start;
        }
    }

    WrapResult {
        lines: out,
        cursor_screen_row,
        cursor_screen_col,
    }
}
