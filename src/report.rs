//! Terminal + JSON reports. The `vp report` command is the hook that sells
//! the tool: it turns a repo into a token bill in under 100ms.

use crate::json::Json;
use crate::pipeline::{self, Options, Scan, Stats};
use crate::tokens;
use crate::util::{human_count, pct, thousands};

pub struct Style {
    pub color: bool,
}

impl Style {
    pub fn green(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[32m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
    pub fn bright(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[92m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
    pub fn amber(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[33m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
    pub fn dim(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[2m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
    pub fn bold(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[1m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
    pub fn cyan(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[36m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
    pub fn red(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[31m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
}

/// The `vp report` screen. Returns the full rendered string (no trailing
/// newline games — caller prints).
pub fn report(
    scan: &mut Scan,
    opts: &Options,
    projections: &[pipeline::Projection],
    wall_ms: u128,
    st: &Style,
) -> String {
    let mut o = String::new();
    let branch = scan
        .git
        .as_ref()
        .map(|g| g.branch.as_str())
        .unwrap_or("no-git");
    let lang_summary = if scan.project.langs.is_empty() {
        "unknown".to_string()
    } else {
        scan.project
            .langs
            .iter()
            .take(3)
            .map(|(l, c)| format!("{} ({})", l, c))
            .collect::<Vec<_>>()
            .join(" + ")
    };

    o.push('\n');
    o.push_str(&format!(
        "  {} {} · {}\n",
        st.bold("vibe-pilot"),
        st.dim(&format!("v{}", crate::VERSION)),
        st.dim("report")
    ));
    o.push_str(&format!(
        "  repo: {} ({}) · {}\n\n",
        st.cyan(&scan.name),
        st.dim(branch),
        st.dim(&lang_summary)
    ));

    let skipped: usize = scan
        .files
        .iter()
        .filter(|f| {
            matches!(
                f.kind,
                crate::profiler::Kind::Lockfile
                    | crate::profiler::Kind::Generated
                    | crate::profiler::Kind::Binary
                    | crate::profiler::Kind::Asset
                    | crate::profiler::Kind::Secret
            )
        })
        .count();
    let secrets: Vec<&str> = scan
        .files
        .iter()
        .filter(|f| f.kind == crate::profiler::Kind::Secret)
        .map(|f| f.rel.as_str())
        .collect();

    o.push_str(&format!(
        "  {}       {} files · {} LOC · {} noise files (lock/generated/binary)\n",
        st.bold("scan   "),
        thousands(scan.files.len()),
        human_count(scan.loc),
        skipped
    ));
    o.push_str(&format!(
        "  {}    {} tok est · {} / prompt · {}\n\n",
        st.bold("raw    "),
        st.bright(&thousands(scan.raw_tokens)),
        st.amber(&tokens::fmt_cost(tokens::cost(
            scan.raw_tokens,
            opts.price_in()
        ))),
        st.dim(&format!("{} @ ${}/MTok", opts.model, opts.price_in()))
    ));

    o.push_str(&format!("  {}\n", st.bold("profile projections")));
    let max_name = projections
        .iter()
        .map(|p| p.profile.len())
        .max()
        .unwrap_or(6);
    for p in projections {
        let active = p.profile == opts.profile.name;
        let name = if active {
            st.bright(&format!("> {}", p.profile))
        } else {
            format!("  {}", p.profile)
        };
        let bar = bar(p.reduction, 24, st);
        let tok_str = format!("{:>9}", thousands(p.tokens));
        o.push_str(&format!(
            "  {:<width$} {} tok {:>8}  {}  {}\n",
            name,
            tok_str,
            pct(p.reduction),
            bar,
            st.dim(&tokens::fmt_cost(p.cost)),
            width = max_name + 2
        ));
    }
    o.push('\n');

    // token sinks — the money shot
    o.push_str(&format!("  {}\n", st.bold("biggest token sinks (raw)")));
    let mut by_tokens: Vec<&crate::pipeline::FileAnalysis> = scan.files.iter().collect();
    by_tokens.sort_by_key(|f| std::cmp::Reverse(f.tokens_raw));
    let mut shown = 0;
    for f in by_tokens.iter().filter(|f| f.tokens_raw > 0).take(12) {
        shown += 1;
        let note = if f.selected {
            st.green(&format!("score {:.1} · included", f.score))
        } else {
            st.dim(f.reason)
        };
        o.push_str(&format!(
            "  {:>10} tok  {} · {}\n",
            thousands(f.tokens_raw),
            f.rel,
            note
        ));
    }
    if shown == 0 {
        o.push_str("  (nothing)\n");
    }
    o.push('\n');

    if !secrets.is_empty() {
        o.push_str(&format!(
            "  {} never read: {}\n",
            st.red("secrets"),
            secrets.join(", ")
        ));
    }
    if scan.capped {
        o.push_str(&format!(
            "  {} file scan capped at {} files — check .gitignore\n",
            st.amber("note"),
            thousands(60_000)
        ));
    }
    o.push_str(&format!(
        "  {} {:.0}{}ms · zero dependencies · prices: Oct 2026\n\n",
        st.dim("wall"),
        wall_ms as f64 / 1.0,
        st.dim("")
    ));
    o
}

fn bar(reduction: f64, width: usize, st: &Style) -> String {
    let filled = ((reduction.clamp(0.0, 1.0)) * width as f64).round() as usize;
    let mut s = String::from("[");
    for i in 0..width {
        if i < filled {
            s.push('#');
        } else {
            s.push(' ');
        }
    }
    s.push(']');
    st.green(&s)
}

/// Machine-readable report.
pub fn report_json(
    scan: &Scan,
    opts: &Options,
    projections: &[pipeline::Projection],
    wall_ms: u128,
) -> Json {
    let model_price = opts.price_in();
    Json::obj(vec![
        ("tool", Json::s("vibe-pilot")),
        ("version", Json::s(crate::VERSION)),
        ("repo", Json::s(&scan.name)),
        (
            "git",
            match &scan.git {
                Some(g) => Json::obj(vec![
                    ("branch", Json::s(&g.branch)),
                    ("changed", Json::n(g.changed.len() as f64)),
                ]),
                None => Json::Null,
            },
        ),
        (
            "project",
            Json::obj(vec![
                ("primary_language", Json::s(&scan.project.primary)),
                (
                    "languages",
                    Json::arr(
                        scan.project
                            .langs
                            .iter()
                            .map(|(l, c)| {
                                Json::obj(vec![("lang", Json::s(l)), ("files", Json::n(*c as f64))])
                            })
                            .collect(),
                    ),
                ),
                (
                    "entries",
                    Json::arr(scan.project.entries.iter().map(|e| Json::s(e)).collect()),
                ),
            ]),
        ),
        (
            "scan",
            Json::obj(vec![
                ("files", Json::n(scan.files.len() as f64)),
                ("loc", Json::n(scan.loc as f64)),
                ("raw_tokens", Json::n(scan.raw_tokens as f64)),
                (
                    "raw_cost_per_prompt",
                    Json::n(tokens::cost(scan.raw_tokens, model_price)),
                ),
            ]),
        ),
        (
            "projections",
            Json::arr(
                projections
                    .iter()
                    .map(|p| {
                        Json::obj(vec![
                            ("profile", Json::s(p.profile)),
                            ("tokens", Json::n(p.tokens as f64)),
                            ("reduction", Json::n(p.reduction)),
                            ("cost_per_prompt", Json::n(p.cost)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("model", Json::s(&opts.model)),
        ("price_per_mtok", Json::n(model_price)),
        ("wall_ms", Json::n(wall_ms as f64)),
    ])
}

/// Stderr one-liner after writing .context.
pub fn emit_summary(path: &str, stats: &Stats, st: &Style) -> String {
    format!(
        "  {} {} · {}/{} files · {} tok (raw {} · {}) · {} -> {} per prompt\n",
        st.green("wrote"),
        st.bold(path),
        stats.files_selected,
        thousands(stats.files_total),
        st.bright(&thousands(stats.out_tokens)),
        thousands(stats.raw_tokens),
        st.green(&pct(stats.reduction())),
        st.green(&tokens::fmt_cost(stats.cost_out)),
        st.dim(&tokens::fmt_cost(stats.cost_raw)),
    )
}

pub fn profiles_table(st: &Style) -> String {
    let mut o = String::new();
    o.push('\n');
    o.push_str(&format!("  {}\n\n", st.bold("built-in profiles")));
    for p in crate::config::all() {
        let budget = if p.max_tokens == usize::MAX {
            "no cap".to_string()
        } else {
            format!("{} tok", thousands(p.max_tokens))
        };
        o.push_str(&format!(
            "  {:<10} {:>10} · comments {} · tests {} · docs {}\n",
            st.bright(p.name),
            budget,
            if p.strip_comments { "stripped" } else { "kept" },
            if p.include_tests { "yes" } else { "no" },
            if p.include_docs { "yes" } else { "no" }
        ));
        o.push_str(&format!("  {:<10} {}\n", "", st.dim(p.desc)));
    }
    o.push_str(&format!(
        "\n  {} override any of them in vibe-pilot.toml — see `vp init`\n\n",
        st.dim("tip:")
    ));
    o
}
