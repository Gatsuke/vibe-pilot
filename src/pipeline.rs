//! Pipeline: scan -> classify -> rank -> select -> emit.
//! This is the orchestrator every command (and the MCP server) calls.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::config::{self, Profile};
use crate::git::{self, GitInfo};
use crate::profiler::{self, Kind, ProjectInfo};
use crate::ranker;
use crate::tokens;
use crate::walker;

/// Hard cap: files above 4 MB are never read, only counted.
const MAX_READ_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct FileAnalysis {
    pub rel: String,
    pub size: u64,
    pub ext: String,
    pub lang: String,
    pub kind: Kind,
    pub score: f64,
    pub recency: f64,
    pub task_hits: f64,
    pub raw: String,
    pub tokens_raw: usize,
    pub kept: Option<String>,     // pruned, comments kept (lazy cache)
    pub stripped: Option<String>, // pruned, comments stripped (lazy cache)
    pub selected: bool,
    pub reason: &'static str,
}

impl FileAnalysis {
    fn ensure_pruned(&mut self, strip: bool) {
        let lang = self.lang.clone();
        if strip && self.stripped.is_none() {
            self.stripped = Some(crate::pruner::prune(&self.raw, &lang, true, 400));
        }
        if !strip && self.kept.is_none() {
            self.kept = Some(crate::pruner::prune(&self.raw, &lang, false, 400));
        }
    }

    pub fn pruned_content(&mut self, strip: bool) -> String {
        self.ensure_pruned(strip);
        if strip {
            self.stripped.clone().unwrap_or_default()
        } else {
            self.kept.clone().unwrap_or_default()
        }
    }

