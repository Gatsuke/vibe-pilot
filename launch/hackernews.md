# Show HN — draft + first-hour defense playbook

## The post

**Title (pick one, A/B your gut):**

- `Show HN: Vibe-Pilot – Prune your codebase before you feed it to AI assistants` (clean, descriptive)
- `Show HN: I cut 88% of the tokens out of my AI coding prompts (Rust, zero deps)` (number-led)

**Text:**

> vibe-pilot is a CLI that prunes a codebase into a small, ranked `.context` file before you feed it to Cursor, Claude Code, Windsurf, or Aider.
>
> The problem it solves is one most of us pay for without seeing: a mid-size repo is 100k–500k tokens of context, a third of which is lockfile, generated code, and boilerplate. On current API pricing that's $0.30–$1.50 of input per prompt, every prompt. This repo's own benchmark fixture: 183,542 raw tokens → 20,791 on the cursor profile (−88.7%).
>
> How it decides what to keep is deliberately boring and documented: score = 3.0·task-relevance + 2.0·git-recency + 1.5·import-centrality + 1.0·path-proximity. Files you touched this week outrank files you forgot in 2023. Lockfiles, generated clients, and binaries are always counted, priced, and skipped — but every excluded file stays visible in a scored map, so the agent can ask for anything by path.
>
> Constraints I cared about: zero runtime dependencies (the whole thing is std, including a 250-line JSON parser for the MCP server), secrets are never read (`.env`, keys, certs are detected by name and skipped before a byte hits memory), comment stripping is full-line-only and test-enforced, and there is no network code at all — it's a local binary that writes one file.
>
> It also runs as an MCP server (`claude mcp add vibe-pilot -- vp mcp`), so the agent can call vp_context / vp_diff itself between prompts.
>
> Every number above is reproducible: `make bench` generates the fixture and prints the receipt. Looking for feedback on the ranking formula, the `.context` format spec (docs/CONTEXT-SPEC.md), and which assistant integrations matter most to you.

**Rules of engagement:** submit 8–10 AM ET on a Tue/Wed/Thu. Do not vote-manipulate. Reply to every substantive comment in the first 90 minutes. Technical honesty > promotion.

## The defense — anticipated objections and honest answers

**"Just use .gitignore."**
.gitignore can't rank files by what you're building, strip boilerplate, enforce a token budget, or tell you what anything costs. vibe-pilot respects .gitignore, then does the parts .gitignore has no opinion about: which files does *this task* need, and what's the bill.

**"repomix / gitingest / code2prompt already do this."**
Respectfully: they pack, we prune. Packers concatenate whatever isn't ignored — useful, and a fine baseline. vibe-pilot scores every file, enforces a budget, and prices the cut. If you're happy with a packer, keep it; if you're hitting context limits or paying per prompt, the receipt is the difference.

**"Token estimates are approximate."**
Yes — chars/3.7, within ~10% of tiktoken on real codebases, and `--price` plugs in exact rates. The report is a receipt, not an invoice. The one thing we refuse to do is ship a tokenizer dependency: zero-dep is a feature here.

**"Prompt caching makes this pointless."**
They stack. Caching discounts repeated input; vibe-pilot shrinks the input before it's ever sent. A cached 183k prefix still costs ~$0.037/request on Sonnet 5.5 — the cursor-profile context costs less than that uncached, and caches for less.

**"Why Rust? Why zero deps? Is this a circlejerk?"**
Fair question. Two answers: `cargo install vibe-pilot` takes seconds and never breaks on a transitive dependency, and a tool whose pitch is "less noise in your context" shouldn't carry a 40-crate dependency tree. The hand-rolled JSON parser is 250 lines with a full test suite — smaller than the serde dependency would be.

**"Does this send my code anywhere?"**
No. No network code exists in the binary. It reads files, writes one `.context` file, prints numbers. You can verify in an afternoon — the whole thing is ~3k lines.

**"The name is silly."**
It is. It's also memorable, and "vibe coding" is the wave the tool rides: you vibe, it pilots the context.

## First-hour checklist

- [ ] Post at 8–10 AM ET; set a 90-minute timer for replies
- [ ] First comment (self-reply) within 2 minutes: the benchmark receipt screenshot + `make bench` pointer
- [ ] Answer every technical question with specifics, admit every limitation instantly
- [ ] If a bug is reported: open an issue, link it, fix fast — visible fix velocity on launch day is the best ad
- [ ] Do NOT link the landing page in every reply; link the repo and let people find it
