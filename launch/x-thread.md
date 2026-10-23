# X / Twitter launch thread — paste-ready

Post timing: launch day, 9:00–10:00 AM ET (Tue–Thu). One tweet per 2–3 minutes,
reply-chained. Numbers below are real, from `make bench` on the public fixture.

---

**1/8 — the hook**

Your AI coding bill has a line item you have never seen.

It's called package-lock.json.

It is 105,557 tokens. It has never fixed a bug. And you paste it into every single prompt.

**2/8 — the math nobody does**

A typical 231-file TS app is 183k tokens of context.

On Sonnet 5.5 that's $0.37 of input per prompt. At 40 prompts/day: $296/month — just to re-explain your own repo to a model that has already forgotten it.

On Opus: $592/month.

**3/8 — the fix**

vibe-pilot prunes your codebase into a hyper-focused `.context` file before it reaches Cursor / Claude Code / Windsurf.

npm i -g vibe-pilot
vp report

That's it. 15ms. Zero dependencies.

**4/8 — the receipt** [attach: assets/demo-report.png]

It doesn't just cut. It shows you the bill first: raw tokens, cost per prompt, what got skipped and why.

Same repo, cursor profile: 20,791 tokens. −88.7%. $0.04/prompt.

**5/8 — it ranks what matters**

You don't get a random slice. Files you touched this week outrank files you forgot in 2023:

score = 3.0·task-relevance + 2.0·git-recency + 1.5·imports + 1.0·proximity

Boring on purpose. Public by design. `vp context "fix the session refresh bug"` → the auth files, not the lockfile.

**6/8 — your agent can call it itself**

It ships as an MCP server:

claude mcp add vibe-pilot -- vp mcp

Now Claude Code / Cursor call `vp_context` before a big change and `vp_diff` between prompts. Context stays lean without you thinking about it.

**7/8 — the safety story**

- `.env`, keys, certs: detected by name, never read. Not one byte.
- Comments stripped only in `--lean`, full-line only, lint directives survive. Test-enforced.
- Local binary. No network code. No telemetry. MIT.

8k lines of Rust, zero runtime dependencies. `cargo install` finishes before your coffee.

**8/8 — ship it**

github.com/Gatsuke/vibe-pilot

Every number in this thread is reproducible: `make bench` in the repo.

If it saves you real money this week, you know where the star button is.

[link + screenshot of the hero banner: assets/og-banner.png]
