use crate::ResourceScope;

/// A value that can name the resources a request touches.
pub trait IdentifyResources {
    /// All resource scopes this value refers to.
    fn resource_scopes(&self) -> Vec<ResourceScope>;
}

impl<T: IdentifyResources> IdentifyResources for Option<T> {
    fn resource_scopes(&self) -> Vec<ResourceScope> {
        self.as_ref().map_or_else(Vec::new, T::resource_scopes)
    }
}

impl<T: IdentifyResources> IdentifyResources for Vec<T> {
    fn resource_scopes(&self) -> Vec<ResourceScope> {
        self.iter().flat_map(T::resource_scopes).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceKindKey;

    struct One(&'static str);
    impl IdentifyResources for One {
        fn resource_scopes(&self) -> Vec<ResourceScope> {
            vec![ResourceScope::new(
                ResourceKindKey::from_static("K"),
                self.0,
            )]
        }
    }

    #[test]
    fn containers() {
        assert_eq!(One("a").resource_scopes().len(), 1);
        assert!(None::<One>.resource_scopes().is_empty());
        assert_eq!(Some(One("a")).resource_scopes()[0].id, "a");
        let v = vec![One("a"), One("b")];
        let ids: Vec<_> = v.resource_scopes().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["a", "b"]);
        assert!(Vec::<One>::new().resource_scopes().is_empty());
    }
}
