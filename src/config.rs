//! Config: built-in profiles + a tolerant `vibe-pilot.toml` (subset of TOML:
//! sections, strings, ints, bools, string arrays — enough for a config file,
//! without pulling in a dependency).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: &'static str,
    pub desc: &'static str,
    pub max_tokens: usize,
    pub strip_comments: bool,
    pub include_tests: bool,
    pub include_docs: bool,
    pub max_file_bytes: u64,
}

pub fn all() -> Vec<Profile> {
    vec![
        Profile {
            name: "lean",
            desc: "skeleton crew - code only, no comments, tiny budget",
            max_tokens: 8_000,
            strip_comments: true,
            include_tests: false,
            include_docs: false,
            max_file_bytes: 64 * 1024,
        },
        Profile {
            name: "cursor",
            desc: "tuned for Cursor's effective attention window",
            max_tokens: 24_000,
            strip_comments: false,
            include_tests: false,
            include_docs: true,
            max_file_bytes: 128 * 1024,
        },
        Profile {
            name: "windsurf",
            desc: "same shape as cursor, Windsurf rules files included",
            max_tokens: 24_000,
            strip_comments: false,
            include_tests: false,
            include_docs: true,
            max_file_bytes: 128 * 1024,
        },
        Profile {
            name: "aider",
            desc: "repo-map style: aggressive pruning, focus files",
            max_tokens: 12_000,
            strip_comments: true,
            include_tests: false,
            include_docs: false,
            max_file_bytes: 96 * 1024,
        },
        Profile {
            name: "claude",
            desc: "large window, doc comments kept, tests included",
            max_tokens: 60_000,
            strip_comments: false,
            include_tests: true,
            include_docs: true,
            max_file_bytes: 192 * 1024,
        },
        Profile {
            name: "full",
            desc: "everything except noise (lockfiles, generated, binaries)",
            max_tokens: usize::MAX,
            strip_comments: false,
            include_tests: true,
            include_docs: true,
            max_file_bytes: 256 * 1024,
        },
    ]
}

pub fn builtin(name: &str) -> Option<Profile> {
    all().into_iter().find(|p| p.name == name)
}

#[derive(Debug, Clone, Default)]
pub struct ProfileOverride {
    pub max_tokens: Option<usize>,
    pub strip_comments: Option<bool>,
    pub include_tests: Option<bool>,
    pub include_docs: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub profile: String,
    pub model: String,
    pub ignore: Vec<String>,
    pub overrides: HashMap<String, ProfileOverride>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            profile: "claude".to_string(),
            model: "claude-sonnet-5.5".to_string(),
            ignore: Vec::new(),
            overrides: HashMap::new(),
        }
    }
}

pub fn config_path(root: &Path) -> PathBuf {
    root.join("vibe-pilot.toml")
}

/// Load config from `root`, printing warnings for anything odd. Never fails.
pub fn load(root: &Path) -> Config {
    let path = config_path(root);
    match std::fs::read_to_string(&path) {
        Ok(src) => match parse(&src) {
            Ok((cfg, warnings)) => {
                for w in warnings {
                    eprintln!("[vp] vibe-pilot.toml: {}", w);
                }
                cfg
            }
            Err(e) => {
                eprintln!("[vp] vibe-pilot.toml parse error (using defaults): {}", e);
                Config::default()
            }
        },
        Err(_) => Config::default(),
    }
}

/// Parse the TOML subset. Returns config + warnings for unknown keys.
pub fn parse(src: &str) -> Result<(Config, Vec<String>), String> {
    let mut cfg = Config::default();
    let mut warnings = Vec::new();
    let mut section = String::new();

    for (no, raw) in src.lines().enumerate() {
        let line = strip_comment(raw).trim().to_string();
        if line.is_empty() {
            continue;
        }
        let lineno = no + 1;
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            if !["vp".into(), "profiles".into()]
                .contains(&section.split('.').next().unwrap_or("").to_string())
            {
                warnings.push(format!("line {}: unknown section [{}]", lineno, section));
            }
            continue;
        }
        let (k, v) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => return Err(format!("line {}: expected key = value", lineno)),
        };
        if section == "vp" {
            match k {
                "profile" => cfg.profile = parse_string(v)?,
                "model" => cfg.model = parse_string(v)?,
                "ignore" => cfg.ignore = parse_string_array(v)?,
                _ => warnings.push(format!("line {}: unknown key '{}' in [vp]", lineno, k)),
            }
        } else if let Some(prof) = section.strip_prefix("profiles.") {
            let prof = prof.trim();
            let o = cfg.overrides.entry(prof.to_string()).or_default();
            match k {
                "max_tokens" => o.max_tokens = Some(parse_int(v)?),
                "strip_comments" => o.strip_comments = Some(parse_bool(v)?),
                "include_tests" => o.include_tests = Some(parse_bool(v)?),
                "include_docs" => o.include_docs = Some(parse_bool(v)?),
                _ => warnings.push(format!(
                    "line {}: unknown key '{}' in [profiles.{}]",
                    lineno, k, prof
                )),
            }
        } else if section.is_empty() {
            warnings.push(format!("line {}: key outside any section", lineno));
        }
    }
    Ok((cfg, warnings))
}

/// Apply config overrides on top of a built-in profile.
pub fn apply_overrides(mut p: Profile, cfg: &Config) -> Profile {
    if let Some(o) = cfg.overrides.get(p.name) {
        if let Some(v) = o.max_tokens {
            p.max_tokens = v;
        }
        if let Some(v) = o.strip_comments {
            p.strip_comments = v;
        }
        if let Some(v) = o.include_tests {
            p.include_tests = v;
        }
        if let Some(v) = o.include_docs {
            p.include_docs = v;
        }
    }
    p
}

fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '#' if !in_str => return &line[..i],
            _ => {}
        }
    }
    line
}

fn parse_string(v: &str) -> Result<String, String> {
    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
        Ok(v[1..v.len() - 1].to_string())
    } else {
        Err(format!("expected quoted string, got '{}'", v))
    }
}

fn parse_bool(v: &str) -> Result<bool, String> {
    match v {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected true/false, got '{}'", v)),
    }
}

fn parse_int(v: &str) -> Result<usize, String> {
    v.parse::<usize>()
        .map_err(|_| format!("expected integer, got '{}'", v))
}

fn parse_string_array(v: &str) -> Result<Vec<String>, String> {
    let v = v.trim();
    if v == "[]" {
        return Ok(Vec::new());
    }
    if !(v.starts_with('[') && v.ends_with(']')) {
        return Err(format!("expected [..] array, got '{}'", v));
    }
    let inner = &v[1..v.len() - 1];
    let mut out = Vec::new();
    for part in inner.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        out.push(parse_string(p)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let src = "# comment\n[vp]\nprofile = \"lean\"\nignore = [\"docs/\", \"bench/\"]\n\n[profiles.lean]\nmax_tokens = 4096\nstrip_comments = true\n";
        let (cfg, w) = parse(src).unwrap();
        assert!(w.is_empty());
        assert_eq!(cfg.profile, "lean");
        assert_eq!(cfg.ignore, vec!["docs/", "bench/"]);
        assert_eq!(cfg.overrides["lean"].max_tokens, Some(4096));
        let p = apply_overrides(builtin("lean").unwrap(), &cfg);
        assert_eq!(p.max_tokens, 4096);
        assert!(p.strip_comments);
    }

    #[test]
    fn builtin_profiles_exist() {
        assert_eq!(all().len(), 6);
        assert!(builtin("claude").is_some());
        assert!(builtin("nope").is_none());
    }
}
