//! Ranker: which files matter RIGHT NOW.
//!
//! score = 3.0*task-relevance + 2.0*git-recency + 1.5*import-centrality
//!       + 1.0*path-proximity + bonuses - kind penalties
//!
//! The formula is deliberately simple, documented, and cheap — the whole
//! point is a fast, explainable signal, not a research project.

use std::collections::HashMap;

use crate::pipeline::FileAnalysis;
use crate::profiler::{Kind, ProjectInfo};

const W_TASK: f64 = 3.0;
const W_RECENCY: f64 = 2.0;
const W_CENTRALITY: f64 = 1.5;
const W_PROXIMITY: f64 = 1.0;

const BONUS_ENTRY: f64 = 1.2;
const BONUS_RULES: f64 = 1.5;
const BONUS_MANIFEST: f64 = 1.0;
const BONUS_README: f64 = 0.8;
const BONUS_ENV_EXAMPLE: f64 = 0.5;

/// Words that match half the files in any repo — pure ranking noise.
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "that", "this", "into", "your", "you", "are", "was",
    "not", "but", "all", "can", "will", "just", "should", "now", "then", "than", "when", "what",
    "where", "how", "why", "who", "its", "his", "her", "our", "their", "has", "have", "had",
    "been", "being", "does", "did", "using", "use", "used", "new", "old", "add", "set", "get",
    "make", "made", "some", "any", "out", "off", "put",
];

/// Split a task description into lowercase search terms.
pub fn task_terms(task: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    for raw in task.split(|c: char| !c.is_ascii_alphanumeric()) {
        let t = raw.to_ascii_lowercase();
        if t.len() >= 3 && !STOPWORDS.contains(&t.as_str()) && !terms.contains(&t) {
            terms.push(t);
        }
    }
    // also add snake/kebab pieces (login_bug -> login, bug)
    let extras: Vec<String> = task
        .split(['_', '-'])
        .filter(|s| s.len() >= 3)
        .map(|s| s.to_ascii_lowercase())
        .collect();
    for e in extras {
        if !terms.contains(&e) && !STOPWORDS.contains(&e.as_str()) {
            terms.push(e);
        }
    }
    terms
}

/// Cheap import-graph centrality: how many files import/reference each stem.
pub fn build_centrality(files: &[FileAnalysis]) -> HashMap<String, f64> {
    let mut indegree: HashMap<String, u32> = HashMap::new();
    for f in files {
        if !matches!(f.kind, Kind::Source | Kind::Test) {
            continue;
        }
        let sample: String = f.raw.chars().take(24_576).collect(); // cap work
        for line in sample.lines() {
            let t = line.trim_start();
            if t.is_empty() || t.len() > 300 {
                continue;
            }
            // js/ts: from "..." / require("...")
            for marker in [
                "from \"",
                "from '",
                "require(\"",
                "require('",
                "import(\"",
                "import('",
            ] {
                if let Some(pos) = t.find(marker) {
                    let rest = &t[pos + marker.len()..];
                    if let Some(endq) = rest.find(['"', '\'']) {
                        let path = &rest[..endq];
                        if path.starts_with('.') || path.starts_with('/') {
                            if let Some(stem) = path_stem(path) {
                                *indegree.entry(stem).or_insert(0) += 1;
                            }
                        }
                        break;
                    }
                }
            }
            // rust: use crate::x::y / mod x;
            if t.starts_with("use crate::") || t.starts_with("use super::") {
                let tail = t.trim_end_matches(';');
                if let Some(last) = tail.rsplit("::").next() {
                    let s = last.trim();
                    if s.len() >= 2 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        *indegree.entry(s.to_ascii_lowercase()).or_insert(0) += 1;
                    }
                }
            } else if t.starts_with("mod ") {
                let tail = t.trim_end_matches(';');
                if let Some(name) = tail.strip_prefix("mod ") {
                    let s = name.trim();
                    if s.len() >= 2 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        *indegree.entry(s.to_ascii_lowercase()).or_insert(0) += 1;
                    }
                }
            }
            // python: from x import y / from .x import y
            if t.starts_with("from ") && t.contains(" import ") {
                let mid = t.strip_prefix("from ").unwrap_or("");
                let module = mid.split(" import ").next().unwrap_or("").trim();
                if !module.is_empty() {
                    let last = module.rsplit('.').next().unwrap_or(module);
                    if last.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                        && !last.is_empty()
                    {
                        *indegree.entry(last.to_ascii_lowercase()).or_insert(0) += 1;
                    }
                }
            }
            // c: #include "x.h"
            if t.starts_with("#include \"") {
                let path = t.trim_start_matches("#include \"").trim_end_matches('"');
                if let Some(stem) = path_stem(path) {
                    *indegree.entry(stem).or_insert(0) += 1;
                }
            }
        }
    }
    indegree
        .into_iter()
        .map(|(k, v)| (k, (v as f64 / 8.0).min(1.0)))
        .collect()
}

