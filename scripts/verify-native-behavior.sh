#!/usr/bin/env bash
# Behavioral verification of the native macOS bridge against a live PostgreSQL.
# Spins a Docker container, builds the BehaviorProof harness via direct swiftc,
# and asserts a real SELECT 1 round-trips through the bridge + page decode.
#
# Usage: ./scripts/verify-native-behavior.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/native"
BUILD="$NATIVE/.build-direct"
CONTAINER="tablerock-pg-verify"
PG_PORT="${TABLEROCK_PG_PORT:-5433}"

echo "==> Starting PostgreSQL container (port $PG_PORT)"
docker rm -f "$CONTAINER" 2>/dev/null || true
docker run -d --name "$CONTAINER" \
    -e POSTGRES_PASSWORD=secret -e POSTGRES_USER=u -e POSTGRES_DB=db \
    -p "$PG_PORT:5432" postgres:18.4-alpine >/dev/null

echo "==> Waiting for PostgreSQL readiness"
for i in $(seq 1 30); do
    if docker exec "$CONTAINER" pg_isready -U u >/dev/null 2>&1; then
        echo "    ready"
        break
    fi
    sleep 1
    [ "$i" -eq 30 ] && { echo "    PostgreSQL did not become ready"; exit 1; }
done

echo "==> Building bridge module (direct swiftc)"
cd "$REPO_ROOT"
./scripts/build-native-app.sh >/dev/null

echo "==> Building + running BehaviorProof"
cd "$NATIVE"
swiftc -I "$BUILD" -I Generated -Xcc -I -Xcc Generated -target arm64-apple-macos14 \
    Sources/BehaviorProof/main.swift \
    "$BUILD/tablerock_ffi.o" "$BUILD/PageV1.o" \
    -L "$REPO_ROOT/target/release" -ltablerock_ffi -framework Foundation \
    -o "$BUILD/BehaviorProof"
DYLD_LIBRARY_PATH="$REPO_ROOT/target/release" \
    TABLEROCK_PG_PORT="$PG_PORT" \
    "$BUILD/BehaviorProof"

echo "==> Cleaning up container"
docker rm -f "$CONTAINER" >/dev/null
