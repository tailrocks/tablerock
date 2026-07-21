# Plan 022: Preview CI/CD — rolling preview release + Homebrew tap (CLI/TUI formula + native macOS cask)

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**:
> `git diff --stat cff5930..HEAD -- .github/ scripts/ native/ crates/tablerock-cli/ Cargo.toml`
> If these paths changed since this plan was written, compare the "Current
> state" section against the live tree before proceeding; on a mismatch, STOP.
> Also re-read the reference workflows named below in their upstream repos —
> holla and parallax move fast; prefer their **latest** main-branch shape over
> the snapshots this plan describes.

## Status

- **IN PROGRESS (2026-07-21): rolling product release, public pull-verified
  tap, formula/cask delivery, hosted provenance checks, manual and organic
  workflows, and real arm64 install/launch/uninstall proof are green (evidence
  554–561); a concurrent uncommitted CI migration replaces the committed
  dependency freshness workflow and modifies `preview.yml`, so final DONE
  awaits reconciliation and fresh proof of that resulting tree**
- **Priority**: P2
- **Effort**: M
- **Risk**: MED (new repo, new runner image, Gatekeeper policy on ad-hoc app)
- **Depends on**: plans/001 (Checks workflow), plans/020 (buildable
  `TableRock.app`); plan 019's Developer ID gate is **not** a dependency —
  the cask ships the ad-hoc-signed preview shape until 019 unblocks
- **Category**: dx / release
- **Planned at**: commit `cff5930`, 2026-07-21

## Why this matters

Every artifact today is build-it-yourself: the TUI needs a Rust toolchain,
the native app needs `scripts/build-native-app.sh` on a machine with Xcode.
There is no way for anyone (operator included) to install "current main" and
verify the product without a checkout. The sibling projects already solved
this: **holla** and **parallax** publish a rolling `preview` GitHub Release
from every green main push and feed a Homebrew tap
(`brew install holla@preview`, `brew install parallax-preview`). TableRock
gets the same, plus something neither sibling has: a **Homebrew cask** that
installs `TableRock.app`, so the current state of the native macOS
application is verifiable with one `brew install --cask` command.

This also becomes the first CI job that builds the Swift/native side at all —
`checks.yml` is cargo-only today — closing a visible gap ahead of plan 021's
release evidence.

## Reference implementations (read before coding)

| Repo | Pattern | Take | Leave |
|---|---|---|---|
| `tailrocks/holla` `.github/workflows/preview.yml` | Push model: main repo builds, then opens+merges a PR against the tap using a PAT (`GH_HOLLA_HOMEBREW_TAP_TOKEN`) | source-sha classification marker, GraphQL commit-count version, rolling `preview` release create/edit pattern, "verify source SHA is on main" guard, `Aliases/holla@preview` symlink | cross-repo PAT (secret to provision + rotate), PR-merge machinery |
| `tailrocks/parallax` `.github/workflows/preview.yml` + `tailrocks/homebrew-parallax` `.github/workflows/update-preview.yml` | Pull model: tap has its own scheduled workflow that downloads the preview release, independently verifies checksums + cosign + SBOM + `gh attestation verify`, then rewrites the formula with its own token | pull-verified tap (no cross-repo secret), apple-darwin build lanes on `macos-latest`, zigbuild glibc-2.17 Linux lanes, `--deny-self-hosted-runners` attestation check, non-cancelling concurrency group | `cargo xtask release-package/verify/validate`, cosign + CycloneDX SBOM (parity is a recorded follow-up, not v1 — TableRock has no xtask crate) |
| jackin | `AGENTS.md` names it a **read-only reference**; not available locally or via public API. The locally available `jackin-agent-brown` publishes a Docker image — different distribution shape | — | — |

Chosen model: **parallax pull-verified tap** (no operator-provisioned
cross-repo token; the tap trusts only what it can independently verify) with
**holla-simplicity artifacts** (tar.gz/zip + sha256 + GitHub artifact
attestation; no cosign/SBOM in v1).

## Current state

- CI: `.github/workflows/checks.yml` (name: **Checks**) on push to main +
  dispatch; cargo-only, no Swift build anywhere in CI.
  `.github/workflows/dependencies.yml` audits action-pin freshness via
  `gh api` (plan 001 step 2 pattern — new actions must be added to it).
