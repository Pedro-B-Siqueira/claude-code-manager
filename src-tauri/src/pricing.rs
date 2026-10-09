//! Equivalent API cost per model (USD per million tokens). Structure follows ClaudeGauge's
//! `ModelCatalog` (MIT); rates come from Anthropic's published prices and were checked against the
//! `totalCostUSD` Claude Code records in `cost-state` transcript lines. Cache writes cost 1.25× input
//! (5-minute TTL) or 2× input (1-hour TTL); cache reads have their own per-model rate.

use crate::transcript::usage::TokenUsage;

const PER_MILLION: f64 = 1_000_000.0;
const CACHE_WRITE_5M_MULTIPLIER: f64 = 1.25;
const CACHE_WRITE_1H_MULTIPLIER: f64 = 2.0;
/// Claude Haiku 5.5 bills prompts above this size at its higher tier.
const HAIKU_5_5_TIER_THRESHOLD: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rates {
    input: f64,
    output: f64,
    cache_read: f64,
}

impl Rates {
    const fn new(input: f64, output: f64, cache_read: f64) -> Self {
        Self { input, output, cache_read }
    }

    /// Most models read cache at a tenth of the input price.
    const fn standard(input: f64, output: f64) -> Self {
        Self::new(input, output, input / 10.0)
    }

    fn cost(&self, usage: &TokenUsage) -> f64 {
        let weighted = usage.input as f64 * self.input
            + usage.output as f64 * self.output
            + usage.cache_write_5m as f64 * self.input * CACHE_WRITE_5M_MULTIPLIER
            + usage.cache_write_1h as f64 * self.input * CACHE_WRITE_1H_MULTIPLIER
            + usage.cache_read as f64 * self.cache_read;
        weighted / PER_MILLION
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelPricing {
    Flat(Rates),
    /// Rates that depend on the prompt size of each request.
    Tiered { base: Rates, large_prompt: Rates, threshold: u64 },
}

impl ModelPricing {
    pub fn cost(&self, usage: &TokenUsage) -> f64 {
        match self {
            Self::Flat(rates) => rates.cost(usage),
            Self::Tiered { base, large_prompt, threshold } => {
                let prompt_tokens = usage.input + usage.cache_read + usage.cache_write_5m + usage.cache_write_1h;
                if prompt_tokens > *threshold { large_prompt.cost(usage) } else { base.cost(usage) }
            }
        }
    }
}

/// `None` for `<synthetic>` and models outside the known families, so no cost is made up.
/// The `[1m]` suffix Claude Code adds for long context does not change the price.
pub fn pricing_for(model: &str) -> Option<ModelPricing> {
    let id = model.to_lowercase();
    if id.is_empty() || id.starts_with('<') {
        return None;
    }
    if id.contains("haiku") {
        return Some(haiku_pricing(&id));
    }
    let rates = if id.contains("fable") || id.contains("mythos") {
        fable_rates(&id)
    } else if id.contains("opus") {
        opus_rates(&id)
    } else if id.contains("sonnet") {
        sonnet_rates(&id)
    } else {
        return None;
    };
    Some(ModelPricing::Flat(rates))
}

fn fable_rates(id: &str) -> Rates {
    if id.contains("fable-5-1") || id.contains("mythos-5-1") {
        Rates::new(10.0, 50.0, 0.25)
    } else {
        Rates::standard(10.0, 50.0)
    }
}

fn opus_rates(id: &str) -> Rates {
    let is_legacy = ["opus-4-1", "opus-4-0", "opus-4-2025", "3-opus"].iter().any(|marker| id.contains(marker));
    if id.contains("opus-5-5") {
        Rates::new(4.0, 20.0, 0.20)
    } else if is_legacy {
        Rates::standard(15.0, 75.0)
    } else {
        Rates::standard(5.0, 25.0)
    }
}

fn sonnet_rates(id: &str) -> Rates {
    if id.contains("sonnet-5") { Rates::standard(2.0, 10.0) } else { Rates::standard(3.0, 15.0) }
}

fn haiku_pricing(id: &str) -> ModelPricing {
    if id.contains("haiku-5-5") {
        return ModelPricing::Tiered {
            base: Rates::standard(0.10, 0.50),
            large_prompt: Rates::standard(0.50, 2.50),
            threshold: HAIKU_5_5_TIER_THRESHOLD,
        };
    }
    let rates = if id.contains("3-5-haiku") || id.contains("haiku-3-5") {
        Rates::standard(0.80, 4.0)
    } else if id.contains("3-haiku") || id.contains("haiku-3") {
        Rates::standard(0.25, 1.25)
    } else {
        Rates::standard(1.0, 5.0)
    };
    ModelPricing::Flat(rates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(input: u64, output: u64, cache_read: u64, write_5m: u64, write_1h: u64) -> TokenUsage {
        TokenUsage { input, output, cache_read, cache_write_5m: write_5m, cache_write_1h: write_1h }
    }

    fn cost(model: &str, tokens: TokenUsage) -> f64 {
        pricing_for(model).expect("known model").cost(&tokens)
    }

    #[test]
    fn opus_5_5_matches_claude_code_recorded_total() {
        let recorded = usage(926, 57_056, 17_728_325, 0, 184_985);
        assert!((cost("claude-opus-5-5", recorded) - 6.170_369).abs() < 1e-6);
    }

    #[test]
    fn sonnet_5_5_matches_claude_code_recorded_total() {
        let recorded = usage(1_168, 15_663, 2_796_888, 0, 82_542);
        assert!((cost("claude-sonnet-5-5", recorded) - 1.048_511_6).abs() < 1e-6);
    }

    #[test]
    fn opus_5_and_legacy_opus_rates() {
        assert!((cost("claude-opus-5", usage(1_000_000, 1_000_000, 0, 0, 0)) - 30.0).abs() < 1e-9);
        assert!((cost("claude-opus-4-1-20250805", usage(1_000_000, 0, 0, 0, 0)) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn fable_5_1_reads_cache_cheaper_than_fable_5() {
        let one_million_read = usage(0, 0, 1_000_000, 0, 0);
        assert!((cost("claude-fable-5-1", one_million_read) - 0.25).abs() < 1e-9);
        assert!((cost("claude-fable-5", one_million_read) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn cache_write_multipliers_follow_ttl() {
        let writes = usage(0, 0, 0, 1_000_000, 1_000_000);
        assert!((cost("claude-sonnet-4-6", writes) - (3.75 + 6.0)).abs() < 1e-9);
    }

    #[test]
    fn haiku_5_5_switches_tier_above_100k_prompt_tokens() {
        let small = usage(2_298, 855, 0, 0, 0);
        assert!((cost("claude-haiku-5-5", small) - 0.000_657_3).abs() < 1e-9);
        let large = usage(150_000, 0, 0, 0, 0);
        assert!((cost("claude-haiku-5-5", large) - 0.075).abs() < 1e-9);
    }

    #[test]
    fn older_haiku_generations() {
        let one_million_input = usage(1_000_000, 0, 0, 0, 0);
        assert!((cost("claude-3-5-haiku-20241022", one_million_input) - 0.8).abs() < 1e-9);
        assert!((cost("claude-haiku-4-5-20251001", one_million_input) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn synthetic_and_unknown_models_have_no_price() {
        assert!(pricing_for("<synthetic>").is_none());
        assert!(pricing_for("gpt-x").is_none());
        assert!(pricing_for("").is_none());
    }

    #[test]
    fn long_context_suffix_keeps_the_price() {
        assert_eq!(pricing_for("claude-opus-5-5[1m]"), pricing_for("claude-opus-5-5"));
    }
}
