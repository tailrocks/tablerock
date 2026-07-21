#!/usr/bin/env bash
# Named Swift XCTest coverage against PostgreSQL, ClickHouse, and Redis.
# Each invocation gets an independently created bridge/runtime and the script
# owns every live-server container it creates.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/native"
RUN_ID="${GITHUB_RUN_ID:-local}-$$"
PG="tablerock-pg-swift-$RUN_ID"
CH="tablerock-ch-swift-$RUN_ID"
REDIS="tablerock-redis-swift-$RUN_ID"

cleanup() {
    docker rm -f "$PG" "$CH" "$REDIS" >/dev/null 2>&1 || true
}
trap cleanup EXIT

mapped_port() {
    docker port "$1" "$2/tcp" | sed -En '1s/^.*:([0-9]+)$/\1/p'
}

run_test() {
    local engine="$1"
    local port="$2"
    local test_name="$3"
    shift 3
    (
        cd "$NATIVE"
        env \
            DYLD_LIBRARY_PATH="$REPO_ROOT/target/release" \
            TABLEROCK_LIVE_TEST=1 \
            TABLEROCK_ENGINE="$engine" \
            TABLEROCK_HOST=127.0.0.1 \
            TABLEROCK_PORT="$port" \
            "$@" \
            swift test -c release --filter "LiveBridgeBehaviorTests/$test_name"
    )
}

echo "==> Building native bridge"
cd "$REPO_ROOT"
cargo build -p tablerock-ffi --release --locked
./scripts/generate-swift-bindings.sh
git diff --exit-code -- native/Generated native/Sources/TableRockBridge/tablerock_ffi.swift

echo "==> PostgreSQL named Swift tests"
docker run -d --name "$PG" \
    -e POSTGRES_PASSWORD=secret -e POSTGRES_USER=u -e POSTGRES_DB=db \
    -P postgres:18.4-alpine >/dev/null
pg_port="$(mapped_port "$PG" 5432)"
for i in $(seq 1 30); do
    docker exec "$PG" pg_isready -U u -d db >/dev/null 2>&1 \
        && docker exec "$PG" psql -U u -d db -c 'SELECT 1' >/dev/null 2>&1 \
        && break
    sleep 1
    [ "$i" -eq 30 ] && { echo "PostgreSQL not ready" >&2; exit 1; }
done
docker exec "$PG" psql -U u -d db -v ON_ERROR_STOP=1 -c \
    'CREATE TABLE public.users (id bigint PRIMARY KEY); INSERT INTO public.users VALUES (1);' \
    >/dev/null
run_test postgresql "$pg_port" testQueryReturnsTypedPageWithExpectedValue \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret \
    TABLEROCK_EXPECT_COLS=n TABLEROCK_EXPECT_ROW=1
run_test postgresql "$pg_port" testQueryReturnsTypedPageWithExpectedValue \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret \
    TABLEROCK_QUERY='SELECT 1.5::double precision AS n' \
    TABLEROCK_EXPECT_COLS=n TABLEROCK_EXPECT_ROW=1.5
run_test postgresql "$pg_port" testCatalogReturnsTypedNodesAndBrowsableObjectPage \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret
run_test postgresql "$pg_port" testPostgreSQLCancellationReportsRuntimeAndTerminalWithinBudget \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret
run_test postgresql "$pg_port" testPostgreSQLReviewTokenAppliesProbe \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret
docker rm -f "$PG" >/dev/null

echo "==> ClickHouse named Swift tests"
docker run -d --name "$CH" \
    -e CLICKHOUSE_USER=u -e CLICKHOUSE_PASSWORD=secret -e CLICKHOUSE_DB=db \
    -P clickhouse/clickhouse-server:25.8 >/dev/null
ch_port="$(mapped_port "$CH" 8123)"
for i in $(seq 1 45); do
    docker exec "$CH" clickhouse-client --user u --password secret --database db \
        --query 'SELECT 1' >/dev/null 2>&1 && break
    sleep 1
    [ "$i" -eq 45 ] && { echo "ClickHouse not ready" >&2; exit 1; }
done
docker exec "$CH" clickhouse-client --user u --password secret --database db \
    --multiquery --query \
    'CREATE TABLE events (id UInt64) ENGINE = MergeTree ORDER BY id; INSERT INTO events VALUES (1);' \
    >/dev/null
run_test clickhouse "$ch_port" testQueryReturnsTypedPageWithExpectedValue \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret \
    TABLEROCK_EXPECT_COLS=n TABLEROCK_EXPECT_ROW=1
run_test clickhouse "$ch_port" testCatalogReturnsTypedNodesAndBrowsableObjectPage \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret
docker rm -f "$CH" >/dev/null

echo "==> Redis named Swift tests"
docker run -d --name "$REDIS" -P redis:8.0 >/dev/null
redis_port="$(mapped_port "$REDIS" 6379)"
for i in $(seq 1 30); do
    docker exec "$REDIS" redis-cli ping 2>/dev/null | grep -q PONG && break
    sleep 1
    [ "$i" -eq 30 ] && { echo "Redis not ready" >&2; exit 1; }
done
run_test redis "$redis_port" testQueryReturnsTypedPageWithExpectedValue \
    TABLEROCK_DB=0 TABLEROCK_USER= TABLEROCK_PASSWORD= TABLEROCK_QUERY=PING
run_test redis "$redis_port" testCatalogReturnsTypedNodesAndBrowsableObjectPage \
    TABLEROCK_DB=0 TABLEROCK_USER= TABLEROCK_PASSWORD=
docker rm -f "$REDIS" >/dev/null

echo "==> All named live Swift bridge tests passed"
