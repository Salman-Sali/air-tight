/// Byte-wise string equality usable in `const fn`.
///
/// Used by derive-generated lookups that let `#[permission(...)]` build its
/// condition in a `static` while still rejecting unknown variants at compile time.
#[doc(hidden)]
pub const fn const_str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::const_str_eq;

    const SAME: bool = const_str_eq("Read", "Read");

    #[test]
    fn compares() {
        const { assert!(SAME) };
        assert!(!const_str_eq("Read", "Rea"));
        assert!(!const_str_eq("Read", "Reed"));
        assert!(const_str_eq("", ""));
    }
}
