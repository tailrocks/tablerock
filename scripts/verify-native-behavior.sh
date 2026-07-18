#!/usr/bin/env bash
# Behavioral verification of the native macOS bridge against all three engines
# (PostgreSQL, ClickHouse, Redis). Spins Docker containers, builds the
# BehaviorProof harness via direct swiftc, and asserts real query round-trips
# through the bridge + page decode for each engine.
#
# Usage: ./scripts/verify-native-behavior.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/native"
BUILD="$NATIVE/.build-direct"

echo "==> Building bridge module + BehaviorProof (direct swiftc)"
cd "$REPO_ROOT"
./scripts/build-native-app.sh >/dev/null
cd "$NATIVE"
swiftc -swift-version 6 -strict-concurrency=complete -warnings-as-errors \
    -I "$BUILD" -I Generated -Xcc -I -Xcc Generated -target arm64-apple-macos14 \
    Sources/BehaviorProof/main.swift \
    "$BUILD/tablerock_ffi.o" "$BUILD/PageV1.o" \
    -L "$REPO_ROOT/target/release" -ltablerock_ffi -framework Foundation \
    -o "$BUILD/BehaviorProof"

run_pg() {
    local name="tablerock-pg-verify"
    echo "==> PostgreSQL"
    docker rm -f "$name" 2>/dev/null || true
    docker run -d --name "$name" \
        -e POSTGRES_PASSWORD=secret -e POSTGRES_USER=u -e POSTGRES_DB=db \
        -p 5433:5432 postgres:18.4-alpine >/dev/null
    for i in $(seq 1 30); do
        docker exec "$name" pg_isready -U u >/dev/null 2>&1 && break
        sleep 1; [ "$i" -eq 30 ] && { echo "    PG not ready"; return 1; }
    done
    DYLD_LIBRARY_PATH="$REPO_ROOT/target/release" \
        TABLEROCK_ENGINE=postgresql TABLEROCK_PORT=5433 TABLEROCK_DB=db \
        TABLEROCK_EXPECT_COLS=n TABLEROCK_EXPECT_ROW=1 \
        "$BUILD/BehaviorProof"
    DYLD_LIBRARY_PATH="$REPO_ROOT/target/release" \
        TABLEROCK_ENGINE=postgresql TABLEROCK_PORT=5433 TABLEROCK_DB=db \
        TABLEROCK_CATALOG=1 "$BUILD/BehaviorProof"
    docker rm -f "$name" >/dev/null
}

run_ch() {
    local name="tablerock-ch-verify"
    echo "==> ClickHouse"
    docker rm -f "$name" 2>/dev/null || true
    docker run -d --name "$name" \
        -e CLICKHOUSE_USER=u -e CLICKHOUSE_PASSWORD=secret -e CLICKHOUSE_DB=db \
        -p 8122:8123 clickhouse/clickhouse-server:25.8 >/dev/null
    sleep 12
    DYLD_LIBRARY_PATH="$REPO_ROOT/target/release" \
        TABLEROCK_ENGINE=clickhouse TABLEROCK_PORT=8122 TABLEROCK_DB=db \
        TABLEROCK_EXPECT_COLS=n TABLEROCK_EXPECT_ROW=1 \
        "$BUILD/BehaviorProof"
    DYLD_LIBRARY_PATH="$REPO_ROOT/target/release" \
        TABLEROCK_ENGINE=clickhouse TABLEROCK_PORT=8122 TABLEROCK_DB=db \
        TABLEROCK_CATALOG=1 "$BUILD/BehaviorProof"
    docker rm -f "$name" >/dev/null
}

run_redis() {
    local name="tablerock-redis-verify"
    echo "==> Redis"
    docker rm -f "$name" 2>/dev/null || true
    docker run -d --name "$name" -p 6380:6379 redis:8.0 >/dev/null
    sleep 4
    DYLD_LIBRARY_PATH="$REPO_ROOT/target/release" \
        TABLEROCK_ENGINE=redis TABLEROCK_PORT=6380 TABLEROCK_DB=0 \
        TABLEROCK_USER="" TABLEROCK_PASSWORD="" TABLEROCK_QUERY="PING" \
        "$BUILD/BehaviorProof"
    DYLD_LIBRARY_PATH="$REPO_ROOT/target/release" \
        TABLEROCK_ENGINE=redis TABLEROCK_PORT=6380 TABLEROCK_DB=0 \
        TABLEROCK_USER="" TABLEROCK_PASSWORD="" TABLEROCK_CATALOG=1 \
        "$BUILD/BehaviorProof"
    docker rm -f "$name" >/dev/null
}

run_pg
run_ch
run_redis
echo "==> All three engines verified"
