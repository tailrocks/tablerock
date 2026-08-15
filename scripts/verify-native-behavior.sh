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
SERVICE_BACKEND="${TABLEROCK_SERVICE_BACKEND:-}"
SERVICE_ROOT=""
PG_PID=""
CH_PID=""
REDIS_PID=""

if [ -z "$SERVICE_BACKEND" ]; then
    if command -v docker >/dev/null 2>&1; then
        SERVICE_BACKEND=docker
    elif [ "$(uname -s)" = Darwin ]; then
        SERVICE_BACKEND=native
    else
        echo "Neither Docker nor the supported native macOS service backend is available" >&2
        exit 1
    fi
fi

cleanup() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker rm -f "$PG" "$CH" "$REDIS" >/dev/null 2>&1 || true
        return
    fi
    [ -z "$PG_PID" ] || kill "$PG_PID" >/dev/null 2>&1 || true
    [ -z "$CH_PID" ] || kill "$CH_PID" >/dev/null 2>&1 || true
    [ -z "$REDIS_PID" ] || kill "$REDIS_PID" >/dev/null 2>&1 || true
    [ -z "$SERVICE_ROOT" ] || rm -rf "$SERVICE_ROOT"
}
trap cleanup EXIT

mapped_port() {
    docker port "$1" "$2/tcp" | sed -En '1s/^.*:([0-9]+)$/\1/p'
}

require_native_tools() {
    local tool
    for tool in initdb pg_ctl createdb psql clickhouse redis-server redis-cli; do
        command -v "$tool" >/dev/null 2>&1 || {
            echo "Missing mise-managed native service tool: $tool" >&2
            exit 1
        }
    done
}

start_postgres() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker run -d --name "$PG" \
            -e POSTGRES_PASSWORD=secret -e POSTGRES_USER=u -e POSTGRES_DB=db \
            -P postgres:18.6-alpine >/dev/null
        pg_port="$(mapped_port "$PG" 5432)"
        for i in $(seq 1 30); do
            docker exec "$PG" pg_isready -U u -d db >/dev/null 2>&1 \
                && docker exec "$PG" psql -U u -d db -c 'SELECT 1' >/dev/null 2>&1 \
                && return
            sleep 1
            [ "$i" -eq 30 ] && { echo "PostgreSQL not ready" >&2; exit 1; }
        done
        return
    fi

    pg_port=$((20000 + ($$ % 1000)))
    initdb -D "$SERVICE_ROOT/postgres" --auth=trust --username=u --no-locale >/dev/null
    pg_ctl -D "$SERVICE_ROOT/postgres" -o "-h 127.0.0.1 -p $pg_port" \
        -l "$SERVICE_ROOT/postgres.log" -w start >/dev/null
    PG_PID="$(head -n 1 "$SERVICE_ROOT/postgres/postmaster.pid")"
    createdb -h 127.0.0.1 -p "$pg_port" -U u db
}

postgres_exec() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker exec "$PG" psql -U u -d db -v ON_ERROR_STOP=1 -c "$1"
    else
        psql -h 127.0.0.1 -p "$pg_port" -U u -d db -v ON_ERROR_STOP=1 -c "$1"
    fi
}

stop_postgres() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker rm -f "$PG" >/dev/null
    else
        pg_ctl -D "$SERVICE_ROOT/postgres" -m fast -w stop >/dev/null
        PG_PID=""
    fi
}

start_clickhouse() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker run -d --name "$CH" \
            -e CLICKHOUSE_USER=u -e CLICKHOUSE_PASSWORD=secret -e CLICKHOUSE_DB=db \
            -P clickhouse/clickhouse-server:26.7.3.19 >/dev/null
        ch_port="$(mapped_port "$CH" 8123)"
        for i in $(seq 1 45); do
            docker exec "$CH" clickhouse-client --user u --password secret --database db \
                --query 'SELECT 1' >/dev/null 2>&1 && return
            sleep 1
            [ "$i" -eq 45 ] && { echo "ClickHouse not ready" >&2; exit 1; }
        done
        return
    fi

    ch_port=$((21000 + ($$ % 1000)))
    ch_tcp_port=$((22000 + ($$ % 1000)))
    mkdir -p "$SERVICE_ROOT/clickhouse"
    clickhouse server -- \
        --path="$SERVICE_ROOT/clickhouse/" \
        --http_port="$ch_port" \
        --tcp_port="$ch_tcp_port" \
        --listen_host=127.0.0.1 \
        --logger.console=true >"$SERVICE_ROOT/clickhouse.log" 2>&1 &
    CH_PID=$!
    for i in $(seq 1 45); do
        clickhouse client --host 127.0.0.1 --port "$ch_tcp_port" \
            --query 'SELECT 1' >/dev/null 2>&1 && break
        sleep 1
        [ "$i" -eq 45 ] && { cat "$SERVICE_ROOT/clickhouse.log" >&2; exit 1; }
    done
    clickhouse client --host 127.0.0.1 --port "$ch_tcp_port" --multiquery --query \
        "CREATE DATABASE db; CREATE USER u IDENTIFIED WITH plaintext_password BY 'secret'; GRANT ALL ON db.* TO u; GRANT SHOW ON db.* TO u; GRANT TABLE ENGINE ON MergeTree TO u;"
}

