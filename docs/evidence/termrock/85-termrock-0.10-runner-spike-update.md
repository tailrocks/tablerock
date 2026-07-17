# TermRock 0.10 Closure Runner Spike Update

## Upstream change

TableRock now pins exact TermRock `main` revision
`20318221c792ee0a0d0145967321adaee57875ae`. Relative to the prior pin
`4b7927335663e0b275588a93ca0ebe6bc4032b0d`, this revision restructures only
the TermRock lookbook and adds a local closure-runner feasibility spike. It does
not change the published `termrock` crate API and adds no migration file.

The spike separates lookbook-owned model/update/render behavior from terminal
lifecycle, event pumping, and draw cadence. Its accepted plan proposes a future
generic closure runner and removal of the speculative `drive_frame`,
`Component`, `View`, `Dirty`, `UpdateResult`, effect-placeholder, and
subscription APIs. That redesign is not yet implemented on `main`.

## TableRock impact and forward decision

TableRock compiles unchanged at this revision. Its root TEA remains
TableRock-owned, as required: one Model/Message/Update/Effect/Subscription/View
flow, with product effects and subscriptions outside TermRock. Current shell
code then used TermRock `UpdateResult`, `Dirty`, and `drive_frame`; those were
the exact call sites requiring migration when the proposed upstream redesign
landed. Migration 0024 later removed them; TableRock adoption is recorded in
[`130-termrock-closure-runner-frame-time-migration.md`](130-termrock-closure-runner-frame-time-migration.md).

TableRock will adopt the new runner immediately after TermRock publishes it and
its sequential migration. It will replace removed convenience result/frame
types directly rather than preserving compatibility shims. Adopting the local
lookbook prototype now would copy an unpublished experiment and create a second
terminal loop, so no premature implementation is retained.

## Verification

- Exact Git revision resolves in the lockfile.
- TableRock workspace tests, lint, and documentation pass against the new pin.
- No public TermRock signature changed in this revision.
- No TableRock source, test, or behavior migration is required yet.

External concepts: generic terminal runner responsibility boundary
Public sources: <https://github.com/tailrocks/termrock/commit/20318221c792ee0a0d0145967321adaee57875ae>
Implementation source: TermRock upstream lookbook spike and TableRock-owned compatibility inspection
Copied code/assets/text: none
