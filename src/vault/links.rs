use regex::Regex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    re.captures_iter(content)
        .map(|c| c[1].to_string())
        .collect()
}

pub fn resolve_link(vault_root: &Path, link_name: &str) -> Option<PathBuf> {
    let target_stem = link_name.trim();
    WalkDir::new(vault_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path().extension().map(|x| x == "md").unwrap_or(false)
                && e.path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case(target_stem))
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
}

pub fn find_link_at_cursor(content: &str, line: usize, col: usize) -> Option<String> {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let lines: Vec<&str> = content.lines().collect();
    let line_str = lines.get(line)?;
    for cap in re.captures_iter(line_str) {
        let m = cap.get(0)?;
        if m.start() <= col && col <= m.end() {
            return Some(cap[1].to_string());
        }
    }
    None
}
