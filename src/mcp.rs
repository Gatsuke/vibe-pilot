//! MCP (Model Context Protocol) server over stdio — newline-delimited
//! JSON-RPC 2.0. Lets Claude Code / Cursor agents call vibe-pilot directly:
//!
//!   claude mcp add vibe-pilot -- vp mcp
//!
//! Tools exposed: vp_report, vp_prune, vp_context, vp_diff.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use crate::json::Json;
use crate::pipeline::{self, Options};
use crate::report::Style;

const PROTOCOL_VERSION: &str = "2025-06-18";

pub fn run() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut line = String::new();
    let mut reader = stdin.lock();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF — client gone
            Ok(_) => {}
            Err(_) => break,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut buf = String::new();
        handle_message(trimmed, &mut buf);
        if !buf.is_empty() {
            let _ = out.write_all(buf.as_bytes());
            let _ = out.flush();
        }
    }
}

fn handle_message(line: &str, out: &mut String) {
    let msg = match Json::parse(line) {
        Ok(m) => m,
        Err(_) => return, // ignore malformed lines per JSON-RPC over stdio convention
    };
    let id = msg.get("id").cloned();
    let method = msg
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    let params = msg.get("params").cloned().unwrap_or(Json::Obj(vec![]));

    // notifications get no response
    let id = match id {
        Some(i) => i,
        None => return,
    };

    let result = match method.as_str() {
        "initialize" => Ok(initialize_result()),
        "notifications/initialized" => return,
        "ping" => Ok(Json::Obj(vec![])),
        "tools/list" => Ok(tools_list()),
        "tools/call" => call_tool(&params),
        _ => Err((-32601, "method not found".to_string())),
    };

    let resp = match result {
        Ok(r) => Json::obj(vec![("jsonrpc", Json::s("2.0")), ("id", id), ("result", r)]).dump(),
        Err((code, message)) => Json::obj(vec![
            ("jsonrpc", Json::s("2.0")),
            ("id", id),
            (
                "error",
                Json::obj(vec![
                    ("code", Json::n(code as f64)),
                    ("message", Json::s(&message)),
                ]),
            ),
        ])
        .dump(),
    };
    out.push_str(&resp);
    out.push('\n');
}

fn initialize_result() -> Json {
    Json::obj(vec![
        ("protocolVersion", Json::s(PROTOCOL_VERSION)),
        (
            "capabilities",
            Json::obj(vec![("tools", Json::obj(vec![]))]),
        ),
        (
            "serverInfo",
            Json::obj(vec![
                ("name", Json::s("vibe-pilot")),
                ("version", Json::s(crate::VERSION)),
            ]),
        ),
    ])
}

fn schema_str(desc: &str) -> Json {
    Json::obj(vec![
        ("type", Json::s("string")),
        ("description", Json::s(desc)),
    ])
}

fn schema_num(desc: &str) -> Json {
    Json::obj(vec![
        ("type", Json::s("number")),
        ("description", Json::s(desc)),
    ])
}

fn tool(name: &str, desc: &str, props: Vec<(&str, Json)>, required: &[&str]) -> Json {
    Json::obj(vec![
        ("name", Json::s(name)),
        ("description", Json::s(desc)),
        (
            "inputSchema",
            Json::obj(vec![
                ("type", Json::s("object")),
                ("properties", Json::obj(props)),
                (
                    "required",
                    Json::arr(required.iter().map(|r| Json::s(r)).collect()),
                ),
            ]),
        ),
    ])
}

fn tools_list() -> Json {
    Json::obj(vec![(
        "tools",
        Json::arr(vec![
            tool(
                "vp_report",
                "Analyze the repository: token counts, cost per prompt, per-profile projections, biggest token sinks. Read-only, fast.",
                vec![(
                    "path",
                    schema_str("Repository path (default: current directory)."),
                )],
                &[],
            ),
            tool(
                "vp_prune",
                "Generate a pruned .context file for this repository and return its content. The context includes a full file map with relevance scores plus the highest-ranked file bodies.",
                vec![
                    ("path", schema_str("Repository path (default: current directory).")),
                    (
                        "profile",
                        schema_str("Budget profile: lean | cursor | windsurf | aider | claude | full (default: claude)."),
                    ),
                    ("task", schema_str("Optional task description to focus file ranking.")),
                    ("maxTokens", schema_num("Hard token budget override.")),
                ],
                &[],
            ),
            tool(
                "vp_context",
                "Generate a task-focused .context: ranks files by relevance to the task, includes the best matches with full bodies plus the repo map.",
                vec![
                    ("task", schema_str("What you are about to build or fix, in plain words.")),
                    ("path", schema_str("Repository path (default: current directory).")),
                    (
                        "profile",
                        schema_str("Budget profile (default: claude)."),
                    ),
                ],
                &["task"],
            ),
            tool(
                "vp_diff",
                "Generate a .context containing only what changed vs HEAD (modified + staged + untracked files), plus the repo map. Use between prompts so the agent only sees fresh work.",
                vec![
                    ("path", schema_str("Repository path (default: current directory).")),
                    (
                        "profile",
                        schema_str("Budget profile (default: claude)."),
                    ),
                ],
                &[],
            ),
        ]),
    )])
}

