use air_tight_core::{Condition, PermissionCategory, ResourceScope};

use crate::{Error, GrantSet};

/// Whether `set` satisfies `cond` for a request touching `scopes`.
///
/// A leaf whose category is bound to resource kind `K` collects the scopes of
/// kind `K`. With none it is checked globally; otherwise the grant must hold on
/// every one of them (a scoped grant on that exact resource or a global grant).
/// A leaf without a resource kind is always checked globally.
pub fn evaluate<P: PermissionCategory>(
    set: &GrantSet,
    cond: &Condition<P>,
    scopes: &[ResourceScope],
) -> bool {
    cond.evaluate(&|p| leaf_holds(set, p, scopes))
}

/// Like [`evaluate`], but returns `Error::Forbidden` carrying the missing keys.
pub(crate) fn ensure<P: PermissionCategory>(
    set: &GrantSet,
    cond: &Condition<P>,
    scopes: &[ResourceScope],
) -> Result<(), Error> {
    let holds = |p: &P| leaf_holds(set, p, scopes);
    if cond.evaluate(&holds) {
        Ok(())
    } else {
        Err(Error::forbidden(cond.missing(&holds)))
    }
}

fn leaf_holds<P: PermissionCategory>(set: &GrantSet, p: &P, scopes: &[ResourceScope]) -> bool {
    let key = p.key();
    let Some(kind) = p.resource_kind() else {
        return set.allows(&key, None);
    };
    let mut relevant = scopes.iter().filter(|s| s.kind == kind).peekable();
    if relevant.peek().is_none() {
        return set.allows(&key, None);
    }
    relevant.all(|s| set.allows(&key, Some((&s.kind, &s.id))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use air_tight_core::{Permission, PermissionKey, ResourceKindKey};

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    enum Act {
        Read,
        Write,
    }

    impl Permission for Act {
        fn action_name(&self) -> &'static str {
            match self {
                Act::Read => "Read",
                Act::Write => "Write",
            }
        }
        fn all() -> &'static [Self] {
            &[Act::Read, Act::Write]
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    enum Perm {
        EvalDoc(Act),
        EvalBill(Act),
    }

    impl PermissionCategory for Perm {
        fn key(&self) -> PermissionKey {
            let (Perm::EvalDoc(a) | Perm::EvalBill(a)) = self;
            PermissionKey::new(format!("{}::{}", self.category_name(), a.action_name()))
        }
        fn category_name(&self) -> &'static str {
            match self {
                Perm::EvalDoc(_) => "EvalDoc",
                Perm::EvalBill(_) => "EvalBill",
            }
        }
        fn all() -> &'static [Self] {
            &[
                Perm::EvalDoc(Act::Read),
                Perm::EvalDoc(Act::Write),
                Perm::EvalBill(Act::Read),
                Perm::EvalBill(Act::Write),
            ]
        }
        fn resource_kind(&self) -> Option<ResourceKindKey> {
            match self {
                Perm::EvalDoc(_) => Some(ResourceKindKey::from_static("EvalDocKind")),
                Perm::EvalBill(_) => None,
            }
        }
        fn from_key(key: &str) -> Option<Self> {
            Self::all()
                .iter()
                .copied()
                .find(|p| p.key().as_str() == key)
        }
    }

    const DOC_READ: Condition<Perm> = Condition::Leaf(Perm::EvalDoc(Act::Read));
    const DOC_WRITE: Condition<Perm> = Condition::Leaf(Perm::EvalDoc(Act::Write));
    const BILL_READ: Condition<Perm> = Condition::Leaf(Perm::EvalBill(Act::Read));

    fn kid(key: &str) -> crate::KeyId {
        crate::Interner::global().intern_key(key).unwrap()
    }

    fn kind() -> crate::KindId {
        crate::Interner::global()
            .intern_kind("EvalDocKind")
            .unwrap()
    }

    fn scope(id: &str) -> ResourceScope {
        ResourceScope::new(ResourceKindKey::from_static("EvalDocKind"), id)
    }

    #[test]
    fn global_grant_covers_every_scope() {
        let mut set = GrantSet::new();
        set.insert_global(kid("EvalDoc::Read"));
        assert!(evaluate(&set, &DOC_READ, &[]));
        assert!(evaluate(&set, &DOC_READ, &[scope("a"), scope("b")]));
        assert!(!evaluate(&set, &DOC_WRITE, &[]));
    }

    #[test]
    fn scoped_grant_needs_every_scope() {
        let mut set = GrantSet::new();
        set.insert_scoped(kid("EvalDoc::Read"), kind(), "a");
        assert!(evaluate(&set, &DOC_READ, &[scope("a")]));
        assert!(!evaluate(&set, &DOC_READ, &[scope("a"), scope("b")]));
        assert!(!evaluate(&set, &DOC_READ, &[scope("b")]));
        // No scope of the kind in the request means a global check.
        assert!(!evaluate(&set, &DOC_READ, &[]));
    }

    #[test]
    fn other_kinds_are_ignored_and_unscoped_categories_are_global() {
        let mut set = GrantSet::new();
        set.insert_scoped(kid("EvalBill::Read"), kind(), "a");
        let other = ResourceScope::new(ResourceKindKey::from_static("Other"), "z");
        assert!(!evaluate(&set, &BILL_READ, &[scope("a")]));
        set.insert_global(kid("EvalBill::Read"));
        assert!(evaluate(&set, &BILL_READ, &[scope("a"), other]));
    }

    #[test]
    fn wildcards_match() {
        let mut set = GrantSet::new();
        set.insert_global(kid("EvalDoc::*"));
        assert!(evaluate(&set, &DOC_WRITE, &[scope("a")]));
        assert!(!evaluate(&set, &BILL_READ, &[]));
        set.insert_global(kid("*"));
        assert!(evaluate(&set, &BILL_READ, &[]));
    }

    #[test]
    fn combinators_and_missing_keys() {
        static ALL: Condition<Perm> = Condition::All(&[DOC_READ, BILL_READ]);
        static ANY: Condition<Perm> = Condition::Any(&[DOC_WRITE, BILL_READ]);
        static NOT: Condition<Perm> = Condition::Not(&BILL_READ);
        let mut set = GrantSet::new();
        set.insert_global(kid("EvalDoc::Read"));
        assert!(!evaluate(&set, &ALL, &[]));
        assert!(!evaluate(&set, &ANY, &[]));
        assert!(evaluate(&set, &NOT, &[]));
        match ensure(&set, &ALL, &[]) {
            Err(Error::Forbidden { missing }) => assert_eq!(missing, ["EvalBill::Read"]),
            other => panic!("unexpected {other:?}"),
        }
        match ensure(&set, &ANY, &[]) {
            Err(Error::Forbidden { missing }) => {
                assert_eq!(missing, ["EvalDoc::Write", "EvalBill::Read"]);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(ensure(&set, &NOT, &[]).is_ok());
    }
}
