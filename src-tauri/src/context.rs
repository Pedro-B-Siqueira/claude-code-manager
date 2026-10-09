//! Context window per model, for the context meter. Current models run with a 1M-token window in
//! Claude Code (auto-compaction fires just under 1M); Haiku 4.5 and earlier generations have 200K.

const STANDARD_WINDOW: u64 = 200_000;
const EXTENDED_WINDOW: u64 = 1_000_000;

const STANDARD_WINDOW_MARKERS: &[&str] =
    &["haiku-4", "-3-", "claude-3", "opus-4-0", "opus-4-1", "opus-4-5", "sonnet-4-0", "sonnet-4-5", "opus-4-2025", "sonnet-4-2025"];

/// Usage beyond the standard window proves a 1M window even for models listed as 200K.
pub fn window_for(model: Option<&str>, observed_context_tokens: u64) -> u64 {
    if observed_context_tokens > STANDARD_WINDOW {
        return EXTENDED_WINDOW;
    }
    let Some(model) = model.map(str::to_lowercase) else { return EXTENDED_WINDOW };
    if model.contains("[1m]") {
        return EXTENDED_WINDOW;
    }
    if STANDARD_WINDOW_MARKERS.iter().any(|marker| model.contains(marker)) {
        STANDARD_WINDOW
    } else {
        EXTENDED_WINDOW
    }
}

pub fn used_percent(context_tokens: u64, window: u64) -> f64 {
    if window == 0 {
        return 0.0;
    }
    (context_tokens as f64 / window as f64 * 100.0).min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_generation_uses_the_1m_window() {
        for model in ["claude-opus-5-5", "claude-sonnet-5-5", "claude-fable-5-1", "claude-haiku-5-5", "claude-opus-5"] {
            assert_eq!(window_for(Some(model), 10_000), 1_000_000, "{model}");
        }
    }

    #[test]
    fn older_generations_use_200k() {
        for model in ["claude-haiku-4-5-20251001", "claude-3-5-haiku-20241022", "claude-sonnet-4-5-20250929", "claude-opus-4-1-20250805"] {
            assert_eq!(window_for(Some(model), 10_000), 200_000, "{model}");
        }
    }

    #[test]
    fn observed_usage_or_suffix_promotes_to_1m() {
        assert_eq!(window_for(Some("claude-sonnet-4-5-20250929"), 250_000), 1_000_000);
        assert_eq!(window_for(Some("claude-sonnet-4-5[1m]"), 0), 1_000_000);
    }

    #[test]
    fn percent_is_capped() {
        assert!((used_percent(750_000, 1_000_000) - 75.0).abs() < 1e-9);
        assert!((used_percent(300_000, 200_000) - 100.0).abs() < 1e-9);
    }
}
