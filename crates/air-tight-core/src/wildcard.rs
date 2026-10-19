use crate::PermissionKey;

/// Whether a stored grant key covers `key`: equal, `*`, or `<Category>::*`.
pub fn grant_matches(grant: &str, key: &PermissionKey) -> bool {
    if grant == "*" || grant == key.as_str() {
        return true;
    }
    match (grant.strip_suffix("::*"), key.as_str().split_once("::")) {
        (Some(cat), Some((key_cat, _))) => !cat.is_empty() && cat == key_cat,
        _ => false,
    }
}

/// Whether `s` is `*` or `<Category>::*` with a non-empty category without `::`.
pub fn is_well_formed_wildcard(s: &str) -> bool {
    s == "*"
        || s.strip_suffix("::*")
            .is_some_and(|c| !c.is_empty() && !c.contains("::"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &'static str) -> PermissionKey {
        PermissionKey::from_static(s)
    }

    #[test]
    fn matching() {
        assert!(grant_matches("Workspace::Read", &k("Workspace::Read")));
        assert!(!grant_matches("Workspace::Read", &k("Workspace::Update")));
        assert!(grant_matches("*", &k("Billing::Manage")));
        assert!(grant_matches("Workspace::*", &k("Workspace::Update")));
        assert!(!grant_matches("Workspace::*", &k("Billing::Manage")));
        assert!(!grant_matches("Work::*", &k("Workspace::Read")));
        assert!(!grant_matches("::*", &k("::Read")));
    }

    #[test]
    fn well_formed() {
        assert!(is_well_formed_wildcard("*"));
        assert!(is_well_formed_wildcard("Workspace::*"));
        assert!(!is_well_formed_wildcard("::*"));
        assert!(!is_well_formed_wildcard("A::B::*"));
        assert!(!is_well_formed_wildcard("Workspace::Read"));
        assert!(!is_well_formed_wildcard(""));
    }
}
