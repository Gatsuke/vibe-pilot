# vibe-pilot — ship less context

**The context optimizer for AI engineers.** Your repo is ~89% noise for the model: lockfiles, generated code, vendored SDKs, three-year-old fixtures. vibe-pilot prunes all of it *before* you feed the codebase to Cursor, Claude Code, Windsurf, or any LLM — and hands you a receipt for the tokens it saved.

```bash
vp report                            # see your token bill, per file class
vp prune --profile cursor            # write .context/ — the pruned repo
vp context fix the session refresh bug   # task-focused context, ranked
```

## Why

On a 231-file TypeScript app, the average AI prompt carries **183,542 tokens of input** — $0.37 per prompt on Sonnet-class pricing, ~$296/month at 40 prompts a day. Most of it is `package-lock.json`. The model never needed it.

vibe-pilot walks your repo (gitignore-aware), classifies every file, ranks what actually matters for your task, and emits a **`.context`** package that runs **20,791 tokens** — same repo, **-88.7%**, $0.04 a prompt. Scan time: **15 ms**.

*Every number is real — the benchmark fixture and runner live in [`bench/`](https://github.com/Gatsuke/vibe-pilot/tree/main/bench). Run `make bench` yourself.*

## Install

```bash
npm i -g vibe-pilot
```

This npm package drops a prebuilt native binary for your platform — **macOS arm64/x64, Linux arm64/x64, Windows x64**. No compilation, no native modules, no other dependencies.

Prefer Rust?

```bash
cargo install --git https://github.com/Gatsuke/vibe-pilot
```

## What's inside

- **6 profiles** — `lean` (8k), `cursor` (24k), `windsurf`, `aider`, `claude` (60k), `full`
- **6 commands** — `report`, `prune`, `context`, `diff`, `watch`, `mcp`
- **cost receipt** — tokens and dollars per prompt, before/after, per model
- **`.context` open format** — stats, task focus, changed files, scored map, pruned files ([spec](https://github.com/Gatsuke/vibe-pilot/blob/main/docs/CONTEXT-SPEC.md))
- **MCP server** — expose `vp_report` / `vp_prune` / `vp_context` / `vp_diff` as tools to any MCP client
- **zero runtime dependencies** — the Rust engine is pure std; this package is a launcher and nothing else

## Docs

- [README (the full story, with receipts)](https://github.com/Gatsuke/vibe-pilot)
- [`.context` format spec](https://github.com/Gatsuke/vibe-pilot/blob/main/docs/CONTEXT-SPEC.md)
- [Landing page](https://gatsuke.github.io/vibe-pilot)

MIT © Gatsuke
