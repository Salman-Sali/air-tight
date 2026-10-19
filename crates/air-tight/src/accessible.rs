use diesel::expression::{AppearsOnTable, Expression, SelectableExpression, ValidGrouping};
use diesel::pg::Pg;
use diesel::query_builder::{AstPass, QueryFragment, QueryId};
use diesel::result::QueryResult;
use diesel::sql_types::{Bool, Text};

/// A boolean SQL expression: "the user may use this permission on the row
/// whose id column is `C`".
///
/// Built by [`crate::AirTight::accessible_expr`] (or boxed by
/// [`crate::AirTight::accessible`]). It renders a correlated subquery pair
/// over `air_tight.user_permissions` and `air_tight.user_roles` joined with
/// `air_tight.role_permissions`, so it reads the database directly and
/// bypasses the grant cache. Every user-provided value is sent as a bind
/// parameter; only the id column is embedded, as `CAST(<column> AS TEXT)`.
#[derive(Debug, Clone)]
pub struct Accessible<C> {
    user_id: String,
    key: String,
    category_wildcard: String,
    kind: String,
    id_column: C,
}

impl<C> Accessible<C> {
    pub(crate) fn new(user_id: &str, key: &str, kind: &str, id_column: C) -> Self {
        // A key without a category cannot match a category wildcard; reusing
        // the key itself keeps the IN list well-formed.
        let category_wildcard = key
            .split_once("::")
            .map_or_else(|| key.to_owned(), |(category, _)| format!("{category}::*"));
        Accessible {
            user_id: user_id.to_owned(),
            key: key.to_owned(),
            category_wildcard,
            kind: kind.to_owned(),
            id_column,
        }
    }
}

impl<C: Expression> Expression for Accessible<C> {
    type SqlType = Bool;
}

impl<C: QueryFragment<Pg>> QueryFragment<Pg> for Accessible<C> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql(
            "(EXISTS (SELECT 1 FROM air_tight.user_permissions air_tight_up \
             WHERE air_tight_up.user_id = ",
        );
        out.push_bind_param::<Text, _>(&self.user_id)?;
        out.push_sql(" AND air_tight_up.permission_key IN (");
        self.push_keys(out.reborrow())?;
        out.push_sql(") AND (air_tight_up.scope_kind IS NULL OR (air_tight_up.scope_kind = ");
        out.push_bind_param::<Text, _>(&self.kind)?;
        out.push_sql(" AND air_tight_up.scope_id = CAST(");
        self.id_column.walk_ast(out.reborrow())?;
        out.push_sql(
            " AS TEXT)))) OR EXISTS (SELECT 1 FROM air_tight.user_roles air_tight_ur \
             JOIN air_tight.role_permissions air_tight_rp \
             ON air_tight_rp.role_id = air_tight_ur.role_id \
             WHERE air_tight_ur.user_id = ",
        );
        out.push_bind_param::<Text, _>(&self.user_id)?;
        out.push_sql(" AND air_tight_rp.permission_key IN (");
        self.push_keys(out.reborrow())?;
        out.push_sql(") AND (air_tight_ur.scope_kind IS NULL OR (air_tight_ur.scope_kind = ");
        out.push_bind_param::<Text, _>(&self.kind)?;
        out.push_sql(" AND air_tight_ur.scope_id = CAST(");
        self.id_column.walk_ast(out.reborrow())?;
        out.push_sql(" AS TEXT)))))");
        Ok(())
    }
}

impl<C> Accessible<C> {
    fn push_keys<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_bind_param::<Text, _>(&self.key)?;
        out.push_sql(", ");
        out.push_bind_param::<Text, _>(&self.category_wildcard)?;
        out.push_sql(", '*'");
        Ok(())
    }
}

impl<C> QueryId for Accessible<C> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<C, GB> ValidGrouping<GB> for Accessible<C>
where
    C: ValidGrouping<GB>,
{
    type IsAggregate = <C as ValidGrouping<GB>>::IsAggregate;
}

impl<C, QS> AppearsOnTable<QS> for Accessible<C> where C: AppearsOnTable<QS> {}

impl<C, QS> SelectableExpression<QS> for Accessible<C> where C: SelectableExpression<QS> {}
