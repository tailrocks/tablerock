# Plan 019: Prove the native architecture — tablerock-ffi, UniFFI, XCFramework, Developer ID distribution (Phase 12)

> **Executor instructions**: Work-package plan; this is a PROOF phase — its
> exit is evidence, and failure BLOCKS native work rather than spawning a
> workaround. Authority: delivery-plan.md "Phase 12",
> `docs/architecture/native-macos-path.md`,
> `docs/architecture/shared-client-contract.md` ("Stable bridge facade",
> "UniFFI bridge gate", "Cross-adapter conformance"),
> `docs/architecture/platform-architecture-sources.md`. STOP conditions
> binding. Update `plans/README.md` when done.
>
> **Drift check (run first)**: plan 018 DONE (roadmap: Phase 12 depends on
> 11). Requires a macOS machine with Xcode + a Developer ID certificate +
> notarization credentials — if absent, STOP (operator-provisioned).

## Status

- **State**: IN PROGRESS (checkpoints 1–2: facade + generated Swift + proof harness;
  XCFramework/notarize blocked — no full Xcode / Developer ID on host)
- **Priority**: P2
- **Effort**: L
- **Risk**: HIGH (architecture gate; external toolchain)
- **Depends on**: plans/018
- **Category**: direction (Phase 12 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18
- **Evidence**:
  - `docs/evidence/delivery/249-plan-019-page-codec-and-ffi-facade.md`
  - `docs/evidence/delivery/250-plan-019-swift-bindings-and-proof.md`

## Fixed constraints (inline — non-negotiable without recorded revision)

- Embedded Rust static library through **synchronous coarse UniFFI**; no
  daemon, local RPC, manual C ABI, WebView, or Mac App Store path.
- Facade shape (shared-client-contract.md:60-66 / native-macos-path.md):

  ```text
  open(profile) -> SessionId
  submit(CommandEnvelope) -> OperationId or rejection
  next_events(cursor, maximum) -> bounded event batch
  fetch_page(result_id, range, revision) -> encoded immutable page
  cancel(operation_id) -> request outcome
  shutdown(deadline) -> shutdown outcome
  ```

- Rust owns Tokio; a non-main Swift actor polls bounded event batches;
  avoid UniFFI-generated async functions (documented Swift 6 gaps).
- Pages cross as ONE versioned columnar `Vec<u8>` + safe envelope; Swift
  validates bounds and decodes off `MainActor`; no per-cell calls; Arrow
  explicitly rejected.
- Panics never unwind over FFI.
- Distribution: direct Developer ID + hardened runtime + notarization +
  stapling, signed embedded framework.
- Mutation plans cross as registry HANDLES, never plan bytes
  (shared-client-contract.md "Cross-adapter conformance").

## Current state (entry gate)

- `tablerock-ffi` crate does NOT exist (workspace has 5 crates; the layer
  is named "later tablerock-ffi" in shared-client-contract.md:48).
- The in-process seam (`EngineService` + coordinator + `ResultStore` +
  registry) has run all three engines (plans 002–016) — the precondition
  "bridge facade is introduced only after the command/event contract has
  run all three engines in-process" holds.
- Page encoding: `PageEnvelope::from_wire`/`validate` +
  `ResultPage::from_parts` (`tablerock-core/src/page.rs:317,574`) already
  define the wire-shape validation; a byte-serialization of the envelope +
  buffers is the remaining encoding work (evidence doc 48 records the
  versioned-serialization boundary as open).
- UniFFI is a NEW dependency (adoption checkpoint: latest stable, exact
  pin, license/MSRV/motivation).

## Scope (checkpoints)

1. **Facade crate**: `crates/tablerock-ffi` — synchronous coarse facade
   over `EngineService` + persistence + registry; owns a multi-thread Tokio
   runtime; explicit runtime construction/destruction (idempotent);
   `catch_unwind` at every entry converting panics to typed errors; page
   byte-encoding (envelope serialization, version 1) with bounds checked
   before allocation.
2. **UniFFI binding**: UDL/proc-macro surface; generated Swift package;
   deterministic generation (commit generated artifacts or prove
   regeneration determinism per native-macos-path.md "generated-artifact
   determinism").
3. **XCFramework packaging**: `staticlib` for `aarch64-apple-darwin` +
   `x86_64-apple-darwin`, `lipo`/`xcodebuild -create-xcframework`, signing
   of the embedded framework; scripted + documented (script lives in repo,
   e.g. `scripts/build-xcframework.sh` — new top-level dir needs no
   decision; keep it minimal).
4. **Swift proof harness**: a minimal Swift 6 strict-concurrency CLI/XCTest
   target (new `native/` directory) exercising: open/submit/poll/fetch/
   cancel/shutdown against real containers; `@MainActor` handoff pattern;
   operation-ID cancellation independent of Swift task drop; buffer
   ownership/leak checks (Instruments `leaks`), allocation counts, page
   decode off-main; panic containment (a test-only facade function that
   panics → typed error, process alive).
5. **Cross-adapter conformance suite**: the shared-client-contract.md list
   — command validation, event ordering + stale rejection + resync, page
   byte-for-byte semantic equivalence in-process vs bridge, cancellation
   terminal states, redaction, oversized rejection, disconnect/restart,
   review-token use/expiry, ambiguous-write non-retry, shutdown with
   pending work — run against BOTH the in-process port and the bridge for
   all three engines.
6. **Distribution proof**: sign (hardened runtime) + notarize + staple a
   trivial wrapper app embedding the framework; clean-machine install run
   (fresh macOS VM/user account): Gatekeeper pass, network + file +
   Keychain probe behavior; update/uninstall leaves no residue beyond
   documented paths. Record the full command transcript in evidence.

## Commands

Rust side: standard suites + `cargo build --release --target …`.
Swift side: `swift test` / `xcodebuild test` (document exact invocations in
the evidence); `codesign --verify --deep --strict`, `spctl -a -vv`,
`xcrun notarytool submit … --wait`, `xcrun stapler validate`.

## Done criteria

- [ ] Conformance suite green on in-process AND bridge for all three engines
- [ ] Page equivalence byte-for-byte (semantic) proven; zero per-cell calls (API review + grep)
- [ ] Swift 6 strict concurrency builds with no blanket `@unchecked Sendable`
- [ ] Panic containment + idempotent destruction + leak-free (Instruments) proven
- [ ] Cancellation via operation ID works while the originating Swift task is cancelled/dropped (test)
- [ ] Notarized, stapled, clean-machine-verified artifact; transcript in evidence
- [ ] Adoption checkpoints for uniffi (+ any Swift tooling) recorded
- [ ] Evidence docs + ROADMAP Phase 12 complete; `plans/README.md` updated

## STOP conditions

- Any gate item fails after honest effort — STOP; per the roadmap, failure
  "blocks native work and requires an explicit architecture revision"; do
  not carry a secondary bridge/distribution path.
- UniFFI's current release cannot express the synchronous facade without
  async generation — STOP (decision revision).
- Signing/notarization credentials unavailable — STOP (operator input).

## Maintenance notes

- The conformance suite becomes CI-required for every future core-contract
  change (native and TUI share it).
- Plan 020 consumes: the XCFramework, the Swift decode layer, and the
  conformance harness as its regression floor.
