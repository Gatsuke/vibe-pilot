//! Git integration via the `git` CLI (zero deps, always accurate).
//!
//! - branch + working-tree changes (`git status --porcelain`)
//! - per-file modification recency (`git log --name-only`)
//! - untracked files, so brand-new work still ranks high

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub rel: String,
    pub status: char, // M / A / D / R ...
}

#[derive(Debug, Clone)]
pub struct GitInfo {
    pub branch: String,
    pub changed: Vec<ChangedFile>,
}

fn run(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Returns None when `root` is not inside a git work tree.
pub fn probe(root: &Path) -> Option<GitInfo> {
    run(root, &["rev-parse", "--is-inside-work-tree"])?;
    let branch = run(root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "HEAD".to_string());
    let status = run(root, &["status", "--porcelain=v1", "-uall"])?;
    let mut changed = Vec::new();
    for line in status.lines() {
        if line.len() < 4 {
            continue;
        }
        let bytes = line.as_bytes();
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let mut rest = &line[3..];
        // renames arrive as "old -> new" — keep the new path
        if let Some(pos) = rest.rfind(" -> ") {
            rest = &rest[pos + 4..];
        }
        let path = unquote(rest);
        if path.is_empty() {
            continue;
        }
        let status_char = if x == '?' {
            'A'
        } else if x != ' ' {
            x
        } else {
            y
        };
        changed.push(ChangedFile {
            rel: path,
            status: status_char,
        });
    }
    Some(GitInfo { branch, changed })
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        let mut out = String::new();
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('t') => out.push('\t'),
                    Some('n') => out.push('\n'),
                    Some(other) => out.push(other),
                    None => {}
                }
            } else {
                out.push(c);
            }
        }
        out
    } else {
        s.to_string()
    }
}

/// Recency score per file (0.0–1.0) from git history. Recent work wins.
pub fn recency(root: &Path) -> HashMap<String, f64> {
    let mut map: HashMap<String, f64> = HashMap::new();
    let now = crate::util::now_secs();
    if let Some(log) = run(
        root,
        &["log", "-n", "600", "--name-only", "--format=%x1e%ct"],
    ) {
        for record in log.split('\u{1e}') {
            let record = record.trim_start_matches('\n');
            if record.trim().is_empty() {
                continue;
            }
            let mut lines = record.lines();
            let ts: u64 = match lines.next().and_then(|l| l.trim().parse().ok()) {
                Some(t) => t,
                None => continue,
            };
            for f in lines {
                let f = f.trim();
                if f.is_empty() || map.contains_key(f) {
                    continue;
                }
                map.insert(f.to_string(), bucket(now.saturating_sub(ts)));
            }
        }
    }
    if let Some(untracked) = run(root, &["ls-files", "--others", "--exclude-standard"]) {
        for f in untracked.lines() {
            let f = f.trim();
            if !f.is_empty() {
                map.insert(f.to_string(), 0.9);
            }
        }
    }
    map
}

fn bucket(age_secs: u64) -> f64 {
    if age_secs <= 86_400 {
        1.0
    } else if age_secs <= 7 * 86_400 {
        0.8
    } else if age_secs <= 30 * 86_400 {
        0.6
    } else if age_secs <= 90 * 86_400 {
        0.4
    } else {
        0.2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquote_paths() {
        assert_eq!(unquote("src/main.rs"), "src/main.rs");
        assert_eq!(unquote("\"src/\\\"weird\\\".rs\""), "src/\"weird\".rs");
    }
}
