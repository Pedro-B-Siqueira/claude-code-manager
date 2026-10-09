use serde::{Deserialize, Serialize};

use super::lenient;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct CacheCreationBreakdown {
    #[serde(deserialize_with = "lenient::count")]
    ephemeral_5m_input_tokens: u64,
    #[serde(deserialize_with = "lenient::count")]
    ephemeral_1h_input_tokens: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RawUsage {
    #[serde(deserialize_with = "lenient::count")]
    input_tokens: u64,
    #[serde(deserialize_with = "lenient::count")]
    output_tokens: u64,
    #[serde(deserialize_with = "lenient::count")]
    cache_read_input_tokens: u64,
    #[serde(deserialize_with = "lenient::count")]
    cache_creation_input_tokens: u64,
    cache_creation: Option<CacheCreationBreakdown>,
}

/// Token counts of one API response, with cache writes split by TTL (they are priced differently).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write_5m: u64,
    pub cache_write_1h: u64,
}

impl From<RawUsage> for TokenUsage {
    fn from(raw: RawUsage) -> Self {
        let (cache_write_5m, cache_write_1h) = match raw.cache_creation {
            Some(breakdown)
                if breakdown.ephemeral_5m_input_tokens > 0 || breakdown.ephemeral_1h_input_tokens > 0 =>
            {
                (breakdown.ephemeral_5m_input_tokens, breakdown.ephemeral_1h_input_tokens)
            }
            // Without the TTL breakdown the whole cache write is treated as 5-minute (same as ClaudeGauge).
            _ => (raw.cache_creation_input_tokens, 0),
        };
        Self {
            input: raw.input_tokens,
            output: raw.output_tokens,
            cache_read: raw.cache_read_input_tokens,
            cache_write_5m,
            cache_write_1h,
        }
    }
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write_5m + self.cache_write_1h
    }

    /// Tokens occupying the context window after this response: the full prompt plus the reply.
    pub fn context_tokens(&self) -> u64 {
        self.total()
    }

    pub fn add(&mut self, other: &TokenUsage) {
        self.input += other.input;
        self.output += other.output;
        self.cache_read += other.cache_read;
        self.cache_write_5m += other.cache_write_5m;
        self.cache_write_1h += other.cache_write_1h;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_ttl_breakdown_when_present() {
        let raw: RawUsage = serde_json::from_str(
            r#"{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,
                "cache_creation_input_tokens":30,
                "cache_creation":{"ephemeral_5m_input_tokens":10,"ephemeral_1h_input_tokens":20}}"#,
        )
        .unwrap();
        let usage = TokenUsage::from(raw);
        assert_eq!((usage.cache_write_5m, usage.cache_write_1h), (10, 20));
        assert_eq!(usage.total(), 145);
    }

    #[test]
    fn falls_back_to_total_cache_write_as_5m() {
        let raw: RawUsage =
            serde_json::from_str(r#"{"input_tokens":1,"cache_creation_input_tokens":40}"#).unwrap();
        let usage = TokenUsage::from(raw);
        assert_eq!((usage.cache_write_5m, usage.cache_write_1h), (40, 0));
    }

    #[test]
    fn tolerates_nulls_and_unknown_fields() {
        let raw: RawUsage =
            serde_json::from_str(r#"{"input_tokens":null,"output_tokens":"7","server_tool_use":{}}"#).unwrap();
        assert_eq!(TokenUsage::from(raw).output, 7);
    }
}