clickhouse_exec() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker exec "$CH" clickhouse-client --user u --password secret --database db \
            --multiquery --query "$1"
    else
        clickhouse client --host 127.0.0.1 --port "$ch_tcp_port" \
            --user u --password secret --database db --multiquery --query "$1"
    fi
}

stop_clickhouse() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker rm -f "$CH" >/dev/null
    else
        kill "$CH_PID"
        wait "$CH_PID" || true
        CH_PID=""
    fi
}

start_redis() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker run -d --name "$REDIS" -P redis:8.10.0 >/dev/null
        redis_port="$(mapped_port "$REDIS" 6379)"
        for i in $(seq 1 30); do
            docker exec "$REDIS" redis-cli ping 2>/dev/null | grep -q PONG && return
            sleep 1
            [ "$i" -eq 30 ] && { echo "Redis not ready" >&2; exit 1; }
        done
        return
    fi

    redis_port=$((23000 + ($$ % 1000)))
    redis-server --bind 127.0.0.1 --port "$redis_port" --save '' --appendonly no \
        >"$SERVICE_ROOT/redis.log" 2>&1 &
    REDIS_PID=$!
    for i in $(seq 1 30); do
        redis-cli -h 127.0.0.1 -p "$redis_port" ping 2>/dev/null | grep -q PONG && return
        sleep 1
        [ "$i" -eq 30 ] && { cat "$SERVICE_ROOT/redis.log" >&2; exit 1; }
    done
}

stop_redis() {
    if [ "$SERVICE_BACKEND" = docker ]; then
        docker rm -f "$REDIS" >/dev/null
    else
        redis-cli -h 127.0.0.1 -p "$redis_port" shutdown nosave
        wait "$REDIS_PID" || true
        REDIS_PID=""
    fi
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
if [ "$SERVICE_BACKEND" = native ]; then
    require_native_tools
    SERVICE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/tablerock-native-services.XXXXXX")"
fi
cargo build -p tablerock-ffi --release --locked
./scripts/generate-swift-bindings.sh
git diff --exit-code -- native/Generated native/Sources/TableRockBridge/tablerock_ffi.swift

echo "==> PostgreSQL named Swift tests"
start_postgres
postgres_exec \
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
stop_postgres

echo "==> ClickHouse named Swift tests"
start_clickhouse
clickhouse_exec \
    'CREATE TABLE events (id UInt64) ENGINE = MergeTree ORDER BY id; INSERT INTO events VALUES (1);' \
    >/dev/null
run_test clickhouse "$ch_port" testQueryReturnsTypedPageWithExpectedValue \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret \
    TABLEROCK_EXPECT_COLS=n TABLEROCK_EXPECT_ROW=1
run_test clickhouse "$ch_port" testCatalogReturnsTypedNodesAndBrowsableObjectPage \
    TABLEROCK_DB=db TABLEROCK_USER=u TABLEROCK_PASSWORD=secret
stop_clickhouse

echo "==> Redis named Swift tests"
start_redis
run_test redis "$redis_port" testQueryReturnsTypedPageWithExpectedValue \
    TABLEROCK_DB=0 TABLEROCK_USER= TABLEROCK_PASSWORD= TABLEROCK_QUERY=PING
run_test redis "$redis_port" testCatalogReturnsTypedNodesAndBrowsableObjectPage \
    TABLEROCK_DB=0 TABLEROCK_USER= TABLEROCK_PASSWORD=
stop_redis

echo "==> All named live Swift bridge tests passed"
