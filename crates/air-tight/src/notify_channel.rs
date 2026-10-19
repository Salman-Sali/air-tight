use std::fmt;

use crate::Error;

/// Maximum identifier length in Postgres.
const MAX_LEN: usize = 63;

/// A validated Postgres LISTEN/NOTIFY channel name.
///
/// Restricted to plain lowercase identifiers (`[a-z_][a-z0-9_]*`, at most 63
/// bytes) so it can be used in `LISTEN` without quoting.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NotifyChannel(String);

impl NotifyChannel {
    /// The default channel name, `air_tight_invalidate`.
    pub const DEFAULT: &'static str = "air_tight_invalidate";

    /// Validates and wraps a channel name.
    pub fn new(name: &str) -> Result<Self, Error> {
        let mut chars = name.chars();
        let first_ok = chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_');
        let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if first_ok && rest_ok && name.len() <= MAX_LEN {
            Ok(NotifyChannel(name.to_owned()))
        } else {
            Err(Error::Notify(format!(
                "invalid notify channel `{name}`: expected [a-z_][a-z0-9_]* of at most {MAX_LEN} bytes"
            )))
        }
    }

    /// Returns the channel name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for NotifyChannel {
    fn default() -> Self {
        NotifyChannel(Self::DEFAULT.to_owned())
    }
}

impl fmt::Display for NotifyChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_identifiers() {
        assert!(NotifyChannel::new("air_tight_invalidate").is_ok());
        assert!(NotifyChannel::new("_x9").is_ok());
        assert_eq!(NotifyChannel::default().as_str(), "air_tight_invalidate");
    }

    #[test]
    fn rejects_everything_else() {
        for bad in [
            "",
            "9x",
            "Upper",
            "a-b",
            "a b",
            "a;drop",
            "a\"b",
            &"a".repeat(64),
        ] {
            assert!(NotifyChannel::new(bad).is_err(), "{bad}");
        }
    }
}
