//! Tests of `accessible_ids` and the diesel `accessible` expression. They
//! need a database and skip when `AIR_TIGHT_TEST_DATABASE_URL` is unset.

mod common;

use air_tight::{
    Access, AirTight, Permission, PermissionCategory, PermissionKey, Principal, ResourceKind,
    ResourceKindKey, ResourceScope, RoleName,
};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(ResourceKind)]
pub enum Res {
    Workspace,
}

#[derive(Permission)]
pub enum Act {
    Read,
    Update,
}

#[derive(PermissionCategory)]
pub enum Perms {
    #[air_tight(resource = res_kinds::Workspace)]
    Workspace(Act),
}

#[derive(Clone, Debug)]
pub struct User;

impl Principal for User {
    fn air_tight_id(&self) -> String {
        String::new()
    }
}

type Client = AirTight<Perms, User>;

diesel::table! {
    at_test_uuids (id) {
        id -> Uuid,
    }
}

diesel::table! {
    at_test_texts (id) {
        id -> Text,
    }
}

const READ: Perms = Perms::Workspace(Act::Read);

fn kind() -> ResourceKindKey {
    ResourceKindKey::from_static("Workspace")
}

fn key(s: &str) -> PermissionKey {
    PermissionKey::new(s)
}

fn ws(id: impl ToString) -> Option<ResourceScope> {
    Some(ResourceScope::new(kind(), id.to_string()))
}

fn client() -> Option<Client> {
    let pool = common::pool()?;
    common::connect()?;
    Some(
        AirTight::builder(pool)
            .notify_channel(&format!("t_{}", Uuid::new_v4().simple()))
            .resource_kinds::<Res>()
            .build()
            .expect("build client"),
    )
}

/// A connection with session-local (temporary, dropped on disconnect) tables holding three uuid rows and three text rows.
struct Fixture {
    conn: PgConnection,
    uuids: [Uuid; 3],
}

impl Fixture {
    fn new() -> Fixture {
        let mut conn = common::connect().expect("database");
        diesel::sql_query("CREATE TEMP TABLE at_test_uuids (id uuid PRIMARY KEY)")
            .execute(&mut conn)
            .unwrap();
        diesel::sql_query("CREATE TEMP TABLE at_test_texts (id text PRIMARY KEY)")
            .execute(&mut conn)
            .unwrap();
        let uuids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        for u in uuids {
            diesel::insert_into(at_test_uuids::table)
                .values(at_test_uuids::id.eq(u))
                .execute(&mut conn)
                .unwrap();
        }
        for t in ["a", "b", "c"] {
            diesel::insert_into(at_test_texts::table)
                .values(at_test_texts::id.eq(t))
                .execute(&mut conn)
                .unwrap();
        }
        Fixture { conn, uuids }
    }

    fn visible_uuids(&mut self, client: &Client, user: &str) -> Vec<Uuid> {
        let mut rows: Vec<Uuid> = at_test_uuids::table
            .filter(client.accessible(user, kind(), READ, at_test_uuids::id))
            .select(at_test_uuids::id)
            .load(&mut self.conn)
            .unwrap();
        rows.sort();
        rows
    }

    fn visible_texts(&mut self, client: &Client, user: &str) -> Vec<String> {
        let mut rows: Vec<String> = at_test_texts::table
            .filter(client.accessible_expr(user, kind(), READ, at_test_texts::id))
            .select(at_test_texts::id)
            .load(&mut self.conn)
            .unwrap();
        rows.sort();
        rows
    }

    fn sorted_uuids(&self, which: &[usize]) -> Vec<Uuid> {
        let mut v: Vec<Uuid> = which.iter().map(|&i| self.uuids[i]).collect();
        v.sort();
        v
    }
}

