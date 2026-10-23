# Contributing to vibe-pilot

Thanks for helping make AI context cheaper for everyone.

## The one rule

This project has **zero runtime dependencies** — pure `std`. That is a
feature: it keeps `cargo install vibe-pilot` at a few seconds, keeps the
binary auditable, and keeps the dependency tree out of *your* context
window. Please don't add one. If a problem seems to need a crate, it almost
always has a 50-line std solution (see `src/json.rs` for the MCP JSON
parser, `src/walker.rs` for the gitignore engine).

## Setup

```bash
git clone https://github.com/Gatsuke/vibe-pilot
cd vibe-pilot
cargo test          # 41 tests, runs in under a second
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## Layout

| Path | What lives there |
| --- | --- |
| `src/walker.rs` | gitignore-aware directory traversal |
| `src/profiler.rs` | file classification (lockfiles, generated, secrets) |
| `src/ranker.rs` | the scoring formula |
| `src/pruner.rs` | safe comment stripping + truncation |
| `src/pipeline.rs` | scan -> rank -> select orchestration |
| `src/context.rs` | the `.context` format renderer |
| `src/json.rs` | hand-rolled JSON for MCP |
| `src/mcp.rs` | MCP stdio server |
| `tests/integration.rs` | end-to-end tests on a synthetic repo |
| `bench/` | reproducible benchmark fixture (`make bench`) |

## Ground rules

- Every behavior change ships with a test. The test suite builds a synthetic
  repo in a temp dir — copy that pattern.
- Benchmarks in the README must be reproducible via `bash bench/run.sh`.
  Never publish a number you can't regenerate.
- Comment stripping must stay conservative: **full-line comments only**,
  lint directives always kept. A pruned file that changes program meaning is
  a critical bug.
- Secrets (`.env`, `*.pem`, keys) must never be read into memory. Ever.
  Tests enforce this — keep it that way.

## Adding a profile

Profiles live in `src/config.rs::all()`. Keep the profile set small and
opinionated; users override budgets in `vibe-pilot.toml`.

## Commit style

Conventional commits (`fix:`, `feat:`, `bench:`, `docs:`). Squash PRs into
one commit unless the intermediate states are interesting.

## Reporting bugs

Include: OS, `vp --version` output, and the output of `vp report` (it
contains no file contents — just counts). If a specific file is pruned
wrong, paste the file (redact anything sensitive) and the command you ran.
