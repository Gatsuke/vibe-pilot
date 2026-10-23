//! vp — vibe-pilot CLI entry point.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use vibe_pilot::cli::{self, Cli, Cmd};
use vibe_pilot::config;
use vibe_pilot::pipeline::{self, Options};
use vibe_pilot::report::Style;
use vibe_pilot::tokens;
use vibe_pilot::util::thousands;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match cli::parse(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::from(2);
        }
    };

    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("vp: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, String> {
    // commands that need a repo path
    let needs_repo = matches!(
        cli.cmd,
        Cmd::Init | Cmd::Report | Cmd::Prune | Cmd::Context | Cmd::Diff
    );

    let root: PathBuf = if needs_repo {
        let p = cli.path.clone().unwrap_or_else(|| ".".to_string());
        let pb = PathBuf::from(&p);
        if !pb.is_dir() {
            return Err(format!("not a directory: {}", p));
        }
        std::fs::canonicalize(&pb).map_err(|e| format!("{}: {}", p, e))?
    } else {
        PathBuf::from(".")
    };

    match cli.cmd {
        Cmd::Help => {
            print!("{}", cli::HELP);
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Version => {
            println!(
                "vibe-pilot {} · {}",
                vibe_pilot::VERSION,
                vibe_pilot::TAGLINE
            );
            println!("zero dependencies · rust · MIT");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Profiles => {
            let st = style(&cli);
            print!("{}", vibe_pilot::report::profiles_table(&st));
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Init => init(&root),
        Cmd::Mcp => {
            vibe_pilot::mcp::run();
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Report => report(&cli, &root),
        Cmd::Prune | Cmd::Context | Cmd::Diff => generate(&cli, &root),
    }
}

fn style(cli: &Cli) -> Style {
    let color = !cli.no_color
        && std::env::var("NO_COLOR")
            .map(|v| v.is_empty())
            .unwrap_or(true)
        && std::io::stdout().is_terminal();
    Style { color }
}

fn init(root: &std::path::Path) -> Result<ExitCode, String> {
    let path = config::config_path(root);
    if path.exists() {
        return Err(format!(
            "{} already exists — delete it to regenerate",
            path.display()
        ));
    }
    let template = r#"# vibe-pilot · the context optimizer for AI engineers
# https://github.com/vibe-pilot/vibe-pilot
# Every key is optional — delete lines to use defaults.

[vp]
# profile used by `vp prune` / `vp context`:
#   lean | cursor | windsurf | aider | claude | full
profile = "claude"

# model used for cost estimates (see `vp report` for the full table)
model = "claude-sonnet-5.5"

# extra ignore patterns, in addition to .gitignore
ignore = []

# override any built-in profile, e.g.:
# [profiles.lean]
# max_tokens = 4096
# strip_comments = true
"#;
    std::fs::write(&path, template).map_err(|e| format!("write {}: {}", path.display(), e))?;
    println!("wrote {}", path.display());
    println!("next: vp report");
    Ok(ExitCode::SUCCESS)
}

fn build_options(cli: &Cli, root: &std::path::Path) -> Result<Options, String> {
    let cfg = config::load(root);
    let profile_name = cli.profile.clone().unwrap_or_else(|| cfg.profile.clone());
    let profile = config::builtin(&profile_name)
        .map(|p| config::apply_overrides(p, &cfg))
        .ok_or_else(|| {
            format!(
                "unknown profile '{}' — run `vp profiles` to list them",
                profile_name
            )
        })?;
    let model = cli.model.clone().unwrap_or_else(|| cfg.model.clone());
    if tokens::find(&model).is_none() && cli.price.is_none() {
        eprintln!(
            "[vp] note: '{}' is not a built-in model — using --price or $2/MTok for math. built-ins: {}",
            model,
            tokens::MODELS.iter().map(|m| m.name).collect::<Vec<_>>().join(", ")
        );
    }
    Ok(Options {
        profile,
        task: cli.task.clone(),
        diff_only: cli.cmd == Cmd::Diff,
        max_tokens: cli.max_tokens,
        strip_comments: if cli.lean { Some(true) } else { None },
        ignore: cfg.ignore.clone(),
        model,
        price_override: cli.price,
    })
}

fn report(cli: &Cli, root: &std::path::Path) -> Result<ExitCode, String> {
    let opts = build_options(cli, root)?;
    let t0 = std::time::Instant::now();
    let mut scan = pipeline::scan(root, &opts);
    let projections = pipeline::project_stats(&mut scan, &opts);
    let wall = t0.elapsed().as_millis();

    if cli.json {
        let j = vibe_pilot::report::report_json(&scan, &opts, &projections, wall);
        println!("{}", j.dump());
    } else {
        let st = style(cli);
        print!(
            "{}",
            vibe_pilot::report::report(&mut scan, &opts, &projections, wall, &st)
        );
        if !cli.quiet {
            eprintln!(
                "  next: vp prune --profile {}   # write .context now",
                opts.profile.name
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn generate(cli: &Cli, root: &std::path::Path) -> Result<ExitCode, String> {
    let opts = build_options(cli, root)?;
    let t0 = std::time::Instant::now();
    let mut scan = pipeline::scan(root, &opts);
    let emitted = pipeline::emit(&mut scan, &opts);
    let wall = t0.elapsed().as_millis();

    if cli.stdout {
        print!("{}", emitted.content);
        return Ok(ExitCode::SUCCESS);
    }

    let out_path = root.join(cli.out.clone().unwrap_or_else(|| ".context".to_string()));
    std::fs::write(&out_path, &emitted.content)
        .map_err(|e| format!("write {}: {}", out_path.display(), e))?;

    if !cli.quiet {
        let st = style(cli);
        eprintln!(
            "{}",
            vibe_pilot::report::emit_summary(
                &out_path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".context".into()),
                &emitted.stats,
                &st
            )
        );
        eprintln!(
            "  {} ms · feed it to your assistant, or add .context to .gitignore",
            thousands(wall as usize)
        );
    }
    Ok(ExitCode::SUCCESS)
}