#[test]
fn accessible_ids_all_variants() {
    let Some(client) = client() else { return };

    for grant in ["Workspace::Read", "*", "Workspace::*"] {
        let u = common::unique("all");
        client.grant(None, &u, key(grant), None).unwrap();
        assert_eq!(
            client.accessible_ids(&u, kind(), READ).unwrap(),
            Access::All,
            "{grant}"
        );
    }

    // A global grant for another action does not help.
    let other = common::unique("other");
    client
        .grant(None, &other, key("Workspace::Update"), None)
        .unwrap();
    assert_eq!(
        client.accessible_ids(&other, kind(), READ).unwrap(),
        Access::None
    );

    // Scoped direct grants, with a wildcard and a non-matching one mixed in.
    let u = common::unique("some");
    client
        .grant(None, &u, key("Workspace::Read"), ws("w2"))
        .unwrap();
    client
        .grant(None, &u, key("Workspace::*"), ws("w1"))
        .unwrap();
    client.grant(None, &u, key("*"), ws("w3")).unwrap();
    client
        .grant(None, &u, key("Workspace::Update"), ws("w9"))
        .unwrap();
    assert_eq!(
        client.accessible_ids(&u, kind(), READ).unwrap(),
        Access::Some(vec!["w1".into(), "w2".into(), "w3".into()])
    );

    // Scoped role assignment.
    let r = common::unique("role");
    let role = client
        .create_role(None, &RoleName::new(&r).unwrap(), None)
        .unwrap();
    client
        .set_role_permissions(None, role, &[key("Workspace::Read")])
        .unwrap();
    let v = common::unique("viewer");
    client.assign_role(None, &v, role, ws("w5")).unwrap();
    assert_eq!(
        client.accessible_ids(&v, kind(), READ).unwrap(),
        Access::Some(vec!["w5".into()])
    );

    // Nothing at all, and an unknown kind.
    let nobody = common::unique("nobody");
    assert_eq!(
        client.accessible_ids(&nobody, kind(), READ).unwrap(),
        Access::None
    );
    assert_eq!(
        client
            .accessible_ids(&u, ResourceKindKey::new("NoSuchKind"), READ)
            .unwrap(),
        Access::None
    );
    client.delete_role(None, role).unwrap();
}

#[test]
fn accessible_expression_global_and_wildcards() {
    let Some(client) = client() else { return };
    let mut fx = Fixture::new();
    let all_uuids = fx.sorted_uuids(&[0, 1, 2]);

    for grant in ["Workspace::Read", "*", "Workspace::*"] {
        let u = common::unique("g");
        client.grant(None, &u, key(grant), None).unwrap();
        assert_eq!(fx.visible_uuids(&client, &u), all_uuids, "{grant}");
        assert_eq!(fx.visible_texts(&client, &u), ["a", "b", "c"], "{grant}");
    }

    // Global grant of an unrelated action and of another category: nothing.
    let u = common::unique("x");
    client
        .grant(None, &u, key("Workspace::Update"), None)
        .unwrap();
    assert!(fx.visible_uuids(&client, &u).is_empty());
    assert!(fx.visible_texts(&client, &u).is_empty());
}

#[test]
fn accessible_expression_scoped_direct() {
    let Some(client) = client() else { return };
    let mut fx = Fixture::new();
    let u = common::unique("direct");
    client
        .grant(None, &u, key("Workspace::Read"), ws(fx.uuids[0]))
        .unwrap();
    client
        .grant(None, &u, key("Workspace::*"), ws(fx.uuids[2]))
        .unwrap();
    // Wrong kind with a matching id must not leak access.
    client
        .grant(
            None,
            &u,
            key("Workspace::Read"),
            Some(ResourceScope::new(
                ResourceKindKey::new("Elsewhere"),
                fx.uuids[1].to_string(),
            )),
        )
        .unwrap();
    client
        .grant(None, &u, key("Workspace::Read"), ws("b"))
        .unwrap();
    assert_eq!(fx.visible_uuids(&client, &u), fx.sorted_uuids(&[0, 2]));
    assert_eq!(fx.visible_texts(&client, &u), ["b"]);

    // Agrees with the cached path.
    assert_eq!(
        client.accessible_ids(&u, kind(), READ).unwrap(),
        Access::Some({
            let mut v = vec![
                fx.uuids[0].to_string(),
                fx.uuids[2].to_string(),
                "b".to_owned(),
            ];
            v.sort();
            v
        })
    );
}

#[test]
fn accessible_expression_scoped_role_and_nothing() {
    let Some(client) = client() else { return };
    let mut fx = Fixture::new();
    let role = client
        .create_role(None, &RoleName::new(&common::unique("role")).unwrap(), None)
        .unwrap();
    client
        .set_role_permissions(None, role, &[key("Workspace::Read")])
        .unwrap();
    let u = common::unique("viewer");
    client.assign_role(None, &u, role, ws(fx.uuids[1])).unwrap();
    client.assign_role(None, &u, role, ws("c")).unwrap();
    assert_eq!(fx.visible_uuids(&client, &u), fx.sorted_uuids(&[1]));
    assert_eq!(fx.visible_texts(&client, &u), ["c"]);

    // A globally assigned role covers everything.
    let g = common::unique("global-role");
    client.assign_role(None, &g, role, None).unwrap();
    assert_eq!(fx.visible_uuids(&client, &g), fx.sorted_uuids(&[0, 1, 2]));

    // Bypasses the cache: fresh even right after a change, and with no grants
    // at all no rows are returned.
    let nobody = common::unique("nobody");
    assert!(fx.visible_uuids(&client, &nobody).is_empty());
    assert!(fx.visible_texts(&client, &nobody).is_empty());

    // A hostile user id is only ever a bind parameter.
    let evil = "x' OR '1'='1";
    assert!(fx.visible_uuids(&client, evil).is_empty());
    client.delete_role(None, role).unwrap();
}
