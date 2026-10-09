//! One secret per app session, so only the processes the app launched can report hook events.

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct TokenRegistry {
    tokens: Mutex<HashMap<String, String>>,
}

impl TokenRegistry {
    pub fn issue(&self, session_key: &str) -> String {
        let token = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        self.tokens.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(session_key.to_owned(), token.clone());
        token
    }

    pub fn revoke(&self, session_key: &str) {
        self.tokens.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(session_key);
    }

    pub fn verify(&self, session_key: &str, presented: &str) -> bool {
        let tokens = self.tokens.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        tokens.get(session_key).is_some_and(|expected| constant_time_equals(expected.as_bytes(), presented.as_bytes()))
    }
}

fn constant_time_equals(expected: &[u8], presented: &[u8]) -> bool {
    if expected.len() != presented.len() {
        return false;
    }
    expected.iter().zip(presented).fold(0u8, |difference, (left, right)| difference | (left ^ right)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_tokens_verify_only_for_their_session() {
        let registry = TokenRegistry::default();
        let token = registry.issue("a");
        assert_eq!(token.len(), 64);
        assert!(registry.verify("a", &token));
        assert!(!registry.verify("b", &token));
        assert!(!registry.verify("a", "wrong"));
        registry.revoke("a");
        assert!(!registry.verify("a", &token));
    }
}
