#[derive(Debug, Clone)]
pub enum HookEvent {
    NoteSave { path: String, content: String },
    NoteOpen { path: String, content: String },
    ModeChange { from: String, to: String },
    MarkdownBlock { lang: String, code: String },
    VaultOpen { path: String },
}

impl HookEvent {
    pub fn name(&self) -> &str {
        match self {
            HookEvent::NoteSave { .. } => "on_save",
            HookEvent::NoteOpen { .. } => "on_open",
            HookEvent::ModeChange { .. } => "on_mode_change",
            HookEvent::MarkdownBlock { .. } => "on_markdown_block",
            HookEvent::VaultOpen { .. } => "on_vault_open",
        }
    }

    /// Permission required for this hook to be delivered
    pub fn required_permission(&self) -> &str {
        match self {
            HookEvent::NoteSave { .. } => "hooks.save",
            HookEvent::NoteOpen { .. } => "hooks.open",
            HookEvent::ModeChange { .. } => "hooks.mode",
            HookEvent::MarkdownBlock { .. } => "markdown",
            HookEvent::VaultOpen { .. } => "hooks.vault",
        }
    }
}

#[derive(Debug, Clone)]
pub enum HookResult {
    None,
    Text(String),
}