    pub fn tokens_pruned(&mut self, strip: bool) -> usize {
        self.ensure_pruned(strip);
        let s = if strip {
            self.stripped.as_deref()
        } else {
            self.kept.as_deref()
        };
        tokens::estimate(s.unwrap_or(""))
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub profile: Profile,
    pub task: Option<String>,
    pub diff_only: bool,
    pub max_tokens: Option<usize>,
    pub strip_comments: Option<bool>,
    pub ignore: Vec<String>,
    pub model: String,
    pub price_override: Option<f64>,
}

impl Options {
    pub fn strip(&self) -> bool {
        self.strip_comments.unwrap_or(self.profile.strip_comments)
    }
    pub fn budget(&self) -> usize {
        self.max_tokens.unwrap_or(self.profile.max_tokens)
    }
    pub fn price_in(&self) -> f64 {
        self.price_override
            .or_else(|| tokens::find(&self.model).map(|m| m.input))
            .unwrap_or(2.0)
    }
}

#[derive(Debug)]
pub struct Scan {
    pub root: PathBuf,
    pub name: String,
    pub project: ProjectInfo,
    pub git: Option<GitInfo>,
    pub files: Vec<FileAnalysis>,
    pub raw_tokens: usize,
    pub loc: usize,
    pub capped: bool,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub files_total: usize,
    pub files_selected: usize,
    pub raw_tokens: usize,
    pub out_tokens: usize,
    pub model: String,
    pub price_in: f64,
    pub cost_raw: f64,
    pub cost_out: f64,
}

impl Stats {
    pub fn reduction(&self) -> f64 {
        if self.raw_tokens == 0 {
            0.0
        } else {
            1.0 - self.out_tokens as f64 / self.raw_tokens as f64
        }
    }
}

#[derive(Debug)]
pub struct Emitted {
    pub content: String,
    pub stats: Stats,
}

/// Full scan of a repo. Never panics on IO problems — unreadable things are
/// simply counted, not included.
pub fn scan(root: &Path, opts: &Options) -> Scan {
    let walked = walker::walk(root, &opts.ignore);
    let mut files: Vec<FileAnalysis> = Vec::with_capacity(walked.files.len());

    for wf in walked.files {
        let lang = profiler::lang_of(&wf.ext).to_string();
        let secret = profiler::is_secret_name(&wf.rel);
        let mut f = FileAnalysis {
            rel: wf.rel,
            size: wf.size,
            ext: wf.ext,
            lang: lang.clone(),
            kind: Kind::Other,
            score: 0.0,
            recency: 0.5,
            task_hits: 0.0,
            raw: String::new(),
            tokens_raw: 0,
            kept: None,
            stripped: None,
            selected: false,
            reason: "included",
        };

        if secret {
            f.kind = Kind::Secret;
            f.reason = "secret - never read";
            f.tokens_raw = (f.size as f64 / tokens::CHARS_PER_TOKEN) as usize;
            files.push(f);
            continue;
        }

        if wf.size > MAX_READ_BYTES {
            f.kind = profiler::classify(&f.rel, &f.ext, None);
            f.reason = if matches!(
                f.kind,
                Kind::Lockfile | Kind::Generated | Kind::Binary | Kind::Asset
            ) {
                kind_reason(f.kind)
            } else {
                "oversize - map only"
            };
            f.tokens_raw = (f.size as f64 / tokens::CHARS_PER_TOKEN) as usize;
            files.push(f);
            continue;
        }

        match std::fs::read(root.join(&f.rel)) {
            Ok(bytes) => {
                if bytes.contains(&0u8) {
                    f.kind = Kind::Binary;
                    f.reason = kind_reason(Kind::Binary);
                    f.tokens_raw = (f.size as f64 / tokens::CHARS_PER_TOKEN) as usize;
                    files.push(f);
                    continue;
                }
                let text = String::from_utf8(bytes).unwrap_or_default();
                let head: String = text.chars().take(2048).collect();
                f.kind = profiler::classify(&f.rel, &f.ext, Some(&head));
                f.tokens_raw = tokens::estimate(&text);
                f.reason = kind_reason(f.kind);
                f.raw = text;
            }
            Err(_) => {
                f.kind = profiler::classify(&f.rel, &f.ext, None);
                f.reason = "unreadable - map only";
                f.tokens_raw = (f.size as f64 / tokens::CHARS_PER_TOKEN) as usize;
            }
        }
        files.push(f);
    }

    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string());
    let project = profiler::analyze(&name, &files, |f| &f.rel, |f| f.kind, |f| &f.lang);
    let git_info = git::probe(root);
    let recency = git::recency(root);
    let centrality = ranker::build_centrality(&files);

    ranker::score(
        &mut files,
        opts.task.as_deref(),
        &project,
        &recency,
        &centrality,
        opts.profile.include_tests,
        opts.profile.include_docs,
    );

    let raw_tokens: usize = files.iter().map(|f| f.tokens_raw + 10).sum();
    let loc: usize = files.iter().map(|f| f.raw.lines().count()).sum();

    Scan {
        root: root.to_path_buf(),
        name,
        project,
        git: git_info,
        files,
        raw_tokens,
        loc,
        capped: walked.capped,
    }
}

fn kind_reason(k: Kind) -> &'static str {
    match k {
        Kind::Lockfile => "lockfile - always skipped",
        Kind::Generated => "generated - map only",
        Kind::Binary => "binary - skipped",
        Kind::Asset => "asset - skipped",
        Kind::Secret => "secret - never read",
        _ => "included",
    }
}

fn eligible(f: &FileAnalysis, opts: &Options) -> bool {
    match f.kind {
        Kind::Source | Kind::Rules | Kind::Config => true,
        Kind::Test => opts.profile.include_tests,
        Kind::Doc => opts.profile.include_docs,
        Kind::Data => f.tokens_raw < 20_000,
        Kind::Other => f.tokens_raw < 8_000,
        _ => false,
    }
}

