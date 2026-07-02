use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub header: String,
    pub link: String,
    pub border: String,
    pub status_bg: String,
    pub tab_active: String,
    pub tab_inactive: String,
    pub sidebar_bg: String,
    pub cursor: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}

impl Theme {
    // ── Presets ──────────────────────────────────────────────────────────────

    pub fn presets() -> Vec<(&'static str, Theme)> {
        vec![
            ("Default", Self::default_theme()),
            ("Matrix",  Self::matrix()),
            ("SDRX",    Self::sdrx()),
            ("Custom",  Self::default_theme()), // placeholder — user edits
        ]
    }

    pub fn preset_names() -> Vec<&'static str> {
        vec!["Default", "Matrix", "SDRX", "Custom"]
    }

    fn default_theme() -> Self {
        Self {
            bg:           "#1e1e2e".into(),
            fg:           "#cdd6f4".into(),
            accent:       "#89b4fa".into(),
            header:       "#cba6f7".into(),
            link:         "#a6e3a1".into(),
            border:       "#313244".into(),
            status_bg:    "#181825".into(),
            tab_active:   "#89b4fa".into(),
            tab_inactive: "#585b70".into(),
            sidebar_bg:   "#181825".into(),
            cursor:       "#f5e0dc".into(),
        }
    }

    fn matrix() -> Self {
        Self {
            bg:           "#000000".into(),
            fg:           "#00ff41".into(),
            accent:       "#00ff41".into(),
            header:       "#39ff14".into(),
            link:         "#00cc33".into(),
            border:       "#003300".into(),
            status_bg:    "#001100".into(),
            tab_active:   "#00ff41".into(),
            tab_inactive: "#005500".into(),
            sidebar_bg:   "#001100".into(),
            cursor:       "#39ff14".into(),
        }
    }

    fn sdrx() -> Self {
        Self {
            bg:           "#0d0014".into(),
            fg:           "#00ff00".into(),
            accent:       "#00e5cc".into(),
            header:       "#bf5fff".into(),
            link:         "#ffffff".into(),
            border:       "#2d0057".into(),
            status_bg:    "#08000f".into(),
            tab_active:   "#00e5cc".into(),
            tab_inactive: "#ff00ff".into(),
            sidebar_bg:   "#08000f".into(),
            cursor:       "#00e5cc".into(),
        }
    }

    // ── Field access ─────────────────────────────────────────────────────────

    pub fn fields_mut(&mut self) -> Vec<(&'static str, &mut String)> {
        vec![
            ("bg",           &mut self.bg),
            ("fg",           &mut self.fg),
            ("accent",       &mut self.accent),
            ("header",       &mut self.header),
            ("link",         &mut self.link),
            ("border",       &mut self.border),
            ("status_bg",    &mut self.status_bg),
            ("tab_active",   &mut self.tab_active),
            ("tab_inactive", &mut self.tab_inactive),
            ("sidebar_bg",   &mut self.sidebar_bg),
            ("cursor",       &mut self.cursor),
        ]
    }

    pub fn get_field_by_index(&self, idx: usize) -> String {
        let values = [
            &self.bg, &self.fg, &self.accent, &self.header, &self.link,
            &self.border, &self.status_bg, &self.tab_active, &self.tab_inactive,
            &self.sidebar_bg, &self.cursor,
        ];
        values.get(idx).map(|s| s.as_str()).unwrap_or("").to_string()
    }

    pub fn set_field_by_index(&mut self, idx: usize, val: String) {
        match idx {
            0  => self.bg = val,
            1  => self.fg = val,
            2  => self.accent = val,
            3  => self.header = val,
            4  => self.link = val,
            5  => self.border = val,
            6  => self.status_bg = val,
            7  => self.tab_active = val,
            8  => self.tab_inactive = val,
            9  => self.sidebar_bg = val,
            10 => self.cursor = val,
            _  => {}
        }
    }

    pub fn parse_color(hex: &str) -> ratatui::style::Color {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return ratatui::style::Color::Rgb(r, g, b);
            }
        }
        ratatui::style::Color::Reset
    }
}
