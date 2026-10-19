use std::fmt;

use uuid::Uuid;

/// The id of an application user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UserId(Uuid);

impl UserId {
    /// Wraps a uuid.
    pub const fn from_uuid(id: Uuid) -> Self {
        UserId(id)
    }

    /// Parses the textual form, `None` if it is not a uuid.
    pub fn parse(text: &str) -> Option<Self> {
        Uuid::parse_str(text.trim()).ok().map(UserId)
    }

    /// The underlying uuid.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
