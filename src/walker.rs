//! Repo walker: recursive directory traversal that respects `.gitignore`
//! files plus a sane set of built-in excludes (build output, caches, vendor
//! trees). Never follows symlinks. Pure std.

use std::fs;
use std::path::Path;

const MAX_FILES: usize = 60_000;

#[derive(Debug)]
pub struct WalkedFile {
    pub rel: String,
    pub size: u64,
    pub ext: String,
}

#[derive(Debug)]
pub struct WalkResult {
    pub files: Vec<WalkedFile>,
    pub capped: bool,
}

#[derive(Clone, Debug)]
struct Pattern {
    negated: bool,
    dir_only: bool,
    anchored: bool,
    glob: String,
}

struct IgnoreSet {
    base: String, // "" for repo root
    pats: Vec<Pattern>,
}

/// Walk `root`, returning text-relevant files (binaries and assets are still
/// returned — classification happens later, so the report can count them).
pub fn walk(root: &Path, extra_ignore: &[String]) -> WalkResult {
    let mut stack: Vec<IgnoreSet> = Vec::new();
    stack.push(IgnoreSet {
        base: String::new(),
        pats: builtin_patterns(),
    });
    if !extra_ignore.is_empty() {
        let pats: Vec<Pattern> = extra_ignore
            .iter()
            .filter_map(|e| {
                let g = e.trim().trim_matches('/').to_string();
                if g.is_empty() {
                    None
                } else {
                    Some(Pattern {
                        negated: false,
                        dir_only: false,
                        anchored: false,
                        glob: g,
                    })
                }
            })
            .collect();
        if !pats.is_empty() {
            stack.push(IgnoreSet {
                base: String::new(),
                pats,
            });
        }
    }
    if let Some(pats) = parse_gitignore_file("", &root.join(".gitignore")) {
        stack.push(IgnoreSet {
            base: String::new(),
            pats,
        });
    }

    let mut files = Vec::new();
    let mut capped = false;
    walk_dir(root, "", &mut stack, &mut files, &mut capped);
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    WalkResult { files, capped }
}

fn walk_dir(
    dir: &Path,
    rel: &str,
    stack: &mut Vec<IgnoreSet>,
    out: &mut Vec<WalkedFile>,
    capped: &mut bool,
) {
    if out.len() >= MAX_FILES {
        *capped = true;
        return;
    }
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for ent in entries {
        if out.len() >= MAX_FILES {
            *capped = true;
            return;
        }
        let name = ent.file_name().to_string_lossy().to_string();
        let ft = match ent.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ft.is_symlink() {
            continue; // never follow symlinks
        }
        let rel_path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", rel, name)
        };
        let is_dir = ft.is_dir();
        if is_ignored(&rel_path, is_dir, stack) {
            continue;
        }
        if is_dir {
            let mut pushed = false;
            if let Some(pats) = parse_gitignore_file(&rel_path, &ent.path().join(".gitignore")) {
                stack.push(IgnoreSet {
                    base: rel_path.clone(),
                    pats,
                });
                pushed = true;
            }
            walk_dir(&ent.path(), &rel_path, stack, out, capped);
            if pushed {
                stack.pop();
            }
        } else {
            let size = ent.metadata().map(|m| m.len()).unwrap_or(0);
            out.push(WalkedFile {
                rel: rel_path,
                size,
                ext: ext_of(&name),
            });
        }
    }
}

fn ext_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((_, e)) if !e.is_empty() => e.to_ascii_lowercase(),
        _ => String::new(),
    }
}

// ---- gitignore ----

fn parse_gitignore_file(base: &str, path: &Path) -> Option<Vec<Pattern>> {
    let src = fs::read_to_string(path).ok()?;
    Some(parse_gitignore(&src, base))
}

fn parse_gitignore(src: &str, _base: &str) -> Vec<Pattern> {
    let mut pats = Vec::new();
    for raw in src.lines() {
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut p = line.to_string();
        let negated = p.starts_with('!');
        if negated {
            p = p[1..].to_string();
        }
        let dir_only = p.ends_with('/');
        if dir_only {
            p.pop();
        }
        let anchored = if p.starts_with('/') {
            p.remove(0);
            true
        } else {
            p.contains('/')
        };
        if p.is_empty() {
            continue;
        }
        pats.push(Pattern {
            negated,
            dir_only,
            anchored,
            glob: p,
        });
    }
    pats
}

/// gitignore semantics: for each candidate path, every prefix (directory
/// component) is tested; the LAST matching pattern wins.
fn is_ignored(rel: &str, is_dir: bool, stack: &[IgnoreSet]) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    let mut decision = false;
    for set in stack {
        for pat in &set.pats {
            for k in 1..=parts.len() {
                let prefix = parts[..k].join("/");
                let prefix_is_dir = if k == parts.len() { is_dir } else { true };
                if pat.dir_only && !prefix_is_dir {
                    continue;
                }
                if pattern_matches(pat, &set.base, &prefix) {
                    decision = !pat.negated;
                }
            }
        }
    }
    decision
}