fn path_stem(path: &str) -> Option<String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = match name.rsplit_once('.') {
        Some((s, _)) if !s.is_empty() => s,
        _ => name,
    };
    let stem = stem.to_ascii_lowercase();
    if stem.len() >= 2 {
        Some(stem)
    } else {
        None
    }
}

/// Score every file in place. `recency` from git (missing -> 0.5),
/// `centrality` from `build_centrality`.
#[allow(clippy::too_many_arguments)]
pub fn score(
    files: &mut [FileAnalysis],
    task: Option<&str>,
    project: &ProjectInfo,
    recency: &HashMap<String, f64>,
    centrality: &HashMap<String, f64>,
    include_tests: bool,
    include_docs: bool,
) {
    let terms = task.map(task_terms).unwrap_or_default();
    let entry_dirs: Vec<String> = project
        .entries
        .iter()
        .filter_map(|e| e.rsplit_once('/').map(|(d, _)| d.to_string()))
        .collect();

    for f in files.iter_mut() {
        // map-only kinds always score zero
        if matches!(
            f.kind,
            Kind::Lockfile | Kind::Generated | Kind::Binary | Kind::Asset | Kind::Secret
        ) {
            f.score = 0.0;
            continue;
        }

        // task relevance
        let mut task_score = 0.0;
        if !terms.is_empty() {
            let path_l = f.rel.to_ascii_lowercase();
            let sample: String = f
                .raw
                .chars()
                .take(4096)
                .collect::<String>()
                .to_ascii_lowercase();
            let mut hit = 0.0;
            for t in &terms {
                if path_l.contains(t.as_str()) {
                    hit += 1.0;
                } else if sample.contains(t.as_str()) {
                    hit += 0.5;
                }
            }
            task_score = hit / terms.len() as f64;
        }
        f.task_hits = task_score;

        let rec = *recency.get(&f.rel).unwrap_or(&0.5);
        f.recency = rec;

        let stem = f
            .rel
            .rsplit('/')
            .next()
            .unwrap_or(&f.rel)
            .rsplit_once('.')
            .map(|(s, _)| s.to_ascii_lowercase())
            .unwrap_or_default();
        let centr = centrality.get(&stem).copied().unwrap_or(0.0);

        // proximity to where the action is
        let dir = f.rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let prox = if entry_dirs.iter().any(|d| d == dir) {
            1.0
        } else if entry_dirs
            .iter()
            .any(|d| f.rel.starts_with(&format!("{}/", d)))
        {
            0.85
        } else if dir == "src" || dir == "lib" || dir.is_empty() {
            0.7
        } else if dir.matches('/').count() == 0 {
            0.5
        } else {
            0.3
        };

        let mut score =
            W_TASK * task_score + W_RECENCY * rec + W_CENTRALITY * centr + W_PROXIMITY * prox;

        // bonuses
        if project.entries.iter().any(|e| e == &f.rel) {
            score += BONUS_ENTRY;
        }
        if f.kind == Kind::Rules {
            score += BONUS_RULES;
        }
        if project.markers.iter().any(|m| m == &f.rel) {
            score += BONUS_MANIFEST;
        }
        let name_l = f.rel.to_ascii_lowercase();
        if name_l == "readme.md" || name_l == "readme" {
            score += BONUS_README;
        }
        if name_l.ends_with(".env.example") || name_l.ends_with(".env.sample") {
            score += BONUS_ENV_EXAMPLE;
        }

        // kind multipliers
        if f.kind == Kind::Test && !include_tests {
            score *= 0.5;
        }
        if f.kind == Kind::Doc && !include_docs {
            score *= 0.4;
        }
        if f.kind == Kind::Data {
            score *= 0.5;
        }
        if f.kind == Kind::Config {
            score *= 0.9;
        }

        f.score = score;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_split() {
        let t = task_terms("Fix the LOGIN bug in auth!");
        assert!(t.contains(&"fix".to_string()));
        assert!(t.contains(&"login".to_string()));
        assert!(t.contains(&"auth".to_string()));
        assert!(!t.contains(&"the".to_string())); // too short
    }
}
