use std::collections::hash_map::RandomState;
use std::fmt::Write;
use std::hash::{BuildHasher, Hasher};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

const LEN: usize = 7;

/// A new short name, like a git short hash: the first 7 hex characters of
/// the SHA-256 of the time, the pid, and a random number.
#[must_use]
pub fn short() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let random = RandomState::new().build_hasher().finish();
    let mut hasher = Sha256::new();
    hasher.update(nanos.to_le_bytes());
    hasher.update(process::id().to_le_bytes());
    hasher.update(random.to_le_bytes());
    let mut hex = String::with_capacity(LEN + 1);
    for byte in hasher.finalize().iter().take(LEN.div_ceil(2)) {
        let _ = write!(hex, "{byte:02x}");
    }
    hex.truncate(LEN);
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_is_seven_lowercase_hex_characters() {
        let name = short();

        assert_eq!(name.len(), 7);
        assert!(name.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')));
    }

    #[test]
    fn short_differs_between_calls() {
        assert_ne!(short(), short());
    }
}
