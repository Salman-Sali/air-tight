use std::collections::HashSet;

/// Helper trait turning an id-carrying field into textual ids.
///
/// DEVIATION from the spec: a blanket `impl<T: Display>` would conflict with
/// the `Option<T>`, `Vec<T>`, `&[T]` and `HashSet<T>` impls (coherence), so the
/// scalar impls are provided for concrete id types (see `impl_scalar_ids!`)
/// and the container impls are generic over `T: ResourceIds`.
pub trait ResourceIds {
    /// The ids carried by this value, as strings.
    fn ids(&self) -> Vec<String>;
}

macro_rules! impl_scalar_ids {
    ($($t:ty),* $(,)?) => {$(
        impl ResourceIds for $t {
            fn ids(&self) -> Vec<String> {
                vec![self.to_string()]
            }
        }
    )*};
}

impl_scalar_ids!(
    uuid::Uuid,
    String,
    &str,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
);

impl<T: ResourceIds> ResourceIds for Option<T> {
    fn ids(&self) -> Vec<String> {
        self.as_ref().map_or_else(Vec::new, T::ids)
    }
}

impl<T: ResourceIds> ResourceIds for Vec<T> {
    fn ids(&self) -> Vec<String> {
        self.iter().flat_map(T::ids).collect()
    }
}

impl<T: ResourceIds> ResourceIds for &[T] {
    fn ids(&self) -> Vec<String> {
        self.iter().flat_map(T::ids).collect()
    }
}

impl<T: ResourceIds, S> ResourceIds for HashSet<T, S> {
    fn ids(&self) -> Vec<String> {
        self.iter().flat_map(T::ids).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars() {
        assert_eq!(5u32.ids(), ["5"]);
        assert_eq!("x".ids(), ["x"]);
        assert_eq!(String::from("y").ids(), ["y"]);
        let u = uuid::Uuid::nil();
        assert_eq!(u.ids(), [u.to_string()]);
    }

    #[test]
    fn containers() {
        assert_eq!(Some(1i64).ids(), ["1"]);
        assert!(None::<i64>.ids().is_empty());
        assert_eq!(vec![1, 2].ids(), ["1", "2"]);
        let arr = [3u8, 4];
        assert_eq!(arr.as_slice().ids(), ["3", "4"]);
        let set: HashSet<_> = ["a"].into_iter().collect();
        assert_eq!(set.ids(), ["a"]);
        assert_eq!(vec![Some(1), None].ids(), ["1"]);
    }
}
