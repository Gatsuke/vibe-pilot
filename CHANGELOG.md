# Changelog

All notable changes to vibe-pilot are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[SemVer](https://semver.org/).

## [0.1.0] — 2026-10-23

First public release.

### Added
- `vp report` — token + cost analysis: per-profile projections, biggest
  token sinks, secrets detection. JSON output via `--json`.
- `vp prune` — writes a pruned `.context` file (profiles: `lean`, `cursor`,
  `windsurf`, `aider`, `claude`, `full`).
- `vp context <task>` — task-focused context: ranks files by task terms,
  git recency, import centrality, and entry-point proximity.
- `vp diff` — git-aware context: only what changed vs HEAD.
- `vp mcp` — MCP stdio server exposing `vp_report`, `vp_prune`,
  `vp_context`, `vp_diff` to Claude Code / Cursor agents.
- `.context` open format: stats, task focus, changed list, full scored map,
  pruned file bodies (spec in `docs/CONTEXT-SPEC.md`).
- gitignore-aware walker, generated-file detection, lockfile skipping,
  secret redaction, safe comment stripping.
- `vibe-pilot.toml` config with profile overrides.
- Reproducible benchmark fixture (`make bench`).
- Zero runtime dependencies.
