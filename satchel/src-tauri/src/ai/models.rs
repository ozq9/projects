//! Known Claude models: what each one supports, and its price (for the cost counter).
//! Any model id works in Settings; unknown ids get permissive defaults and no price.

/// Default model for every call. Configurable in Settings.
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";

/// USD per million tokens.
#[derive(Debug, Clone, Copy)]
pub struct Pricing {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
}

const fn price(input: f64, output: f64, cache_read: f64) -> Pricing {
    // 5-minute cache writes cost 1.25x input.
    Pricing { input, output, cache_write: input * 1.25, cache_read }
}

#[derive(Debug, Clone, Copy)]
pub struct ModelInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub pricing: Pricing,
    /// `output_config.format` (JSON schema constrained output).
    pub structured_outputs: bool,
    /// `output_config.effort`.
    pub effort: bool,
    /// Server-side refusal fallback (`fallbacks: "default"`).
    pub server_fallback: bool,
}

const fn model(
    id: &'static str,
    name: &'static str,
    pricing: Pricing,
    structured_outputs: bool,
    effort: bool,
    server_fallback: bool,
) -> ModelInfo {
    ModelInfo { id, name, pricing, structured_outputs, effort, server_fallback }
}

pub const KNOWN_MODELS: &[ModelInfo] = &[
    model("claude-opus-5-5", "Claude Opus 5.5", price(4.0, 20.0, 0.20), true, true, true),
    model("claude-sonnet-5-5", "Claude Sonnet 5.5", price(2.0, 10.0, 0.10), true, true, true),
    model("claude-haiku-5-5", "Claude Haiku 5.5", price(0.10, 0.50, 0.01), true, true, false),
    model("claude-fable-5-2", "Claude Fable 5.2", price(8.0, 40.0, 0.20), true, true, true),
    model("claude-fable-5-1", "Claude Fable 5.1", price(10.0, 50.0, 1.0), true, true, true),
    model("claude-opus-5", "Claude Opus 5", price(5.0, 25.0, 0.50), true, true, true),
    model("claude-sonnet-5", "Claude Sonnet 5", price(2.0, 10.0, 0.20), true, true, false),
    model("claude-opus-4-8", "Claude Opus 4.8", price(5.0, 25.0, 0.50), true, true, false),
    model("claude-opus-4-7", "Claude Opus 4.7", price(5.0, 25.0, 0.50), false, true, false),
    model("claude-opus-4-6", "Claude Opus 4.6", price(5.0, 25.0, 0.50), false, true, false),
    model("claude-sonnet-4-6", "Claude Sonnet 4.6", price(3.0, 15.0, 0.30), false, true, false),
    model("claude-haiku-4-5", "Claude Haiku 4.5", price(1.0, 5.0, 0.10), true, false, false),
];

/// Exact match first, then the longest known id that prefixes `id` (handles dated snapshots).
pub fn lookup(id: &str) -> Option<&'static ModelInfo> {
    KNOWN_MODELS.iter().find(|m| m.id == id).or_else(|| {
        KNOWN_MODELS
            .iter()
            .filter(|m| id.starts_with(m.id))
            .max_by_key(|m| m.id.len())
    })
}

#[derive(Debug, Clone, Copy)]
pub struct Capabilities {
    pub structured_outputs: bool,
    pub effort: bool,
    pub server_fallback: bool,
}

pub fn capabilities(id: &str) -> Capabilities {
    match lookup(id) {
        Some(m) => Capabilities {
            structured_outputs: m.structured_outputs,
            effort: m.effort,
            server_fallback: m.server_fallback,
        },
        // Unknown (probably newer) model: assume the modern feature set, but don't send the
        // fallback parameter, which is rejected by models that don't support it.
        None => Capabilities { structured_outputs: true, effort: true, server_fallback: false },
    }
}

/// Estimated cost in USD, or `None` if the model's price is unknown.
pub fn cost_usd(id: &str, input: u64, output: u64, cache_write: u64, cache_read: u64) -> Option<f64> {
    let p = lookup(id)?.pricing;
    Some(
        (input as f64 * p.input
            + output as f64 * p.output
            + cache_write as f64 * p.cache_write
            + cache_read as f64 * p.cache_read)
            / 1_000_000.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_and_cost() {
        assert_eq!(lookup("claude-opus-5-5").unwrap().name, "Claude Opus 5.5");
        assert_eq!(lookup("claude-haiku-4-5-20251001").unwrap().id, "claude-haiku-4-5");
        // "claude-opus-5-5" must not be mistaken for "claude-opus-5".
        assert_eq!(lookup("claude-opus-5-5").unwrap().id, "claude-opus-5-5");
        assert!(lookup("my-custom-model").is_none());

        let c = cost_usd("claude-opus-5-5", 1_000_000, 100_000, 0, 0).unwrap();
        assert!((c - 6.0).abs() < 1e-9); // $4 in + $2 out
        assert!(cost_usd("my-custom-model", 1, 1, 0, 0).is_none());
        assert!(!capabilities("my-custom-model").server_fallback);
    }
}
