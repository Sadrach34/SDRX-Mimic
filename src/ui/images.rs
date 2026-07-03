use std::sync::OnceLock;

use base64::Engine;
use image::DynamicImage;
use ratatui::style::Color;
use ratatui::text::{Line, Span};

/// Upper bound (longest side, px) for images sent over the kitty protocol.
/// The terminal — not us — scales the bitmap down to fit `c=cols,r=rows`
/// cells using its own real cell pixel size, so we just need enough source
/// resolution to look sharp; downscaling here first (as if every terminal
/// used the same fixed cell size) is what caused blurry/pixelated output.
const MAX_KITTY_DIM: u32 = 1600;

static KITTY_PROBE: OnceLock<bool> = OnceLock::new();

/// Detects kitty graphics protocol support from terminal environment
/// variables. Mirrors mditor's `kittySupported()` heuristic.
pub fn kitty_supported() -> bool {
    *KITTY_PROBE.get_or_init(|| {
        if std::env::var_os("TERM_GRAPHICS").is_some() {
            return true;
        }
        // KITTY_PID/KITTY_WINDOW_ID/GHOSTTY_*/WEZTERM_* are set by the local
        // terminal process and are NOT forwarded over SSH (they're not in
        // sshd's default AcceptEnv list), so over SSH only $TERM survives —
        // check each signal independently instead of requiring pairs.
        if std::env::var_os("KITTY_WINDOW_ID").is_some() || std::env::var_os("KITTY_PID").is_some() {
            return true;
        }
        if std::env::var_os("GHOSTTY_RESOURCES_DIR").is_some() {
            return true;
        }
        if std::env::var_os("WEZTERM_EXECUTABLE").is_some() || std::env::var_os("WEZTERM_PANE").is_some() {
            return true;
        }
        // $TERM is forwarded over SSH (needed for termcap/terminfo), so this
        // is the one signal that reliably survives a remote session — but
        // tmux/screen rewrite it to "screen*"/"tmux*", masking the real
        // terminal underneath. Substring match, not exact, since some
        // configs use e.g. "xterm-kitty-256color".
        if std::env::var("TERM").map(|t| t.contains("kitty")).unwrap_or(false) {
            return true;
        }
        if std::env::var("TERM_PROGRAM")
            .map(|p| {
                let p = p.to_lowercase();
                p.contains("ghostty") || p.contains("wezterm") || p.contains("kitty")
            })
            .unwrap_or(false)
        {
            return true;
        }
        false
    })
}

pub fn load_image(path: &std::path::Path) -> Option<DynamicImage> {
    let is_svg = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("svg"))
        .unwrap_or(false);
    if is_svg {
        let data = std::fs::read(path).ok()?;
        return rasterize_svg(&data);
    }
    image::open(path).ok()
}

/// Downloads an image over HTTP(S). Blocking — callers must run this off
/// the render thread (see `App::web_images` / `blit_images`), since a slow
/// or hanging server would otherwise freeze the whole UI.
pub fn fetch_image(url: &str) -> Option<DynamicImage> {
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .ok()?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut resp.into_reader(), &mut bytes).ok()?;
    // The URL's extension is unreliable (query strings, extensionless CDN
    // paths), so just try raster decode first and fall back to SVG.
    match image::load_from_memory(&bytes) {
        Ok(img) => Some(img),
        Err(_) => rasterize_svg(&bytes),
    }
}

/// Rasterizes an SVG document to a bitmap at its intrinsic size — the
/// `image` crate is raster-only and can't decode vector formats, so SVGs
/// need converting to pixels before they can flow through the same
/// mosaic/kitty pipeline as PNG/JPEG/etc.
fn rasterize_svg(data: &[u8]) -> Option<DynamicImage> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(data, &opt).ok()?;
    let size = tree.size();
    let w = size.width().ceil().max(1.0) as u32;
    let h = size.height().ceil().max(1.0) as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // tiny-skia stores premultiplied alpha; image::RgbaImage expects
    // straight alpha, so undo the premultiplication per pixel.
    let mut raw = pixmap.take();
    for px in raw.chunks_exact_mut(4) {
        let a = px[3];
        if a > 0 && a < 255 {
            px[0] = ((px[0] as u32 * 255) / a as u32).min(255) as u8;
            px[1] = ((px[1] as u32 * 255) / a as u32).min(255) as u8;
            px[2] = ((px[2] as u32 * 255) / a as u32).min(255) as u8;
        }
    }

    let buf = image::RgbaImage::from_raw(w, h, raw)?;
    Some(DynamicImage::ImageRgba8(buf))
}