- Workspace version `0.1.0` (`Cargo.toml:6`), no per-crate drift.
- CLI binary is named **`tablerock-cli`** (no `[[bin]]` section in
  `crates/tablerock-cli/Cargo.toml`) and has **no `--version` flag** —
  `main.rs` calls straight into `tablerock_cli::run_caught()` which starts
  the TUI. `plans/README.md` "Findings considered and rejected" rejects
  **clap/arg parsing**, not version identity; a hand-rolled print-and-exit
  fast-path does not reopen that decision (record this distinction in the
  evidence doc).
- Tests spawning the binary by name: `tests/process_contract.rs:8` and
  `tests/pty_lifecycle.rs:160` use `env!("CARGO_BIN_EXE_tablerock-cli")`.
- Native app: `scripts/build-native-app.sh` → ad-hoc-signed
  `native/dist/TableRock.app`, **arm64-only**, `-target
  arm64-apple-macos26.0`, direct swiftc (no SwiftPM), links
  `libtablerock_ffi.dylib`. Info.plist hardcodes
  `CFBundleShortVersionString` `0.1.0` and `CFBundleVersion` `1`
  (`scripts/build-native-app.sh:70-85`). Requires Xcode/macOS 26 SDK
  (evidence 406: Xcode 26.6).
- Plan 019 distribution gate (Developer ID + notarization) is **BLOCKED on
  operator credentials** — the preview cask must ship the ad-hoc shape with
  an explicit Gatekeeper caveat until that unblocks.
- Version-override precedent: holla's `build.rs` reads
  `HOLLA_VERSION_OVERRIDE` at compile time with
  `cargo:rerun-if-env-changed`; CI passes
  `0.1.0-preview.<commit-count>+<short-sha>`.
- Homebrew tap precedent: `homebrew-holla`/`homebrew-parallax` layout is
  `Formula/<name>-preview.rb` + `Aliases/<name>@preview` symlink + README
  documenting `brew tap` → `brew trust` → `brew install`. Neither has a
  `Casks/` directory — the cask is new ground; keep it minimal.
- Evidence frontier: 541 (`docs/evidence/README.md`); next doc is 542+.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Version fast-path | `cargo run -p tablerock-cli -- --version` | `tablerock 0.1.0`, exit 0, no TUI init |
| Override check | `TABLEROCK_VERSION_OVERRIDE=0.1.0-preview.1+abc1234 cargo build -p tablerock-cli` then `target/debug/tablerock --version` | prints override string |
| CLI tests | `cargo test -p tablerock-cli` | all pass (incl. renamed `CARGO_BIN_EXE_tablerock`) |
| Native app build | `TABLEROCK_APP_VERSION=0.1.0-preview.1+abc1234 ./scripts/build-native-app.sh` | app builds; `plutil -p native/dist/TableRock.app/Contents/Info.plist` shows the version |
| App zip | `ditto -c -k --keepParent native/dist/TableRock.app tablerock-app-aarch64-apple-darwin.zip` | deterministic-enough zip for sha256 publishing |
| Preview run | `gh run list --workflow=preview.yml --limit 1` | `success` |
| Formula install | `brew tap tailrocks/tablerock && brew trust tailrocks/tablerock && brew install tablerock@preview` | `tablerock --version` prints preview version |
| Cask install | `brew install --cask tablerock-app@preview` | `/Applications/TableRock.app` present |
| Runner probe | `gh api /repos/actions/runner-images` or a throwaway dispatch job with `runs-on: macos-26` | image exists (STOP condition if not) |

## Scope

**In scope — `tailrocks/tablerock`** (the only files you should
create/modify here):
- `crates/tablerock-cli/Cargo.toml` — add `[[bin]] name = "tablerock"`,
  `path = "src/main.rs"`
