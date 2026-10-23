# 30-second demo — frame-by-frame storyboard

Goal: a GIF (or muted vertical cut for TikTok/Reels) that shows the whole loop —
bill → cut → receipt — with zero narration. Everything below is REAL output;
record it, don't fake it.

## Setup

- Terminal: 1100x640, dark theme (#0A0F0D), monospace 15–16px, no window chrome (or macOS style)
- Shell prompt: minimal `$` in green
- Record at 60fps (asciinema + agg, or Kap/OBS); export GIF ≤ 15 MB, or MP4 for social
- Type-speed: fast but readable (~90–120 chars/sec with typing animation)
- The repo on screen: the bench fixture (`bash bench/gen.sh`) — numbers must match the README

## The 30 seconds

| # | Time | Frame | On screen | Caption overlay (if any) |
|---|------|-------|-----------|--------------------------|
| 1 | 0:00–0:03 | cold open, black | `$ vp report` typed, cursor blinking | — |
| 2 | 0:03–0:08 | the report prints (fast, 2× replay speed for the long parts) | `raw 183,542 tok · $0.37 / prompt` highlighted; `105,557 tok package-lock.json · lockfile — always skipped` | "your context bill" |
| 3 | 0:08–0:11 | beat | screen holds on the projections block | "-88.7% before you touch anything" |
| 4 | 0:11–0:16 | second command | `$ vp context fix the session refresh bug` typed | — |
| 5 | 0:16–0:22 | the write summary | `wrote .context · 102/232 files · 20,807 tok (raw 227,736 · -90.9%) · was $0.4555 → $0.0416 per prompt` | "was $0.45 → now $0.04" |
| 6 | 0:22–0:27 | the file | `$ head -40 .context` — stats table, task focus with `src/auth/session.ts`, map with scores | "the agent sees the map — and asks for what it needs" |
| 7 | 0:27–0:30 | end card | logo + `vibe-pilot` + `npm i -g vibe-pilot` + github URL on brand green | — |

## Beats that must land

1. **The dollar figure** (frame 2) — the hook is money, not tokens. Make sure $0.37 is on screen ≥ 1.5s.
2. **The arrow** (frame 5) — `was $0.4555 → $0.0416` is the whole product. ≥ 2s on screen.
3. **The map** (frame 6) — skeptics are won by seeing that nothing is hidden: every file, every score.

## Variants to cut from the same recording

- **15s cut:** frames 1, 2, 4, 5, 7 — for replies and retweets
- **Vertical 9:16:** same frames, terminal on top half, captions bottom third — TikTok/Reels/Shorts
- **MCP bonus clip (10s):** `claude mcp add vibe-pilot -- vp mcp` then a Claude Code screenshot where the agent calls `vp_context` itself — post as a reply to anyone asking "does it work with Claude Code?"

## Thumbnail (for video hosts)

- assets/og-banner.png works as-is (terminal receipt + "Stop paying the context tax.")