fn pattern_matches(pat: &Pattern, base: &str, target: &str) -> bool {
    let suffix = if base.is_empty() {
        target.to_string()
    } else {
        let bp = format!("{}/", base);
        match target.strip_prefix(&bp) {
            Some(s) => s.to_string(),
            None => return false,
        }
    };
    if pat.anchored {
        let pc: Vec<char> = pat.glob.chars().collect();
        let sc: Vec<char> = suffix.chars().collect();
        glob_match(&pc, &sc)
    } else {
        let segs: Vec<&str> = suffix.split('/').collect();
        let pc: Vec<char> = pat.glob.chars().collect();
        for i in 0..segs.len() {
            let cand: Vec<char> = segs[i..].join("/").chars().collect();
            if glob_match(&pc, &cand) {
                return true;
            }
        }
        false
    }
}

/// Glob matcher: `?` = one char (not `/`), `*` = run of chars (not `/`),
/// `**` = anything including `/`. Recursive with backtracking.
fn glob_match(p: &[char], t: &[char]) -> bool {
    if p.is_empty() {
        return t.is_empty();
    }
    match p[0] {
        '*' => {
            if p.len() > 1 && p[1] == '*' {
                let rest = if p.len() > 2 && p[2] == '/' {
                    &p[3..]
                } else {
                    &p[2..]
                };
                for i in 0..=t.len() {
                    if glob_match(rest, &t[i..]) {
                        return true;
                    }
                }
                false
            } else {
                for i in 0..=t.len() {
                    if i > 0 && t[i - 1] == '/' {
                        break;
                    }
                    if glob_match(&p[1..], &t[i..]) {
                        return true;
                    }
                }
                false
            }
        }
        '?' => !t.is_empty() && t[0] != '/' && glob_match(&p[1..], &t[1..]),
        c => !t.is_empty() && t[0] == c && glob_match(&p[1..], &t[1..]),
    }
}

fn pat(glob: &str, dir_only: bool) -> Pattern {
    Pattern {
        negated: false,
        dir_only,
        anchored: false,
        glob: glob.to_string(),
    }
}

/// Built-in excludes: dependency trees, build output, caches, junk files.
/// Everything here would be ignored by any sane paste anyway.
fn builtin_patterns() -> Vec<Pattern> {
    let dirs = [
        "node_modules",
        ".git",
        "target",
        "dist",
        "build",
        "out",
        "vendor",
        "venv",
        ".venv",
        "__pycache__",
        ".next",
        ".nuxt",
        ".output",
        ".turbo",
        ".cache",
        "coverage",
        ".idea",
        ".vscode",
        ".gradle",
        ".pytest_cache",
        ".mypy_cache",
        ".ruff_cache",
        ".sass-cache",
        "Pods",
        ".terraform",
        ".yarn",
        "bower_components",
        ".dart_tool",
        ".expo",
        ".angular",
        ".parcel-cache",
        ".pnpm-store",
        "cmake-build-debug",
        "CMakeFiles",
        ".git2",
        ".upm",
        ".tox",
        ".nox",
        "htmlcov",
        ".hypothesis",
        ".ipynb_checkpoints",
        ".rollup.cache",
    ];
    let files = [
        ".DS_Store",
        "Thumbs.db",
        "desktop.ini",
        "*.pyc",
        "*.pyo",
        "*.log",
        "*.tsbuildinfo",
        "*.class",
        "*.profraw",
    ];
    let mut pats: Vec<Pattern> = dirs.iter().map(|d| pat(d, true)).collect();
    pats.extend(files.iter().map(|f| pat(f, false)));
    pats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_basics() {
        let g = |p: &str, t: &str| {
            let pc: Vec<char> = p.chars().collect();
            let tc: Vec<char> = t.chars().collect();
            glob_match(&pc, &tc)
        };
        assert!(g("*.ts", "a.ts"));
        assert!(!g("*.ts", "a.tsx"));
        assert!(!g("*.ts", "src/a.ts")); // '*' does not cross '/'
        assert!(g("**/*.ts", "src/deep/a.ts"));
        assert!(g("src/**", "src/a/b"));
        assert!(g("build", "build"));
        assert!(g("a?c", "abc"));
        assert!(!g("a?c", "ac"));
    }

    #[test]
    fn gitignore_patterns() {
        let pats = parse_gitignore(
            "# comment\nnode_modules\ndist/\n!/keep\n/docs/legacy\n*.min.js\n",
            "",
        );
        assert_eq!(pats.len(), 5);
        assert!(pats[1].dir_only);
        assert!(pats[2].negated);
        assert!(pats[3].anchored);
    }
}
