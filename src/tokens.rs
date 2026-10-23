//! Token estimation + live model pricing (input $ per million tokens).
//!
//! Estimator: code tokenizes at roughly 3.7 chars/token across GPT and Claude
//! tokenizers (measured within about 10% of tiktoken on real codebases).
//! Fast, dependency-free, good enough to make cost visible — which is the
//! entire point of vibe-pilot.

/// Average characters per token for source code.
pub const CHARS_PER_TOKEN: f64 = 3.7;

/// Rough per-file overhead (path + markdown wrapper) in tokens.
pub const FILE_OVERHEAD_TOK: usize = 30;

pub struct Model {
    pub name: &'static str,
    pub input: f64,  // USD per 1M input tokens
    pub output: f64, // USD per 1M output tokens
}

/// Prices checked 2026-10 against official pricing pages. Override with
/// `--price <usd>` when they move again.
pub const MODELS: &[Model] = &[
    Model {
        name: "claude-sonnet-5.5",
        input: 2.0,
        output: 10.0,
    },
    Model {
        name: "claude-opus-5.5",
        input: 4.0,
        output: 20.0,
    },
    Model {
        name: "claude-haiku-4.5",
        input: 1.0,
        output: 5.0,
    },
    Model {
        name: "gpt-6.1-sol",
        input: 2.0,
        output: 10.0,
    },
    Model {
        name: "gpt-6-astra",
        input: 10.0,
        output: 50.0,
    },
    Model {
        name: "gemini-3.1-pro",
        input: 2.0,
        output: 12.0,
    },
];

pub fn find(name: &str) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.name == name)
}

pub fn default_model() -> &'static Model {
    &MODELS[0]
}

/// Estimated token count for a chunk of text.
pub fn estimate(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    (text.chars().count() as f64 / CHARS_PER_TOKEN).ceil() as usize
}

/// USD cost of `tokens` at `price_per_mtok`.
pub fn cost(tokens: usize, price_per_mtok: f64) -> f64 {
    tokens as f64 / 1_000_000.0 * price_per_mtok
}

/// "$0.08", "$1.42", "$0.0004"
pub fn fmt_cost(v: f64) -> String {
    if v >= 1.0 {
        format!("${:.2}", v)
    } else {
        format!("${:.4}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_sane() {
        assert_eq!(estimate(""), 0);
        // ~370 chars of code -> ~100 tokens
        let t = "x".repeat(370);
        assert_eq!(estimate(&t), 100);
    }

    #[test]
    fn cost_math() {
        let c = cost(712_400, 2.0);
        assert!((c - 1.4248).abs() < 1e-9);
        assert_eq!(fmt_cost(c), "$1.42");
        assert_eq!(fmt_cost(0.000421), "$0.0004");
    }

    #[test]
    fn model_table() {
        assert!(find("claude-sonnet-5.5").is_some());
        assert!(find("nope").is_none());
    }
}