fn text_result(text: String) -> Json {
    Json::obj(vec![(
        "content",
        Json::arr(vec![Json::obj(vec![
            ("type", Json::s("text")),
            ("text", Json::Str(text)),
        ])]),
    )])
}

fn error_result(message: String) -> Json {
    Json::obj(vec![
        (
            "content",
            Json::arr(vec![Json::obj(vec![
                ("type", Json::s("text")),
                ("text", Json::Str(message)),
            ])]),
        ),
        ("isError", Json::b(true)),
    ])
}

fn call_tool(params: &Json) -> Result<Json, (i32, String)> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or((-32602, "missing tool name".to_string()))?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or(Json::Obj(vec![]));

    let result = std::panic::catch_unwind(|| exec_tool(name, &args));
    match result {
        Ok(Ok(text)) => Ok(text_result(text)),
        Ok(Err(msg)) => Ok(error_result(msg)),
        Err(_) => Ok(error_result(
            "vibe-pilot panicked while scanning — likely an unreadable path".into(),
        )),
    }
}

fn exec_tool(name: &str, args: &Json) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {}", e))?;
    let root: PathBuf = match args.get("path").and_then(|p| p.as_str()) {
        Some(p) if !p.is_empty() => cwd.join(p),
        _ => cwd,
    };
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }

    let cfg = crate::config::load(&root);
    let profile_name = args
        .get("profile")
        .and_then(|p| p.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| cfg.profile.clone());
    let profile = crate::config::builtin(&profile_name)
        .map(|p| crate::config::apply_overrides(p, &cfg))
        .ok_or_else(|| {
            format!(
                "unknown profile '{}' — use lean | cursor | windsurf | aider | claude | full",
                profile_name
            )
        })?;

    let opts = Options {
        profile,
        task: args
            .get("task")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        diff_only: name == "vp_diff",
        max_tokens: args
            .get("maxTokens")
            .and_then(|m| m.as_num())
            .map(|n| n as usize),
        strip_comments: None,
        ignore: cfg.ignore.clone(),
        model: cfg.model.clone(),
        price_override: None,
    };

    match name {
        "vp_report" => {
            let mut scan = pipeline::scan(&root, &opts);
            let projections = pipeline::project_stats(&mut scan, &opts);
            let st = Style { color: false };
            Ok(
                crate::report::report(&mut scan, &opts, &projections, 0, &st)
                    .trim()
                    .to_string(),
            )
        }
        "vp_prune" | "vp_context" | "vp_diff" => {
            if name == "vp_context" && opts.task.is_none() {
                return Err("vp_context requires a task".into());
            }
            let mut scan = pipeline::scan(&root, &opts);
            let emitted = pipeline::emit(&mut scan, &opts);
            Ok(emitted.content)
        }
        other => Err(format!(
            "unknown tool '{}' — expected vp_report | vp_prune | vp_context | vp_diff",
            other
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_has_all_four() {
        let j = tools_list();
        let tools = j.get("tools").unwrap().as_arr().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .map(|t| t.get("name").unwrap().as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec!["vp_report", "vp_prune", "vp_context", "vp_diff"]
        );
    }

    #[test]
    fn handles_initialize() {
        let mut out = String::new();
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        handle_message(msg, &mut out);
        assert!(out.contains("\"serverInfo\""));
        assert!(out.contains("vibe-pilot"));
        assert!(out.ends_with("}\n"));
    }

    #[test]
    fn notifications_get_no_response() {
        let mut out = String::new();
        handle_message(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            &mut out,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn unknown_method_error() {
        let mut out = String::new();
        handle_message(r#"{"jsonrpc":"2.0","id":7,"method":"nope"}"#, &mut out);
        assert!(out.contains("-32601"));
    }
}
