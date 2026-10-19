use std::any::type_name;

use axum::http::Method;

use super::RouteTarget;

/// A typed wrapper over `axum::routing::MethodRouter` that remembers which
/// handler serves which method. Build it with [`crate::routing`].
pub struct MethodRouter<S = ()> {
    pub(crate) inner: axum::routing::MethodRouter<S>,
    pub(crate) handlers: Vec<(Method, &'static str)>,
}

/// The handler path from a factory's type name, generics stripped.
pub(crate) fn handler_name<F>() -> &'static str {
    let full = type_name::<F>();
    full.split_once('<').map_or(full, |(head, _)| head)
}

macro_rules! chain_methods {
    ($($name:ident => $method:ident),* $(,)?) => {$(
        #[doc = concat!("Adds a `", stringify!($method), "` handler; see [`crate::routing::", stringify!($name), "`].")]
        pub fn $name<F, R, T>(mut self, handler: F) -> Self
        where
            F: FnOnce() -> R,
            R: RouteTarget<T, S>,
            T: 'static,
        {
            self.handlers.push((Method::$method, handler_name::<F>()));
            self.inner = self.inner.$name(handler().into_handler());
            self
        }
    )*};
}

impl<S: Clone + Send + Sync + 'static> MethodRouter<S> {
    pub(crate) fn single<F, R, T>(method: Method, handler: F) -> Self
    where
        F: FnOnce() -> R,
        R: RouteTarget<T, S>,
        T: 'static,
    {
        let name = handler_name::<F>();
        let filter = axum::routing::MethodFilter::try_from(method.clone())
            .expect("air_tight only builds method routers for standard methods");
        MethodRouter {
            inner: axum::routing::on(filter, handler().into_handler()),
            handlers: vec![(method, name)],
        }
    }

    chain_methods!(
        get => GET,
        post => POST,
        put => PUT,
        patch => PATCH,
        delete => DELETE,
        head => HEAD,
        options => OPTIONS,
        trace => TRACE,
    );

    /// The `(method, handler path)` pairs registered so far.
    pub fn handlers(&self) -> &[(Method, &'static str)] {
        &self.handlers
    }
}
