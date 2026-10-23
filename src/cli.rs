//! Hand-rolled argument parser. No clap, no deps — the CLI surface is small
//! and a dependency tree for `--profile claude` would be ironic in a tool
//! that deletes dependency trees from your context.

#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    Init,
    Report,
    Prune,
    Context,
    Diff,
    Mcp,
    Profiles,
    Version,
    Help,
}

#[derive(Debug, Clone)]
pub struct Cli {
    pub cmd: Cmd,
    pub path: Option<String>,
    pub task: Option<String>,
    pub out: Option<String>,
    pub profile: Option<String>,
    pub max_tokens: Option<usize>,
    pub lean: bool,
    pub model: Option<String>,
    pub price: Option<f64>,
    pub json: bool,
    pub no_color: bool,
    pub quiet: bool,
    pub stdout: bool,
}

pub const HELP: &str = "\
vibe-pilot 0.1.0 — the context optimizer for AI engineers

Stop paying the context tax. vibe-pilot prunes your codebase into a
hyper-focused .context file before you feed it to Cursor, Claude Code,
Windsurf, or Aider.

USAGE:
    vp <command> [path] [flags]

COMMANDS:
    init                 create vibe-pilot.toml with sane defaults
    report               token + cost analysis of this repo (writes nothing)
    prune                write a pruned .context file for your AI assistant
    context <task...>    task-focused .context — e.g. vp context fix the login bug
    diff                 .context with only what changed vs HEAD (git-aware)
    mcp                  run as an MCP server on stdio (Claude Code / Cursor)
    profiles             list built-in profiles

FLAGS:
    -p, --profile <name>   lean | cursor | windsurf | aider | claude | full
    -t, --task <text>      focus the context on a task
    -o, --out <file>       output path (default: .context)
        --stdout           print .context to stdout instead of writing a file
        --max-tokens <n>   hard token budget
        --lean             strip comments aggressively
        --model <name>     cost model: claude-sonnet-5.5 | claude-opus-5.5 |
                             claude-haiku-4.5 | gpt-6.1-sol | gpt-6-astra |
                             gemini-3.1-pro
        --price <usd>      custom input price per 1M tokens
        --json             machine-readable report
        --no-color         disable ANSI colors
    -q, --quiet            minimal output
    -h, --help             this help
    -V, --version          version

PROFILES:
    lean 8k · cursor 24k · windsurf 24k · aider 12k · claude 60k · full uncapped

QUICK START:
    vp report                     # see your token bill
    vp prune --profile cursor     # write .context
    vp context fix the auth bug   # task-focused context

DOCS:
    https://github.com/vibe-pilot/vibe-pilot
";

pub fn parse(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli {
        cmd: Cmd::Help,
        path: None,
        task: None,
        out: None,
        profile: None,
        max_tokens: None,
        lean: false,
        model: None,
        price: None,
        json: false,
        no_color: false,
        quiet: false,
        stdout: false,
    };

    let mut it = args.iter();
    let first = match it.next() {
        Some(a) => a.as_str(),
        None => return Ok(cli), // bare `vp` -> help
    };

    cli.cmd = match first {
        "init" => Cmd::Init,
        "report" => Cmd::Report,
        "prune" => Cmd::Prune,
        "context" => Cmd::Context,
        "diff" => Cmd::Diff,
        "mcp" => Cmd::Mcp,
        "profiles" => Cmd::Profiles,
        "-h" | "--help" | "help" => Cmd::Help,
        "-V" | "--version" | "version" => Cmd::Version,
        other => return Err(format!("unknown command '{}'\n\n{}", other, HELP)),
    };

    let mut positional: Vec<String> = Vec::new();
    let mut task_flag: Option<String> = None;
    let rest: Vec<String> = it.cloned().collect();
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        macro_rules! val {
            ($name:expr) => {{
                i += 1;
                rest.get(i)
                    .cloned()
                    .ok_or_else(|| format!("flag {} needs a value", $name))?
            }};
        }
        match a.as_str() {
            "-p" | "--profile" => cli.profile = Some(val!(a)),
            "-t" | "--task" => task_flag = Some(val!(a)),
            "-o" | "--out" => cli.out = Some(val!(a)),
            "--max-tokens" => {
                let v = val!(a);
                cli.max_tokens = Some(
                    v.parse::<usize>()
                        .map_err(|_| format!("--max-tokens: '{}' is not a number", v))?,
                );
            }
            "--lean" => cli.lean = true,
            "--model" => cli.model = Some(val!(a)),
            "--price" => {
                let v = val!(a);
                cli.price = Some(
                    v.parse::<f64>()
                        .map_err(|_| format!("--price: '{}' is not a number", v))?,
                );
            }
            "--json" => cli.json = true,
            "--no-color" => cli.no_color = true,
            "-q" | "--quiet" => cli.quiet = true,
            "--stdout" => cli.stdout = true,
            "-h" | "--help" => {
                cli.cmd = Cmd::Help;
                return Ok(cli);
            }
            "-V" | "--version" => {
                cli.cmd = Cmd::Version;
                return Ok(cli);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag '{}'", other));
            }
            other => positional.push(other.to_string()),
        }
        i += 1;
    }

    // `vp context fix the login bug` — leftover positionals are the task
    if cli.cmd == Cmd::Context {
        let words: Vec<String> = positional
            .iter()
            .filter(|p| !p.starts_with('-'))
            .cloned()
            .collect();
        if !words.is_empty() {
            let joined = words.join(" ");
            if task_flag.is_some() {
                return Err("pass the task either as flags or as words, not both".into());
            }
            cli.task = Some(joined);
        } else if task_flag.is_none() {
            return Err(format!(
                "context needs a task, e.g. `vp context fix the login bug`\n\n{}",
                HELP
            ));
        }
    } else if let Some(p) = positional.first() {
        cli.path = Some(p.clone());
    }
    if task_flag.is_some() {
        cli.task = task_flag;
    }

    Ok(cli)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prune() {
        let args: Vec<String> = ["prune", "-p", "cursor", "--max-tokens", "9000"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let c = parse(&args).unwrap();
        assert_eq!(c.cmd, Cmd::Prune);
        assert_eq!(c.profile.as_deref(), Some("cursor"));
        assert_eq!(c.max_tokens, Some(9000));
    }

    #[test]
    fn parses_context_words() {
        let args: Vec<String> = ["context", "fix", "the", "login", "bug"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let c = parse(&args).unwrap();
        assert_eq!(c.task.as_deref(), Some("fix the login bug"));
    }

    #[test]
    fn parses_path_and_out() {
        let args: Vec<String> = ["report", ".", "-o", "x.md"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let c = parse(&args).unwrap();
        assert_eq!(c.path.as_deref(), Some("."));
        assert_eq!(c.out.as_deref(), Some("x.md"));
    }

    #[test]
    fn rejects_garbage() {
        let args: Vec<String> = ["frobnicate"].iter().map(|s| s.to_string()).collect();
        assert!(parse(&args).is_err());
        let args: Vec<String> = ["prune", "--nope"].iter().map(|s| s.to_string()).collect();
        assert!(parse(&args).is_err());
    }
}
