use super::HandlerKind;

/// One handler registered by the attribute macros.
///
/// Built by generated code only; the fields are public so the macros can
/// construct it in a `static`.
#[derive(Debug)]
pub struct HandlerEntry {
    /// `module_path!()` where the handler is defined.
    pub module_path: &'static str,
    /// The handler's function name.
    pub name: &'static str,
    /// Protected or public.
    pub kind: HandlerKind,
    /// The condition as written in the attribute (empty for public handlers).
    pub condition: &'static str,
    /// The permission keys named by the condition.
    pub keys: fn() -> Vec<String>,
    /// The resource kinds the condition's categories are bound to.
    pub resource_kinds: fn() -> Vec<String>,
}

impl HandlerEntry {
    /// `module_path::name`.
    pub fn path(&self) -> String {
        format!("{}::{}", self.module_path, self.name)
    }

    /// Whether `path` (e.g. from `std::any::type_name`, generics stripped)
    /// names this handler.
    pub fn matches(&self, path: &str) -> bool {
        path.strip_prefix(self.module_path)
            .and_then(|rest| rest.strip_prefix("::"))
            .is_some_and(|rest| rest == self.name)
    }
}
