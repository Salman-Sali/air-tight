use crate::PermissionKey;

/// A leaf of a condition tree (the permission itself).
pub type Leaf<P> = P;

/// A compile-time permission expression, buildable in `static` items.
#[derive(Debug)]
pub enum Condition<P: 'static> {
    /// A single permission.
    Leaf(P),
    /// All children must hold. Empty is true.
    All(&'static [Condition<P>]),
    /// At least one child must hold. Empty is false.
    Any(&'static [Condition<P>]),
    /// Negation: "lacks the permission", never a deny rule.
    Not(&'static Condition<P>),
}

impl<P: 'static> Condition<P> {
    /// Evaluates the tree, asking `check` about each leaf.
    pub fn evaluate(&self, check: &dyn Fn(&P) -> bool) -> bool {
        match self {
            Condition::Leaf(p) => check(p),
            Condition::All(cs) => cs.iter().all(|c| c.evaluate(check)),
            Condition::Any(cs) => cs.iter().any(|c| c.evaluate(check)),
            Condition::Not(c) => !c.evaluate(check),
        }
    }

    /// All leaves in the tree (including negated ones), in order.
    pub fn leaves(&self) -> Vec<&P> {
        let mut out = Vec::new();
        self.collect_leaves(&mut out);
        out
    }

    fn collect_leaves<'a>(&'a self, out: &mut Vec<&'a P>) {
        match self {
            Condition::Leaf(p) => out.push(p),
            Condition::All(cs) | Condition::Any(cs) => {
                cs.iter().for_each(|c| c.collect_leaves(out));
            }
            Condition::Not(c) => c.collect_leaves(out),
        }
    }
}

impl<P: crate::PermissionCategory> Condition<P> {
    /// Best-effort list of permission keys to blame for a failed condition.
    ///
    /// Returns the leaves that failed in a non-negated position. Inside a
    /// satisfied `Any` nothing is reported; inside a failing `Any` all failing
    /// alternatives are reported. Leaves under `Not` are never reported, since
    /// the remedy would be to remove a permission. The result is deduplicated
    /// and empty if the condition holds.
    pub fn missing(&self, check: &dyn Fn(&P) -> bool) -> Vec<PermissionKey> {
        let mut out = Vec::new();
        if !self.evaluate(check) {
            self.collect_missing(check, &mut out);
        }
        out
    }

    fn collect_missing(&self, check: &dyn Fn(&P) -> bool, out: &mut Vec<PermissionKey>) {
        match self {
            Condition::Leaf(p) => {
                let key = p.key();
                if !check(p) && !out.contains(&key) {
                    out.push(key);
                }
            }
            Condition::All(cs) => cs
                .iter()
                .filter(|c| !c.evaluate(check))
                .for_each(|c| c.collect_missing(check, out)),
            Condition::Any(cs) => {
                if !self.evaluate(check) {
                    cs.iter().for_each(|c| c.collect_missing(check, out));
                }
            }
            Condition::Not(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{P, TA};

    const R: Condition<P> = Condition::Leaf(P::W(TA::Read));
    const U: Condition<P> = Condition::Leaf(P::W(TA::Update));
    const B: Condition<P> = Condition::Leaf(P::B(TA::Read));

    fn has(set: &'static [P]) -> impl Fn(&P) -> bool {
        move |p| set.contains(p)
    }

    #[test]
    fn leaf_and_empty() {
        assert!(R.evaluate(&has(&[P::W(TA::Read)])));
        assert!(!R.evaluate(&has(&[])));
        assert!(Condition::<P>::All(&[]).evaluate(&has(&[])));
        assert!(!Condition::<P>::Any(&[]).evaluate(&has(&[])));
    }

    #[test]
    fn all_any_not_nested() {
        static C: Condition<P> = Condition::All(&[Condition::Any(&[R, U]), Condition::Not(&B)]);
        assert!(C.evaluate(&has(&[P::W(TA::Update)])));
        assert!(!C.evaluate(&has(&[P::W(TA::Update), P::B(TA::Read)])));
        assert!(!C.evaluate(&has(&[])));
        assert_eq!(C.leaves().len(), 3);
    }

    #[test]
    fn missing_reports_failed_positive_leaves() {
        static C: Condition<P> = Condition::All(&[R, U, Condition::Not(&B)]);
        let m = C.missing(&has(&[P::W(TA::Read), P::B(TA::Read)]));
        assert_eq!(m, [PermissionKey::from_static("W::Update")]);
        assert!(
            C.missing(&has(&[P::W(TA::Read), P::W(TA::Update)]))
                .is_empty()
        );
    }

    #[test]
    fn missing_any_reports_all_alternatives() {
        static C: Condition<P> = Condition::Any(&[R, U]);
        let m = C.missing(&has(&[]));
        assert_eq!(m.len(), 2);
        assert!(C.missing(&has(&[P::W(TA::Read)])).is_empty());
    }
}
