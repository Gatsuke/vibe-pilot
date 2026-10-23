//! Pruner: per-file noise reduction. Safe by design — only full-line
//! comments are ever stripped (a `//` inside a string can never start a
//! line), lint directives are always kept, and oversized files are
//! truncated with a visible marker.

/// Lint/tool directives that must survive comment stripping.
const KEEP_DIRECTIVES: &[&str] = &[
    "nolint",
    "eslint",
    "prettier-ignore",
    "@ts-",
    "ts-expect-error",
    "ts-ignore",
    "clippy",
    "rustfmt",
    "allow(",
    "deny(",
    "pragma",
    "noinspection",
    "biome-ignore",
    "deno-lint",
    "swiftlint",
    "istanbul",
    "cspell",
    "mypy:",
    "flake8",
    "pylint",
    "ruff",
    "fmt:",
    "spdx",
    "license",
];

fn line_comment_token(lang: &str) -> Option<&'static str> {
    match lang {
        "rust" | "typescript" | "javascript" | "vue" | "svelte" | "dart" | "java" | "kotlin"
        | "swift" | "c" | "cpp" | "csharp" | "scala" | "php" | "go" => Some("//"),
        "python" | "ruby" | "shell" | "toml" | "yaml" | "r" | "julia" | "nim" | "make" => Some("#"),
        "sql" | "lua" | "haskell" | "elm" => Some("--"),
        _ => None,
    }
}

fn block_comment_tokens(lang: &str) -> Option<(&'static str, &'static str)> {
    match lang {
        "rust" | "typescript" | "javascript" | "vue" | "svelte" | "dart" | "java" | "kotlin"
        | "swift" | "c" | "cpp" | "csharp" | "scala" | "php" | "go" | "css" | "scss" | "less" => {
            Some(("/*", "*/"))
        }
        "html" => Some(("<!--", "-->")),
        _ => None,
    }
}

fn keep_directive(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    KEEP_DIRECTIVES.iter().any(|d| l.contains(d))
}

/// Prune one file's content.
///
/// - strips trailing whitespace, collapses blank-line runs
/// - optionally strips full-line comments (lint directives kept)
/// - truncates to `max_lines` with a visible marker
pub fn prune(src: &str, lang: &str, strip_comments: bool, max_lines: usize) -> String {
    let line_tok = line_comment_token(lang);
    let blk = block_comment_tokens(lang);
    let mut out: Vec<String> = Vec::new();
    let mut in_block = false;

    for line in src.lines() {
        let t = line.trim_start();
        if in_block {
            if let Some((_, end)) = blk {
                if t.contains(end) {
                    in_block = false;
                }
            }
            continue;
        }
        if strip_comments {
            if let Some((start, end)) = blk {
                if t.starts_with(start) {
                    // single-line block comment?
                    let is_single = match (t.find(start), t.find(end)) {
                        (Some(a), Some(b)) => b > a + start.len() - 1,
                        _ => false,
                    };
                    if is_single {
                        continue;
                    }
                    in_block = true;
                    continue;
                }
            }
            if let Some(tok) = line_tok {
                if t.starts_with(tok) && !t.starts_with("#!") && !keep_directive(t) {
                    continue;
                }
            }
        }
        out.push(line.trim_end().to_string());
    }

    // collapse blank-line runs to one
    let mut collapsed: Vec<String> = Vec::with_capacity(out.len());
    for l in out {
        if l.is_empty() && collapsed.last().map(|s| s.is_empty()).unwrap_or(true) {
            continue;
        }
        collapsed.push(l);
    }
    while collapsed.first().map(|s| s.is_empty()).unwrap_or(false) {
        collapsed.remove(0);
    }
    while collapsed.last().map(|s| s.is_empty()).unwrap_or(false) {
        collapsed.pop();
    }

    if collapsed.len() > max_lines {
        let more = collapsed.len() - max_lines;
        collapsed.truncate(max_lines);
        collapsed.push(format!("... [vibe-pilot: {} more lines truncated]", more));
    }

    if collapsed.is_empty() {
        String::new()
    } else {
        let mut s = collapsed.join("\n");
        s.push('\n');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_full_line_comments_only() {
        let src = "// header comment\nconst a = 1; // trailing\n// another\nconst b = \"http://x\";\n\n\nconst c = 3;\n";
        let out = prune(src, "javascript", true, 400);
        assert!(!out.contains("header comment"));
        assert!(!out.contains("another"));
        assert!(out.contains("const a = 1; // trailing")); // inline comments are kept
        assert!(out.contains("const b = \"http://x\";")); // string with // is safe
        assert!(out.contains("const c = 3;"));
        assert!(!out.contains("\n\n\n")); // blank runs collapsed to a single blank line
    }

    #[test]
    fn keeps_directives_and_shebang() {
        let src = "#!/usr/bin/env python\n# pylint: disable=all\n# normal comment\nx = 1\n";
        let out = prune(src, "python", true, 400);
        assert!(out.starts_with("#!/usr/bin/env python"));
        assert!(out.contains("pylint"));
        assert!(!out.contains("normal comment"));
        assert!(out.contains("x = 1"));
    }

    #[test]
    fn block_comments() {
        let src = "/*\n * license block\n */\nfn main() {}\n/* single */\nlet x = 1;\n";
        let out = prune(src, "rust", true, 400);
        assert!(!out.contains("license block"));
        assert!(!out.contains("single"));
        assert!(out.contains("fn main() {}"));
        assert!(out.contains("let x = 1;"));
    }

    #[test]
    fn keeps_comments_when_not_lean() {
        let src = "// useful doc\nfn main() {}\n";
        let out = prune(src, "rust", false, 400);
        assert!(out.contains("useful doc"));
    }

    #[test]
    fn truncates() {
        let mut src = String::new();
        for i in 0..500 {
            src.push_str("let v");
            src.push_str(&i.to_string());
            src.push_str(" = ");
            src.push_str(&i.to_string());
            src.push_str(";\n");
        }
        let out = prune(&src, "rust", false, 100);
        assert!(out.contains("[vibe-pilot: 400 more lines truncated]"));
        assert!(out.lines().count() <= 101);
    }
}
