# Portable real-server CI host capabilities

Date: 2026-07-22

## Failure class

Velnor runs each Linux job inside a minimal root container. CI incorrectly
assumed the GitHub-hosted runner shape: `sudo`, `lsb_release`, and `gpg` were
preinstalled, and the job could mount a tmpfs. Runs 29864379361 and 29865254664
proved the missing tools; run 29866510795 then proved the container correctly
has no `CAP_SYS_ADMIN` mount authority.

These were runner-assumption failures after every preceding real PostgreSQL,
ClickHouse, Redis, UniFFI, import, export, and performance case passed.

## Correction

- Privileged package commands execute directly when the job already has UID 0
  and retain `sudo` elevation on hosted non-root runners.
- PGDG setup installs its repository prerequisites from the distribution first
  and reads the codename from `/etc/os-release`.
- The ENOSPC gate creates a nextest archive for the `tablerock-files` library,
  resolves the active installed executable with `mise which`, and
  copies both into a disposable Ubuntu 26.04 container. It selects only the
  exact fail-closed test on a kernel-backed 1 MiB Docker `--tmpfs`. This
  preserves real ENOSPC behavior without granting host mount authority,
  bypassing the repository's nextest-only test policy, or weakening the
  assertion.
- A trap removes only the uniquely named owned container on every exit path.

## Verification

CI run 29867470205 passed all jobs. Its real-server job passed:

- all engine real-server suites and hostile TLS/restart cases;
- UniFFI live bridge conformance;
- CSV import and streaming export;
- first-row performance budgets;
- PostgreSQL 18 client installation plus dump/restore;
- true ENOSPC fail-closed cleanup inside the Docker tmpfs.

Local checks: `actionlint .github/workflows/ci.yml`, `git diff --check`, and
local nextest archive creation and exact filter selection all pass. Run
29868378990 exposed the initial mise symlink. Exact-head run 29869612537 then
proved `readlink -f` still selected mise's logical shim, which cannot run after
being copied alone. The forward correction asks mise for the real active binary
before crossing the container boundary.

## Provenance

No external product source, test, text, screenshot, layout, measurement, color,
asset, or key binding influenced this CI correction. It derives entirely from
TableRock's existing failure-injection requirement and hosted runner evidence.
