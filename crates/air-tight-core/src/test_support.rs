//! Hand-written enums used by unit tests.
use crate::{Permission, PermissionCategory, PermissionKey, ResourceKindKey};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TA {
    Read,
    Update,
}

impl Permission for TA {
    fn action_name(&self) -> &'static str {
        match self {
            TA::Read => "Read",
            TA::Update => "Update",
        }
    }
    fn all() -> &'static [Self] {
        &[TA::Read, TA::Update]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum P {
    W(TA),
    B(TA),
}

impl PermissionCategory for P {
    fn key(&self) -> PermissionKey {
        PermissionKey::new(format!(
            "{}::{}",
            self.category_name(),
            self.action().action_name()
        ))
    }
    fn category_name(&self) -> &'static str {
        match self {
            P::W(_) => "W",
            P::B(_) => "B",
        }
    }
    fn all() -> &'static [Self] {
        &[
            P::W(TA::Read),
            P::W(TA::Update),
            P::B(TA::Read),
            P::B(TA::Update),
        ]
    }
    fn resource_kind(&self) -> Option<ResourceKindKey> {
        match self {
            P::W(_) => Some(ResourceKindKey::from_static("W")),
            P::B(_) => None,
        }
    }
    fn from_key(key: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|p| p.key().as_str() == key)
    }
}

impl P {
    fn action(&self) -> TA {
        match self {
            P::W(a) | P::B(a) => *a,
        }
    }
}

#[test]
fn category_roundtrip() {
    for p in P::all() {
        assert_eq!(P::from_key(p.key().as_str()), Some(*p));
    }
    assert_eq!(P::from_key("nope"), None);
}
