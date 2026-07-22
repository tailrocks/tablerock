# Evidence 634: hosted control and runner isolation

Date: 2026-07-22

## Outcome

Four native controls that existed in the model but failed canonical XCUITest
operation now expose deterministic macOS accessibility actions:

- result cells use the standard `NSButton` target/action path, including AX
  press, before projecting selection into the value inspector;
- external-URL authority actions live in the sheet body instead of a toolbar
  group that could collapse out of the accessibility tree;
- Quick Switcher owns an identified search text field instead of relying on
  whichever `.searchable` field XCUITest enumerates first;
- Explain proof selects the identified application menu command and waits for
  its connected-session enablement before invoking it.

The repeated Velnor PTY timeout was isolated to overlap between the Rust job
and the real-server integration job on the same self-hosted machine. The same
SHA passed the complete GitHub-hosted Rust job, and the exact stress test passed
both macOS and a clean Linux Rust 1.97.1 container. CI now makes integration
depend on the Rust job. This removes cross-job server/container load from the
30-second terminal-starvation proof without reducing its event flood, changing
its timeout, or adding retries.

## Verification

```text
mise exec -- swift build --package-path native -c release
Build complete

mise exec -- cargo test -p tablerock-cli --test pty_lifecycle -- --nocapture
4 passed

docker run ... rust:1.97.1-bookworm cargo test -p tablerock-cli \
  --test pty_lifecycle high_rate_mouse_and_resize_do_not_starve_terminal_quit
1 passed

GitHub-hosted CI run 29886270653, Format/lint/test
success

mise exec -- cargo clippy -p tablerock-cli -p tablerock-engine --tests -- -D warnings
green

mise exec -- actionlint .github/workflows/ci.yml
green

mise exec -- cargo fmt --all --check
green

git diff --check
green
```

Local Swift XCTest remains unavailable because this host's Command Line Tools
SDK does not provide `XCTest`. Canonical Xcode/XCUITest and the serialized
Velnor lane remain required after push.

## Primary sources

- GitHub Actions workflow syntax for `jobs.<job_id>.needs`:
  <https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idneeds>
- Apple controls and XCUITest behavior are verified by the repository's
  canonical hosted Xcode project rather than inferred from local SDK behavior.

## Clean-room provenance

TablePro public documentation was checked only for broad workflow existence:
external links require confirmation, connection/query switching is searchable,
and Explain is a query-workbench action. No source, tests, strings, assets,
geometry, measurements, colors, layout, or key bindings were copied. TableRock
control placement, identifiers, Rust ownership, authority rules, and tests are
independently defined from repository requirements and direct failure evidence.
