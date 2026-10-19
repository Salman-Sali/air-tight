use air_tight_core::{PermissionCategory, Principal, ResourceKindKey};
use diesel::expression::{AppearsOnTable, BoxableExpression, Expression, SelectableExpression};
use diesel::pg::Pg;
use diesel::query_builder::QueryFragment;
use diesel::sql_types::Bool;

use crate::air_tight::AirTight;
use crate::{Access, Accessible, Error};

impl<P: PermissionCategory, U: Principal> AirTight<P, U> {
    /// Which resources of `kind` the user may access with `perm`, from the cache.
    ///
    /// `Access::All` when the grant set allows the key globally (the exact
    /// key, `<Category>::*` or `*`). Otherwise `Access::Some` with the sorted
    /// ids of the scoped entries of that kind that allow the key (same
    /// wildcard rules), or `Access::None` when there are none.
    pub fn accessible_ids(
        &self,
        user_id: &str,
        kind: ResourceKindKey,
        perm: P,
    ) -> Result<Access, Error> {
        let set = self.grant_set(user_id)?;
        let key = perm.key();
        if set.allows(&key, None) {
            return Ok(Access::All);
        }
        let ids = set.scoped_ids_allowing(&key, &kind);
        Ok(if ids.is_empty() {
            Access::None
        } else {
            Access::Some(ids)
        })
    }

    /// A diesel filter expression: rows whose `id_column` the user may access
    /// with `perm`, as the concrete [`Accessible`] type.
    ///
    /// Global grants and wildcard keys count. It is a correlated subquery that
    /// reads the database directly and bypasses the grant cache, so it is
    /// always fresh but costs a subquery per candidate row; for small id sets
    /// prefer [`AirTight::accessible_ids`]. The column is compared as text.
    pub fn accessible_expr<C>(
        &self,
        user_id: &str,
        kind: ResourceKindKey,
        perm: P,
        id_column: C,
    ) -> Accessible<C> {
        Accessible::new(user_id, perm.key().as_str(), kind.as_str(), id_column)
    }

    /// Like [`AirTight::accessible_expr`] but boxed, for use in dynamically
    /// built queries: `table.filter(client.accessible(user, kind, perm, table::id))`.
    pub fn accessible<QS, C>(
        &self,
        user_id: &str,
        kind: ResourceKindKey,
        perm: P,
        id_column: C,
    ) -> Box<dyn BoxableExpression<QS, Pg, SqlType = Bool>>
    where
        C: Expression
            + QueryFragment<Pg>
            + SelectableExpression<QS>
            + AppearsOnTable<QS>
            + Send
            + 'static,
        C: diesel::expression::ValidGrouping<
                (),
                IsAggregate = diesel::expression::is_aggregate::No,
            >,
        QS: 'static,
    {
        Box::new(self.accessible_expr(user_id, kind, perm, id_column))
    }
}
