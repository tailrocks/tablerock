# Main-Branch Delivery

## Non-negotiable workflow

All TableRock work happens directly on `main`.

- Never create, switch to, publish, or retain another branch.
- Never open a pull request.
- Never force-push, rewrite, reset, or delete published `main` history.
- Use small forward commits; repair mistakes with a new forward commit.
- Commit only after the checkpoint's evidence passes.
- Push every completed commit immediately unless the operator explicitly says
  not to push.

The same rule applies when TableRock needs a TermRock change: edit TermRock
directly on its `main`, commit and push it directly, then update TableRock in a
later direct `main` commit. Jackin is a read-only reference in this program.

## TableRock checkpoint sequence

1. Confirm the workspace is on `main`, synchronized, and has no unrelated
   changes.
2. Confirm the roadmap phase/research decision is approved.
3. Record upstream source and clean-room provenance.
4. Implement one coherent, buildable checkpoint.
5. Update tests, research, roadmap status, user documentation, and support claims
   in the same checkpoint.
6. Run the checkpoint's format/lint/test/docs/security/license/performance gates.
7. Review the complete diff and forbidden-data scan.
8. Commit on `main` using Conventional Commits, `git commit -s`, and:

   ```text
   Co-authored-by: Codex <codex@openai.com>
   ```

9. Push `main` immediately.
10. Verify local `main`, remote `main`, and the recorded evidence commit match.

No branch, pull request, merge commit, squash workflow, or review-thread state is
part of delivery. Review happens against the local diff and recorded evidence
before the direct commit.

## TermRock extension sequence

When TableRock needs a reusable component or API absent from TermRock:

1. Define the neutral interaction/render contract from the approved TableRock
   need; remove all database and product vocabulary.
2. Work directly in the TermRock repository on `main`; create no branch or pull
   request.
3. Add or extend the component, public docs, lookbook story, deterministic
   preview, Buffer/input tests, compatibility metadata, and benchmark where hot.
4. Run TermRock format/lint/test/docs/lookbook/compatibility gates and Jackin
   build/tests when an existing API changes.
5. Commit with TermRock's Conventional/DCO/co-author requirements and push
   TermRock `main` immediately.
6. Record the full pushed TermRock commit ID.
7. Return to TableRock `main`, update the exact Git revision and lockfile, run
   TableRock's complete affected suite, update provenance/docs, commit with
   sign-off/co-author, and push TableRock `main`.

Never use a floating TermRock `main` dependency. Never copy the component into
TableRock while waiting. The TermRock commit must be independently reusable by
TableRock, Jackin, and future products; immediate Jackin feature adoption is not
required, but compatibility is.

## Dirty-worktree rule

Existing unrelated changes belong to the operator. Do not absorb, discard,
rewrite, or hide them. Narrow the checkpoint around them. If the same lines
cannot be safely separated, stop and request direction before committing.

## Failed checkpoint

Before commit, fix or revert only the checkpoint's own uncommitted changes. Once
a commit is pushed, preserve history and make a new forward repair. Record failed
spikes and rejected architecture paths in decision history because no pull-
request conversation exists.

## Provenance without pull requests

Every implementation commit or its paired research record names:

- the TableRock requirement/roadmap row;
- official database/client/platform/library sources used;
- clean-room reference category, when a broad external workflow motivated it;
- dependencies, versions, licenses, and generated artifacts introduced;
- verification run and support claim changed;
- rejected path only when needed to explain the fixed decision.

No reference-product screenshot, measurement, text, key binding, asset,
identifier, test, or source-derived implementation is stored as provenance.

## Current research checkpoint

This research update follows the same rule: direct work on `main`, no branch and
no pull request. It must be validated, committed with DCO and the Codex co-author
trailer, pushed immediately, and verified against `origin/main`.