/// Select + assemble the `.context` output.
pub fn emit(scan: &mut Scan, opts: &Options) -> Emitted {
    let strip = opts.strip();
    let budget = opts.budget();

    let changed: Option<HashSet<String>> = if opts.diff_only {
        scan.git
            .as_ref()
            .map(|g| g.changed.iter().map(|c| c.rel.clone()).collect())
    } else {
        None
    };

    // map overhead estimate
    let mut used = scan.files.len().min(1_500) * 12 + 400;

    let mut order: Vec<usize> = (0..scan.files.len())
        .filter(|&i| {
            let f = &scan.files[i];
            if f.size > opts.profile.max_file_bytes && f.kind != Kind::Rules {
                return false;
            }
            if let Some(ch) = &changed {
                if !ch.contains(&f.rel)
                    && f.kind != Kind::Rules
                    && !scan.project.markers.iter().any(|m| m == &f.rel)
                {
                    return false;
                }
            }
            eligible(f, opts)
        })
        .collect();
    order.sort_by(|&a, &b| {
        scan.files[b]
            .score
            .partial_cmp(&scan.files[a].score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(scan.files[a].tokens_raw.cmp(&scan.files[b].tokens_raw))
    });

    let forced: HashSet<String> = {
        let mut s: HashSet<String> = scan.project.rules.iter().cloned().collect();
        for m in &scan.project.markers {
            if m.rsplit('/')
                .next()
                .map(config_builtin_manifest)
                .unwrap_or(false)
            {
                s.insert(m.clone());
            }
        }
        if !opts.diff_only {
            for e in &scan.project.entries {
                s.insert(e.clone());
            }
        }
        s
    };

    let mut selected_idx: Vec<usize> = Vec::new();
    for &i in &order {
        let f = &mut scan.files[i];
        let t = f.tokens_pruned(strip) + tokens::FILE_OVERHEAD_TOK;
        let is_forced = forced.contains(&f.rel);
        if is_forced || used + t <= budget {
            f.selected = true;
            f.reason = "included";
            used += t;
            selected_idx.push(i);
        } else {
            f.reason = "budget";
        }
    }

    let out_tokens: usize = selected_idx
        .iter()
        .map(|&i| scan.files[i].tokens_pruned(strip) + tokens::FILE_OVERHEAD_TOK)
        .sum();
    let files_selected = selected_idx.len();

    let stats = Stats {
        files_total: scan.files.len(),
        files_selected,
        raw_tokens: scan.raw_tokens,
        out_tokens,
        model: opts.model.clone(),
        price_in: opts.price_in(),
        cost_raw: tokens::cost(scan.raw_tokens, opts.price_in()),
        cost_out: tokens::cost(out_tokens, opts.price_in()),
    };

    let content = crate::context::render(scan, opts, &stats);
    Emitted { content, stats }
}

fn config_builtin_manifest(name: &str) -> bool {
    matches!(
        name,
        "package.json"
            | "cargo.toml"
            | "go.mod"
            | "pyproject.toml"
            | "setup.py"
            | "gemfile"
            | "composer.json"
            | "mix.exs"
            | "pom.xml"
    )
}

/// Projection of tokens per profile for the report command.
pub struct Projection {
    pub profile: &'static str,
    pub tokens: usize,
    pub reduction: f64,
    pub cost: f64,
}

pub fn project_stats(scan: &mut Scan, opts: &Options) -> Vec<Projection> {
    let mut out = Vec::new();
    for p in config::all() {
        let mut o = opts.clone();
        o.profile = p.clone();
        o.max_tokens = None;
        o.task = None;
        o.diff_only = false;
        let e = emit(scan, &o);
        // reset selection flags and reasons for the next round
        for f in scan.files.iter_mut() {
            f.selected = false;
            f.reason = kind_reason(f.kind);
        }
        out.push(Projection {
            profile: p.name,
            tokens: e.stats.out_tokens,
            reduction: e.stats.reduction(),
            cost: e.stats.cost_out,
        });
    }
    out
}