/// Encodes an image as a kitty graphics protocol (APC `ESC_G`) escape
/// sequence chunked into base64 payloads, sized to fill `cols x rows`
/// terminal cells. Caller is responsible for positioning the cursor at
/// the target cell before writing these bytes.
pub fn encode_kitty(img: &DynamicImage, cols: u16, rows: u16) -> Vec<u8> {
    let (orig_w, orig_h) = (img.width(), img.height());
    let rgba = if orig_w > MAX_KITTY_DIM || orig_h > MAX_KITTY_DIM {
        img.resize(MAX_KITTY_DIM, MAX_KITTY_DIM, image::imageops::FilterType::Lanczos3)
            .to_rgba8()
    } else {
        img.to_rgba8()
    };
    let (w, h) = rgba.dimensions();

    let payload = base64::engine::general_purpose::STANDARD.encode(rgba.as_raw());
    let chunks: Vec<&str> = payload
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    let mut out = Vec::new();
    let n = chunks.len();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = if i + 1 < n { 1 } else { 0 };
        if i == 0 {
            out.extend_from_slice(
                format!(
                    "\x1b_Ga=T,f=32,s={},v={},c={},r={},m={};",
                    w, h, cols, rows, more
                )
                .as_bytes(),
            );
        } else {
            out.extend_from_slice(format!("\x1b_Gm={};", more).as_bytes());
        }
        out.extend_from_slice(chunk.as_bytes());
        out.extend_from_slice(b"\x1b\\");
    }
    out
}

#[derive(Clone)]
pub struct PreparedImage {
    pub mosaic: Vec<Line<'static>>,
    /// Present only when the terminal advertised kitty graphics support;
    /// pre-encoded and ready to write at the target cell position.
    pub kitty_bytes: Option<Vec<u8>>,
}

/// Scales and encodes an already-decoded image once so callers can cache
/// the result per (path, cols, rows) and blit/write it every frame without
/// re-preparing.
pub fn prepare_from_image(img: &DynamicImage, cols: u16, rows: u16) -> PreparedImage {
    let mosaic = mosaic_lines(img, cols, rows);
    let kitty_bytes = if kitty_supported() {
        Some(encode_kitty(img, cols, rows))
    } else {
        None
    };
    PreparedImage { mosaic, kitty_bytes }
}

/// Picks the largest `cols x rows` that fits inside `avail_cols x avail_rows`
/// while preserving the source image's aspect ratio, treating each
/// terminal cell as 1 (w) x 2 (h) "pixels" — matches the half-block mosaic
/// sampling and keeps kitty's own cell-to-pixel scaling looking right too.
/// Without this, images get stretched to fill the placeholder box exactly,
/// which is what made everything look smeared/distorted.
pub fn fit_cells(src_w: u32, src_h: u32, avail_cols: u16, avail_rows: u16) -> (u16, u16) {
    if src_w == 0 || src_h == 0 || avail_cols == 0 || avail_rows == 0 {
        return (avail_cols.max(1), avail_rows.max(1));
    }
    let avail_px_w = avail_cols as f64;
    let avail_px_h = avail_rows as f64 * 2.0;
    let scale = (avail_px_w / src_w as f64).min(avail_px_h / src_h as f64);
    let fit_px_w = (src_w as f64 * scale).round().max(1.0);
    let fit_px_h = (src_h as f64 * scale).round().max(2.0);
    let cols = (fit_px_w as u16).clamp(1, avail_cols);
    let rows = ((fit_px_h / 2.0).round() as u16).clamp(1, avail_rows);
    (cols, rows)
}

/// Renders an image as half-block ANSI art (▀ with distinct fg/bg per
/// cell), used as a universal fallback when kitty isn't supported — and
/// as the always-drawn base layer, since it lives in the ratatui Buffer
/// directly rather than needing raw terminal escapes.
pub fn mosaic_lines(img: &DynamicImage, cols: u16, rows: u16) -> Vec<Line<'static>> {
    let cols = cols.max(1);
    let rows = rows.max(1);
    // Each terminal row renders two vertical pixel samples (top half-block
    // glyph over two stacked color cells) for roughly square-looking output.
    let px_rows = rows as u32 * 2;
    let scaled = img.resize_exact(cols as u32, px_rows, image::imageops::FilterType::Triangle);
    let rgba = scaled.to_rgba8();

    let mut lines = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let mut spans = Vec::with_capacity(cols as usize);
        for col in 0..cols {
            let top = rgba.get_pixel(col as u32, (row as u32) * 2);
            let bottom = rgba.get_pixel(col as u32, (row as u32) * 2 + 1);
            let fg = Color::Rgb(top[0], top[1], top[2]);
            let bg = Color::Rgb(bottom[0], bottom[1], bottom[2]);
            spans.push(Span::styled(
                "▀",
                ratatui::style::Style::default().fg(fg).bg(bg),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines
}
