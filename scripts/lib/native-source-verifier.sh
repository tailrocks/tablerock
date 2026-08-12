#!/usr/bin/env bash
# Shared source scope for native presentation structural verifiers.
# Callers must define REPO_ROOT before sourcing this file.

if [[ -z "${REPO_ROOT:-}" ]]; then
  echo "error: REPO_ROOT must be set before native-source-verifier.sh" >&2
  exit 2
fi

native_source_roots=("$REPO_ROOT/native/Sources/TableRockApp")
native_production_source_roots=(
  "$REPO_ROOT/native/Sources/TableRockApp"
  "$REPO_ROOT/native/Sources/TableRockFeature"
  "$REPO_ROOT/native/Sources/TableRockBridge"
)
if [[ -d "$REPO_ROOT/native/Sources/TableRockPresentation" ]]; then
  native_source_roots+=("$REPO_ROOT/native/Sources/TableRockPresentation")
  native_production_source_roots+=("$REPO_ROOT/native/Sources/TableRockPresentation")
fi

native_source_has_regex() {
  local pattern="$1"
  rg -q -- "$pattern" "${native_source_roots[@]}"
}

native_source_has_fixed() {
  local pattern="$1"
  rg -Fq -- "$pattern" "${native_source_roots[@]}"
}

native_production_source_has_regex() {
  local pattern="$1"
  rg -q -- "$pattern" "${native_production_source_roots[@]}"
}

native_production_source_has_fixed() {
  local pattern="$1"
  rg -Fq -- "$pattern" "${native_production_source_roots[@]}"
}

native_source_regex_count() {
  local pattern="$1"
  {
    rg -o --no-filename -- "$pattern" "${native_source_roots[@]}" || true
  } | awk 'END { print NR }'
}
