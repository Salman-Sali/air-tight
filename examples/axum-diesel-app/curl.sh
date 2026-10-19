#!/usr/bin/env bash
# Exercises the example server and checks the documented status codes.
#
#   cargo run -p axum-diesel-app        # in one terminal
#   ./examples/axum-diesel-app/curl.sh  # in another
#
# BASE overrides the server address. Exits non-zero if any status differs.
set -u

BASE="${BASE:-http://127.0.0.1:3000}"

ALICE=aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa
BOB=bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb
CAROL=cccccccc-cccc-cccc-cccc-cccccccccccc
WORKSPACE_A=11111111-1111-1111-1111-111111111111
WORKSPACE_B=22222222-2222-2222-2222-222222222222
INVOICE_A=a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1
INVOICE_B=b1b1b1b1-b1b1-b1b1-b1b1-b1b1b1b1b1b1
MISSING=99999999-9999-9999-9999-999999999999

failures=0
BODY_FILE=$(mktemp)
trap 'rm -f "$BODY_FILE"' EXIT
LAST_BODY=""

# check EXPECTED_STATUS DESCRIPTION METHOD PATH USER [JSON_BODY]
# USER is a user id, or "-" for no X-User-Id header.
check() {
  local expected="$1" description="$2" method="$3" path="$4" user="$5" body="${6:-}"
  local args=(-s -o "$BODY_FILE" -w '%{http_code}' -X "$method" "$BASE$path")
  [ "$user" != "-" ] && args+=(-H "X-User-Id: $user")
  [ -n "$body" ] && args+=(-H 'Content-Type: application/json' -d "$body")
  local status
  status=$(curl "${args[@]}")
  LAST_BODY=$(cat "$BODY_FILE")
  if [ "$status" = "$expected" ]; then
    printf 'PASS %s  %-5s %-32s %s\n' "$status" "$method" "$description" "$path"
  else
    printf 'FAIL expected %s got %s  %s %s (%s)\n' "$expected" "$status" "$method" "$path" "$description"
    failures=$((failures + 1))
  fi
}

echo "== open and unauthenticated"
check 200 "health, no auth"            GET /health -
check 401 "no user"                    GET /workspaces -
check 401 "malformed user id"          GET /workspaces not-a-uuid
check 401 "unknown user"               GET /workspaces "$MISSING"

echo "== list filtering (cache path and SQL path)"
for path in /workspaces /workspaces/sql; do
  check 200 "alice sees all"           GET "$path" "$ALICE"
  check 200 "bob sees workspace A"     GET "$path" "$BOB"
  check 200 "carol sees none"          GET "$path" "$CAROL"
done

echo "== workspaces"
check 200 "alice, wildcard"            GET "/workspaces/$WORKSPACE_B" "$ALICE"
check 200 "bob, scoped to A"           GET "/workspaces/$WORKSPACE_A" "$BOB"
check 403 "bob, not scoped to B"       GET "/workspaces/$WORKSPACE_B" "$BOB"
check 403 "carol, nothing"             GET "/workspaces/$WORKSPACE_A" "$CAROL"
check 401 "no user"                    GET "/workspaces/$WORKSPACE_A" -
check 403 "bob lacks Update"           PATCH "/workspaces/$WORKSPACE_A" "$BOB" '{"name":"Workspace A"}'
check 200 "alice renames"              PATCH "/workspaces/$WORKSPACE_A" "$ALICE" '{"name":"Workspace A"}'

echo "== invoices"
check 200 "alice, wildcard"            GET "/invoices/$INVOICE_A" "$ALICE"
check 403 "carol, nothing"             GET "/invoices/$INVOICE_B" "$CAROL"
check 201 "alice creates"              POST /invoices "$ALICE" "{\"workspace_id\":\"$WORKSPACE_A\",\"amount\":500}"
check 403 "bob lacks Invoice::Create"  POST /invoices "$BOB" "{\"workspace_id\":\"$WORKSPACE_A\",\"amount\":500}"
check 404 "alice, unknown workspace"   POST /invoices "$ALICE" "{\"workspace_id\":\"$MISSING\",\"amount\":500}"
check 200 "alice moves"                POST /invoices/move "$ALICE" "{\"invoice_id\":\"$INVOICE_A\",\"related_invoice_ids\":[],\"target_workspace_id\":\"$WORKSPACE_A\"}"
check 403 "bob cannot move"            POST /invoices/move "$BOB" "{\"invoice_id\":\"$INVOICE_A\",\"related_invoice_ids\":[\"$INVOICE_B\"],\"target_workspace_id\":\"$WORKSPACE_A\"}"

echo "== billing"
check 200 "alice, wildcard"            GET /billing "$ALICE"
check 403 "bob"                        GET /billing "$BOB"
check 403 "carol"                      GET /billing "$CAROL"
check 403 "alice manages, so not read-only" GET /billing/readonly "$ALICE"
check 403 "carol"                      GET /billing/readonly "$CAROL"

echo "== admin"
ROLE="curl-role-$(date +%s)-$$"
check 201 "alice creates a role"       POST /admin/roles "$ALICE" "{\"name\":\"$ROLE\",\"permissions\":[\"Invoice::Read\"]}"
ROLE_ID=$(printf '%s' "$LAST_BODY" | sed -E 's/.*"id":"([^"]+)".*/\1/')
check 409 "duplicate role name"        POST /admin/roles "$ALICE" "{\"name\":\"$ROLE\",\"permissions\":[]}"
check 400 "unknown permission key"     POST /admin/roles "$ALICE" "{\"name\":\"$ROLE-2\",\"permissions\":[\"Nope::Nope\"]}"
check 200 "alice assigns the role"     POST /admin/assign "$ALICE" "{\"user_id\":\"someone-else\",\"role_id\":\"$ROLE_ID\",\"scope\":{\"kind\":\"Workspace\",\"id\":\"$WORKSPACE_A\"}}"
check 400 "unknown scope kind"         POST /admin/assign "$ALICE" "{\"user_id\":\"someone-else\",\"role_id\":\"$ROLE_ID\",\"scope\":{\"kind\":\"Nope\",\"id\":\"x\"}}"
check 200 "alice grants"               POST /admin/grant "$ALICE" '{"user_id":"someone-else","permission":"Billing::Read"}'
check 403 "bob is not an admin"        POST /admin/grant "$BOB" '{"user_id":"someone-else","permission":"Billing::Read"}'
check 403 "carol is not an admin"      POST /admin/roles "$CAROL" '{"name":"nope"}'
check 401 "no user"                    POST /admin/assign - '{"user_id":"x","role_id":"'"$MISSING"'"}'

echo
if [ "$failures" -eq 0 ]; then
  echo "all checks passed"
else
  echo "$failures check(s) failed"
  exit 1
fi
