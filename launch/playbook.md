# Launch playbook — how this repo trends

The product is done. Trending is a distribution problem. This is the plan,
in order, with the boring mechanics spelled out.

## T-minus-1 day: pre-flight

- [ ] `cargo publish` — `cargo install vibe-pilot` must work before anything goes public
- [ ] GitHub repo settings:
  - Description: `The context optimizer for AI engineers. Stop paying the context tax.`
  - Topics (all of them, they are search surface): `rust`, `ai-tools`, `claude`, `cursor`,
    `windsurf`, `aider`, `mcp`, `context-engineering`, `llm`, `developer-tools`, `cli`,
    `token-optimization`, `ai-coding`
  - Social preview: upload `assets/og-banner.png` (Settings → Social preview) — this is
    what every X/HN/Reddit card shows
  - Enable Discussions; enable Issues; disable Wiki
  - Branch protection on `main` after the launch PRs settle
- [ ] Tag `v0.1.0` and let `release.yml` attach binaries (Linux x64/arm64, macOS both, Windows)
- [ ] Verify `install.sh` against a real release URL
- [ ] GitHub Pages ON for `docs/` (Settings → Pages → deploy from branch, `/docs`) —
  landing page goes live at `<user>.github.io/vibe-pilot/`; point a domain later
- [ ] Pin a launch issue: "v0.1.0 is out — what should rank higher?" (feedback magnet)

## Launch day — the schedule (all times ET)

| Time | Action | Where |
|------|--------|-------|
| 08:30 | X thread goes up (launch/x-thread.md) | X |
| 09:00 | Show HN (launch/hackernews.md) | news.ycombinator.com |
| 09:05 | Self-reply on HN with the receipt screenshot | HN |
| 09:15 | Post the demo GIF as a reply to the thread | X |
| 12:00 | `r/ClaudeAI` — "I built a context optimizer for Claude Code" (receipt-first, no link in title) | Reddit |
| 12:30 | `r/cursor` — same angle, Cursor-specific framing | Reddit |
| 15:00 | `r/LocalLLaMA` or `r/programming` — angle: the token-cost math | Reddit |
| 17:00 | Submit PRs to awesome lists: `awesome-mcp-servers`, `awesome-claude-code`, `awesome-rust`, `awesome-cli-apps` | GitHub |
| Evening | Dev.to / Lobsters (if invitation rules allow) cross-post with different intro | — |

**One channel at a time, staggered.** If HN is going well, slow the Reddit posts by
an hour; concentrated bursts beat simultaneous dribble.

## How GitHub Trending actually works (2026)

Trending ranks by **star velocity relative to repo size and age**, roughly: stars
today vs. stars over the past weeks. Practical targets:

- **Day 1: 60–120 stars** puts a new repo on the trending page for `rust` and often the
  global page. The X thread + HN + Reddit stack should produce this organically.
- **Days 2–7: sustain 20–40/day** via awesome lists, search traffic, and the landing page.
- The repo re-enters trending whenever a daily spike beats its own trailing average —
  so a "v0.1.1 shipped within 48h" release creates a second wave.

Do not buy stars, do not ask for star-for-star. GitHub purges those and the repo
gets flagged. Ask people who actually install it.

## The first 48 hours — velocity playbook

1. **Reply to everything, fast.** A launch-day repo with <2h response time on issues
   reads as alive; that alone converts visitors to stars.
2. **Ship visible fixes.** The best launch-day content is a commit that says
   `fix: handle X, reported by @user in #12`. Screenshot it into the X thread.
3. **Benchmarks as content.** When someone posts "here's my repo's receipt" — quote-post it.
   User-generated receipts are the product marketing itself.
4. **The MCP registries.** List on the major MCP server directories and Claude Code's
   community lists — durable, compounding traffic that outlives launch week.
5. **Watch the funnel:** repo views → clones → stars. If views are high but clones low,
   the README hero isn't landing; if clones are high but stars low, the quick start
   needs to be faster (it is already 2 commands — probably fine).

## Week one

- [ ] v0.1.1 within 72h (top-voted issue or two, even if small) — re-triggers trending
- [ ] Convert the pinned issue's best ideas into roadmap checkmarks in the README
- [ ] First external PR: label it `good first issue`-style welcome, merge fast, thank publicly
- [ ] Write the "week one numbers" post (installs, receipts shared, $ saved by users) —
      this is the second X thread and it closes the loop on the honesty positioning

## Positioning guardrails (read before replying to anyone)

- Numbers are always reproducible (`make bench`) or clearly labeled as estimates.
- Never claim exact token counts; we estimate (±10%) and say so.
- Competitors get respect in public (repomix is a good tool; we're a different shape).
- The receipt is the hero — money saved, not vibes.
