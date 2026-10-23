//! vibe-pilot — the context optimizer for AI engineers.
//!
//! Prunes your codebase into a hyper-focused `.context` file before you feed
//! it to AI coding assistants like Cursor, Claude Code, or Windsurf.
//!
//! Zero external dependencies. Pure std. That is the point.

pub mod cli;
pub mod config;
pub mod context;
pub mod git;
pub mod json;
pub mod mcp;
pub mod pipeline;
pub mod profiler;
pub mod pruner;
pub mod ranker;
pub mod report;
pub mod tokens;
pub mod util;
pub mod walker;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const TAGLINE: &str = "the context optimizer for AI engineers";
