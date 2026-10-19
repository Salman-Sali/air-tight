use crate::{IdentifyResources, ResourceIds, ResourceKindMarker, ResourceScope};

/// Pairs a value of type `Ids` with the resource kinds its parts refer to.
///
/// This is the helper behind the built-in extractors `AirTightPath<Ids, Kinds>`
/// and `AirTightQuery<Ids, Kinds>`: `Kinds` implements `ResourceKinds<Ids>`
/// and turns the extracted value into resource scopes. It lives in this crate
/// because `ResourceKindMarker` is local here, which lets coherence see that
/// tuples are never markers and so the impls below cannot overlap.
///
/// Implemented for:
///
/// - a single marker `K` over any [`ResourceIds`] value (`AirTightPath<Uuid, Workspace>`),
/// - tuples of markers of arity 1 to 8 over a tuple of [`ResourceIds`] values of
///   the same arity (`AirTightPath<(Uuid, Uuid), (Workspace, Invoice)>`),
/// - `()` over any [`IdentifyResources`] value (a struct deriving the trait).
pub trait ResourceKinds<Ids> {
    /// The scopes named by `ids`, each tagged with the matching kind.
    fn scopes(ids: &Ids) -> Vec<ResourceScope>;
}

fn scopes_of<K: ResourceKindMarker>(ids: Vec<String>) -> impl Iterator<Item = ResourceScope> {
    let kind = K::key();
    ids.into_iter()
        .map(move |id| ResourceScope::new(kind.clone(), id))
}

impl<T: ResourceIds, K: ResourceKindMarker> ResourceKinds<T> for K {
    fn scopes(ids: &T) -> Vec<ResourceScope> {
        scopes_of::<K>(ids.ids()).collect()
    }
}

impl<T: IdentifyResources> ResourceKinds<T> for () {
    fn scopes(ids: &T) -> Vec<ResourceScope> {
        ids.resource_scopes()
    }
}

macro_rules! impl_tuple_kinds {
    ($(($($t:ident $k:ident $i:tt),+)),+ $(,)?) => {$(
        impl<$($t: ResourceIds, $k: ResourceKindMarker),+> ResourceKinds<($($t,)+)> for ($($k,)+) {
            fn scopes(ids: &($($t,)+)) -> Vec<ResourceScope> {
                let mut out = Vec::new();
                $(out.extend(scopes_of::<$k>(ids.$i.ids()));)+
                out
            }
        }
    )+};
}

impl_tuple_kinds!(
    (T0 K0 0),
    (T0 K0 0, T1 K1 1),
    (T0 K0 0, T1 K1 1, T2 K2 2),
    (T0 K0 0, T1 K1 1, T2 K2 2, T3 K3 3),
    (T0 K0 0, T1 K1 1, T2 K2 2, T3 K3 3, T4 K4 4),
    (T0 K0 0, T1 K1 1, T2 K2 2, T3 K3 3, T4 K4 4, T5 K5 5),
    (T0 K0 0, T1 K1 1, T2 K2 2, T3 K3 3, T4 K4 4, T5 K5 5, T6 K6 6),
    (T0 K0 0, T1 K1 1, T2 K2 2, T3 K3 3, T4 K4 4, T5 K5 5, T6 K6 6, T7 K7 7),
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceKindKey;

    struct A;
    struct B;
    impl ResourceKindMarker for A {
        fn key() -> ResourceKindKey {
            ResourceKindKey::from_static("A")
        }
    }
    impl ResourceKindMarker for B {
        fn key() -> ResourceKindKey {
            ResourceKindKey::from_static("B")
        }
    }

    struct Q(String);
    impl IdentifyResources for Q {
        fn resource_scopes(&self) -> Vec<ResourceScope> {
            vec![ResourceScope::new(
                ResourceKindKey::from_static("Q"),
                &self.0,
            )]
        }
    }

    fn pairs(s: Vec<ResourceScope>) -> Vec<(String, String)> {
        s.into_iter().map(|s| (s.kind.to_string(), s.id)).collect()
    }

    #[test]
    fn single_and_containers() {
        let s = <A as ResourceKinds<String>>::scopes(&"x".to_owned());
        assert_eq!(pairs(s), [("A".into(), "x".into())]);
        let s = <A as ResourceKinds<Vec<u32>>>::scopes(&vec![1, 2]);
        assert_eq!(
            pairs(s),
            [("A".into(), "1".into()), ("A".into(), "2".into())]
        );
    }

    #[test]
    fn tuples() {
        let s = <(A,) as ResourceKinds<(u8,)>>::scopes(&(7,));
        assert_eq!(pairs(s), [("A".into(), "7".into())]);
        let s = <(A, B) as ResourceKinds<(&str, Option<u8>)>>::scopes(&("a", Some(2)));
        assert_eq!(
            pairs(s),
            [("A".into(), "a".into()), ("B".into(), "2".into())]
        );
        let s = <(A, B, A) as ResourceKinds<(u8, u8, u8)>>::scopes(&(1, 2, 3));
        assert_eq!(s.len(), 3);
        assert_eq!(s[2].id, "3");
        type K8 = (A, B, A, B, A, B, A, B);
        let s = <K8 as ResourceKinds<(u8, u8, u8, u8, u8, u8, u8, u8)>>::scopes(&(
            0, 1, 2, 3, 4, 5, 6, 7,
        ));
        assert_eq!(s.len(), 8);
        assert_eq!((s[7].kind.as_str(), s[7].id.as_str()), ("B", "7"));
    }

    #[test]
    fn unit_delegates() {
        let s = <() as ResourceKinds<Q>>::scopes(&Q("z".into()));
        assert_eq!(pairs(s), [("Q".into(), "z".into())]);
    }
}
