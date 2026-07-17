# Phase 2 Profile Endpoint Summary Evidence

## Checkpoint

The bounded profile summary now contains a redacted `ProfileEndpointSummary`.
Host and port are each either a bounded literal revalidated through the
authoritative Host/Port property constructor, or an explicit `SecretSource`
marker with no reference or resolved value.

## Core API migration

Before, `ProfileSourceFacts` redundantly carried host/port source enums. Now it
contains only aggregate secret-risk booleans, while `ProfileEndpointSummary`
owns source kind plus an optional validated display literal. Consumers migrate
`item.sources().host()/port()` to `item.endpoint().host()/port()` and use
`literal_value()` only after checking the source.

No compatibility fields or duplicate endpoint representation remain. Debug
renders bounded literal lengths, never endpoint text.

## Adapter privacy contract

The SQL projection selects `text_value` only from Host/Port rows whose
`source_kind = Literal`. A secret-backed environment, 1Password, prompt,
Keychain, or dangerous-local endpoint yields SQL `NULL` and becomes only the
core marker. No other property text/blob/reference column is selected.

Literal host and port values are reconstructed through core bounds and port
syntax/range validation. A database-valid but semantically invalid port fails
closed as metadata-only `ProfileDecode`; no fallback string is displayed.

## Evidence

- Core tests prove literal-host/secret-port access and Debug redaction.
- Three-engine persistence pages expose literal host/port facts.
- The Redis host is changed to an environment source containing a known
  sentinel; the summary exposes only `SecretSource`, no literal, and no Debug
  sentinel.
- A stored literal port `99999` passes database shape constraints but list
  decoding rejects it through core semantics; a following health command
  succeeds.
- Pagination, exact filters, normalized search, source-risk flags, and
  least-data secret exclusions remain green.

## Deliberate boundary

Endpoint display facts are complete below presentation. Live health/latency/TLS
outcomes require an engine-owned session/test snapshot and remain open. The UI
must render secret-backed endpoint parts as unresolved markers, never resolve
them while listing.

## Verification record

- `cargo test -p tablerock-core --test profile_list`: 4 passed.
- `cargo test -p tablerock-persistence --test profile_create`: 8 passed.
- `cargo test --workspace --all-targets --locked`: 105 passed, 3 ignored.
- Workspace format, Clippy with warnings denied, rustdoc, least-data SQL review,
  diff, English-only, redaction, and provenance review: pass.

External concepts: least-privilege endpoint projections and discriminated unresolved values
Public sources: no new external source; contract derives from approved TableRock profile and privacy architecture
Implementation source: TableRock-owned core summary, persistence projection, and tests
Copied code/assets/text: none