- `crates/tablerock-cli/build.rs` (create) — `TABLEROCK_VERSION_OVERRIDE`
  compile-time env plumbing (copy holla's `build.rs` shape)
- `crates/tablerock-cli/src/main.rs` — hand-rolled `--version`/`-V`
  fast-path before `run_caught()` (no clap; match on `std::env::args`)
- `crates/tablerock-cli/tests/process_contract.rs`,
  `crates/tablerock-cli/tests/pty_lifecycle.rs` — `CARGO_BIN_EXE_tablerock`
  rename + a version-output assertion in `process_contract.rs`
- `scripts/build-native-app.sh` — accept `TABLEROCK_APP_VERSION`
  (CFBundleShortVersionString stays `0.1.0`; put the full preview string in
  a new `TableRockPreviewVersion` Info.plist key) and
  `TABLEROCK_BUNDLE_VERSION` (CFBundleVersion, default `1`)
- `.github/workflows/preview.yml` (create)
- `.github/workflows/dependencies.yml` — extend stale-pin audit for every
  newly introduced action
- `docs/evidence/` — one new numbered doc + index line
- `plans/README.md` — status row

**In scope — new repo `tailrocks/homebrew-tablerock`** (operator creates the
repo; you seed it, trunk-only rules apply there too):
- `README.md` (tap/trust/install instructions, both lanes), `LICENSE`
  (Apache-2.0, copy from homebrew-holla)
- `Formula/tablerock-preview.rb` + `Aliases/tablerock@preview` symlink
- `Casks/tablerock-app@preview.rb`
- `.github/workflows/update-preview.yml` (pull-verified model)

**Out of scope**:
- Stable release channel (`Formula/tablerock.rb`, versioned tags) — first
  tagged release owns it; add `conflicts_with` wiring then.
- Developer ID signing / notarization / stapling — plan 019's operator
  gate; this plan ships the ad-hoc shape and upgrades later (see
  Maintenance notes).
- cosign signatures + CycloneDX SBOM parity with parallax — recorded
  follow-up, needs an xtask-equivalent decision first.
- x86_64 / universal native app slice (build script is arm64-only today;
  widening it is plan-021 territory).
- Linux system packages (holla-apt precedent) — not requested.
- Any clap/arg-parsing framework adoption.

## Git workflow

Both repos are trunk-only (`AGENTS.md`): work directly on `main`, never
branch, never open a PR (the tap's update workflow commits directly via
`gh api PUT`, like homebrew-parallax — no PR machinery). Conventional
Commits, DCO sign-off (`git commit -s`), push each commit immediately.
Suggested subjects: `feat(cli): add release version identity`,
`ci: add rolling preview release workflow`, tap: `init: seed preview
formula, cask, and pull-verify workflow`.

## Steps

### Step 1: Release identity in the binary

1. `[[bin]]` rename `tablerock-cli` → `tablerock`; update both
   `CARGO_BIN_EXE_tablerock-cli` references.
2. `build.rs`: emit `TABLEROCK_VERSION` from `TABLEROCK_VERSION_OVERRIDE`
   env (fallback `CARGO_PKG_VERSION`), with
   `cargo:rerun-if-env-changed=TABLEROCK_VERSION_OVERRIDE` — mirror
   holla's `build.rs`.
3. `main.rs`: if the first arg is `--version` or `-V`, print
   `tablerock {env!("TABLEROCK_VERSION")}` and exit 0 **before** any TUI/
   terminal work. Unknown args keep current behavior (start TUI) — do not
   grow an arg surface.
4. Extend `process_contract.rs` with a test asserting the version output.

**Verify**: commands table rows 1–3 all pass; `cargo fmt --all --check &&
cargo clippy --workspace --all-targets` clean.

### Step 2: Parameterize native app version stamping

`scripts/build-native-app.sh`: replace the hardcoded plist literals with
`TABLEROCK_APP_VERSION` (default `0.1.0`, written to a
`TableRockPreviewVersion` key; `CFBundleShortVersionString` stays plain
`x.y.z` — Apple wants numeric triples there) and
`TABLEROCK_BUNDLE_VERSION` (default `1`, written to `CFBundleVersion`).
No behavior change when the envs are unset.

**Verify**: commands table row 4; a plain `./scripts/build-native-app.sh`
still produces a byte-identical plist apart from nothing (defaults path).

### Step 3: Author `.github/workflows/preview.yml`

Trigger: `workflow_run` on **Checks** (`types: [completed]`,
`branches: [main]`) + `workflow_dispatch`. Copy parallax's guard condition
verbatim (success + push event + same-repo + main). Concurrency: group
`tablerock-preview-publish`, `cancel-in-progress: false` (parallax
rationale comment). `permissions: contents: read` at top; escalate per-job.

Jobs:
1. **source-changed** (ubuntu): resolve source SHA (dispatch vs
   workflow_run), fetch
   `https://raw.githubusercontent.com/tailrocks/homebrew-tablerock/main/Formula/tablerock-preview.rb`,
   extract the `# source-sha:` marker, classify
   `git diff --name-only old...new` against:
   `.github/workflows/preview.yml`, `Cargo.toml`, `Cargo.lock`,
   `crates/**`, `native/**`, `scripts/build-native-app.sh`,
   `scripts/generate-swift-bindings.sh`. Fall open on any fetch/diff
   failure (parallax pattern). Compute
   `version=0.1.0-preview.<GraphQL commit count>+<short-sha>` (copy the
   GraphQL query block).
2. **build-cli** (matrix): `aarch64-apple-darwin` + `x86_64-apple-darwin`
   on `macos-latest` with plain cargo; `aarch64-unknown-linux-gnu` +
   `x86_64-unknown-linux-gnu` on `ubuntu-latest` with
   `cargo zigbuild --target <target>.2.17` (glibc floor, parallax
   precedent). Build `-p tablerock-cli --release --locked` with
   `TABLEROCK_VERSION_OVERRIDE=<version>`; package
   `tablerock-<target>.tar.gz` + `.sha256`; attest with
   `actions/attest-build-provenance` (needs `id-token: write`,
   `attestations: write`); upload artifact.
3. **build-app** (`runs-on: macos-26` — see STOP): run Step-2's script with
   `TABLEROCK_APP_VERSION=<version>`,
   `TABLEROCK_BUNDLE_VERSION=<commit count>`; `ditto -c -k --keepParent`
   into `tablerock-app-aarch64-apple-darwin.zip` + `.sha256`; attest;
   upload.
4. **publish** (ubuntu, `contents: write`): download all artifacts, verify
   source SHA is on main (holla's compare-API guard), then
   `gh release edit/create preview` — prerelease, `--target <sha>`, title
   `Preview <version>`, `--clobber` upload of all 10 assets (five archives
   plus five checksum files; holla's
   view/edit/create fallback block verbatim).

Every action SHA-pinned with a `# vX.Y.Z` comment (repo convention).

**Verify**: `workflow_dispatch` the workflow; all jobs green;
`gh release view preview` lists 10 assets with the expected version in the
title.

### Step 4: Extend the stale-pin audit

Add every action introduced in Step 3 (`upload/download-artifact`,
`attest-build-provenance`, zigbuild/zig setup, cache/sccache if adopted) to
the `gh api` freshness assertions in `dependencies.yml`, following the
existing pattern.

**Verify**: every distinct action SHA across all three workflows has a
matching freshness assertion.

### Step 5: Create and seed `tailrocks/homebrew-tablerock`

Operator action first: create the public repo (Actions enabled, default
`GITHUB_TOKEN` workflow permissions: read+write). **STOP if you cannot get
the repo created.** Then seed:

1. `Formula/tablerock-preview.rb` — parallax-preview shape: leading
   `# source-sha:` marker, `class TablerockPreview`, `version`,
   `on_macos`/`on_linux` × `on_arm`/`on_intel` blocks pointing at
   `https://github.com/tailrocks/tablerock/releases/download/preview/tablerock-<target>.tar.gz`,
   `bin.install "tablerock"`, test block asserting
   `shell_output("#{bin}/tablerock --version")` contains `version.to_s`.
   `Aliases/tablerock@preview` → `../Formula/tablerock-preview.rb`.
2. `Casks/tablerock-app@preview.rb` — token **`tablerock-app@preview`**
   (deliberately NOT `tablerock@preview`: that token is the formula alias;
   same-token formula/cask forces `--cask` disambiguation everywhere).
   Contents: `version` (if `brew audit` rejects the `+` build-metadata
   character, use the `,`-separated cask idiom `0.1.0-preview.N,shortsha`),
   `sha256`, `url` to the app zip, `depends_on macos: ">= :tahoe"`,
   `depends_on arch: :arm64`, `app "TableRock.app"`, `zap trash:
   "~/Library/Application Support/TableRock"`, and a `caveats` block
   stating: preview is **ad-hoc signed, not notarized** (plan 019 gate);
   first launch requires right-click → Open or
   `xattr -dr com.apple.quarantine /Applications/TableRock.app`.
3. `.github/workflows/update-preview.yml` — homebrew-parallax's
   pull-verify shape, simplified to this plan's artifact set: cron
   (`*/30`-ish offset minutes) + dispatch; `gh release view preview --repo
   tailrocks/tablerock --json name,targetCommitish`; validate version
   regex + 40-hex SHA; download all assets; assert the exact 10-asset set;
   sha256 each; `gh attestation verify --repo tailrocks/tablerock
   --source-ref refs/heads/main --source-digest <sha>
   --deny-self-hosted-runners` on each of the five archives; run the Linux x86 binary
   and assert `--version` output equals the release version; unzip the app
   archive and assert `TableRock.app/Contents/MacOS/TableRock` exists +
   Info.plist contains the version string (structural check only — ubuntu
   cannot launch it); rewrite **both** the formula and the cask from
   templates; commit via `gh api PUT` contents API with its own
   `github.token` (no PAT).
4. `README.md`: `brew tap tailrocks/tablerock`, `brew trust
   tailrocks/tablerock` (holla README records that untrusted taps refuse to
   load), `brew install tablerock@preview`, `brew install --cask
   tablerock-app@preview`.

**Verify**: tap workflow dispatch run green; both `Formula/` and `Casks/`
files carry the current source-sha marker.

### Step 6: End-to-end verification on a real Mac

Commands table rows 8–9: tap, trust, install formula → `tablerock
--version` matches the release version; install cask → app in
`/Applications`, launches after the documented quarantine step, About/
window title sane; `brew uninstall tablerock@preview` and `brew uninstall
--cask tablerock-app@preview` both clean. Record the full transcript for
the evidence doc.

### Step 7: Evidence + bookkeeping

Next-numbered evidence doc (frontier 541 at planning time) under
`docs/evidence/delivery/`: decision record (pull-verified tap model,
push-model rejection rationale, cask/formula token split, `--version`
fast-path vs the clap rejection, ad-hoc Gatekeeper caveat), workflow
inventory, verification transcript, and named exclusions (cosign/SBOM,
stable channel, x86_64 app). Index line in `docs/evidence/README.md`.
Update the plan 022 row in `plans/README.md`.

## Test plan

- New Rust test: `process_contract.rs` version-output assertion (Step 1).
- Everything else is verified by execution: preview.yml green on dispatch
  **and** on the next organic Checks-completed trigger; tap update
  workflow green; brew install/uninstall transcript on a real arm64 Mac
  (Step 6). The tap's verify job doubles as the standing regression test —
  it fails loudly if the release and formulas drift.

## Done criteria

- [ ] `tablerock --version` prints overridable release identity; tests green
- [ ] `preview.yml` green from both `workflow_dispatch` and a real
      Checks-completed trigger; rolling `preview` prerelease has 10 release
      assets and all five archives are attested
- [ ] All new action pins covered by the dependencies.yml freshness audit
- [ ] `tailrocks/homebrew-tablerock` exists with formula + alias + cask +
      pull-verify workflow, all carrying the current source-sha
- [ ] `brew install tablerock@preview` and `brew install --cask
      tablerock-app@preview` both verified on a real arm64 Mac
- [ ] Cask caveats accurately describe the ad-hoc/Gatekeeper state
- [ ] Evidence doc added + indexed; `plans/README.md` row updated
- [ ] No files outside the in-scope lists modified in either repo

## STOP conditions

- No GitHub-hosted runner image provides Xcode 26 / the macOS 26 SDK
  (probe `macos-26` first) — STOP; self-hosted runners are an operator
  decision, and the tap's `--deny-self-hosted-runners` check would need a
  recorded revision.
- Operator cannot create `tailrocks/homebrew-tablerock` — STOP (ship
  nothing tap-side; do not park formulas in the product repo).
- Gatekeeper on current macOS 26 refuses to run the quarantine-stripped
  ad-hoc app at all — record it, keep the CLI formula lane, STOP the cask
  lane and report (it then waits on plan 019 credentials).
- You find yourself adding clap, a config surface, or any second CLI flag
  beyond `--version`/`-V`.
- Checks is red on main — the preview must only ever build green commits;
  fix the baseline first.

## Maintenance notes

- **When plan 019's Developer ID gate unblocks**: sign + notarize + staple
  the app in `build-app`, drop the cask caveat, and add
  `xcrun stapler validate`-equivalent verification (`spctl -a -vv`) plus
  notarization checks to the tap's verify job. That flips plan 021's
  "release evidence" from ad-hoc preview to the real distribution shape.
- **First tagged release**: add stable `Formula/tablerock.rb` +
  `Casks/tablerock-app.rb`, then add `conflicts_with` between stable and
  preview (holla/parallax precedent).
- **cosign + SBOM parity with parallax**: follow-up decision (needs an
  xtask-equivalent home); until then GitHub attestation is the only
  provenance chain — the tap must keep `--deny-self-hosted-runners`.
- When `checks.yml` gains/renames jobs, the `workflow_run` trigger keys on
  the workflow **name** (`Checks`) — renaming it silently disables
  previews; the tap's staleness (source-sha stuck) is the detection signal.
- Widening the native app to universal (x86_64 slice) touches
  `build-native-app.sh`, the cask `depends_on arch`, and the asset name —
  one commit, all three together.
