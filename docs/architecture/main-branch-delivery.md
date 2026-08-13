# Pull-Request Delivery

## Non-negotiable workflow

All TableRock changes enter `main` through a pull request.

- Never push directly to `main`.
- Keep one goal on one authorized branch and in one pull request. The current
  branch and goal-specific restriction live in [`AGENTS.md`](../../AGENTS.md).
- Never create a duplicate or concurrent branch or pull request for that goal.
- Keep the published branch buildable through small forward commits. Repair
  mistakes forward; do not force-push, rebase, reset, or rewrite its history.
- Merge current `main` into a published branch when branch protection reports
  it behind, then rerun the affected evidence gates.
- Push every completed checkpoint immediately unless the operator explicitly
  asks to hold it.

Required TermRock work follows the same reviewed-delivery constraint and lands
before TableRock pins the exact revision. Jackin remains a read-only reference.

## TableRock checkpoint sequence

1. Confirm the workspace is on the authorized branch, synchronized with its
   remote, includes current `origin/main`, and has no unrelated changes.
2. Confirm the roadmap phase/research decision is approved.
3. Record upstream sources and clean-room provenance where applicable.
4. Implement one coherent, buildable checkpoint.
5. Update tests, architecture, roadmap status, user documentation, support
   claims, and evidence together when behavior changes.
6. Run the checkpoint's format, lint, test, documentation, security, license,
   and performance gates as required by the changed surface.
7. Review the complete diff and forbidden-data scan.
8. Commit with a Conventional Commit subject, DCO sign-off, and:

   ```text
   Co-authored-by: Codex <codex@openai.com>
   ```

9. Push the checkpoint to the same pull-request branch immediately.
10. Verify remote checks against that exact commit. If `main` advanced, merge it
    forward, push, and rerun checks before merge.
11. Merge only after required checks and evidence pass. Do not create another
    branch or pull request to work around a failing gate.

## TermRock extension sequence

When TableRock needs a reusable component or API absent from TermRock:

1. Define a neutral interaction/render contract from the approved TableRock
   need; remove database and product vocabulary.
2. Follow TermRock's repository rules and TableRock's requirement that the
   change receive pull-request review. Do not modify Jackin.
3. Add or extend public docs, lookbook coverage, deterministic previews, Buffer
   and input tests, compatibility metadata, and benchmarks where hot.
4. Run TermRock format, lint, test, docs, lookbook, and compatibility gates;
   run Jackin build/tests only when an existing public API changes.
5. Merge the reviewed TermRock change and record its full commit ID.
6. Return to the existing TableRock pull-request branch, update the exact Git
   revision and lockfile, run TableRock's affected suite, update provenance and
   docs, commit, and push.

Never use a floating TermRock `main` dependency. Never copy a missing component
into TableRock while waiting. The TermRock change must remain independently
reusable by TableRock, Jackin, and future products.

## Dirty-worktree rule

Existing unrelated changes belong to the operator. Do not absorb, discard,
rewrite, or hide them. Narrow the checkpoint around them. If the same lines
cannot be safely separated, stop and request direction before committing.

## Failed checkpoint

Before commit, fix or revert only the checkpoint's own uncommitted changes. Once
a commit is pushed, preserve history and make a new forward repair. Record
failed spikes and rejected architecture paths in decision history; the pull
request preserves review context but does not replace repository evidence.

## Provenance

Every influenced implementation commit or its paired requirement/test record
names:

- the TableRock requirement or roadmap row;
- official database, client, platform, and library sources used;
- the clean-room reference category when an external workflow motivated it;
- dependencies, versions, licenses, and generated artifacts introduced;
- verification run and support claim changed;
- rejected paths only when needed to explain the fixed decision.

No reference-product screenshot, measurement, text, key binding, asset,
identifier, test, or source-derived implementation is stored as provenance.
