# 543 — Native automation identifiers

Date: 2026-07-21

## Decision

Plan 021 checkpoint 13 now has one stable, nonlocalized automation identity
for every required native surface. Static surfaces use the canonical plan IDs;
durable profile, query-tab, object-tab, and catalog-node surfaces append their
owned stable identity rather than a row position or visible title.

The architecture-level cause of the gap was that the native accessibility gate
validated human-readable labels but had no identifier inventory. The same gate
now rejects removal of the canonical identifiers, so future presentation-text
changes cannot silently break automation discovery.

## Surface map

| Surface | Identifier |
|---|---|
| Workbench window/root | `window.workbench` |
| Profile sidebar/create | `sidebar.profiles`, `profile.add` |
| Profile editor | `profile.editor.name`, `profile.editor.save` |
| Durable profile row | `profile.<profile-id-hex>` |
| Catalog outline/node | `catalog.outline`, `catalog.node.<opaque-node-key>` |
| Query editor/actions/status | `query.editor`, `query.run`, `query.cancel`, `query.status` |
| Result grid/paging | `results.grid`, `results.next-page` |
| Durable tabs | `query.tab.<uuid>`, `object.tab.<uuid>` |

`query.status` is a persistent surface: it renders `Idle` before an operation
and then the Rust-owned summary, cancellation outcome, or redacted error. This
avoids both absent-id and duplicate-id ambiguity.

AppKit wrappers retain human-readable accessibility labels and values while
adding `setAccessibilityIdentifier`; SwiftUI controls use
`accessibilityIdentifier`. Apple documents identifiers as test identities that
are not user-visible. Public sources:

- https://developer.apple.com/documentation/swiftui/view/accessibilityidentifier(_:)
- https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol

Copied code, assets, text, geometry, colors, or key bindings: none.

## Verification

```text
bash -n scripts/verify-native-accessibility.sh
env PATH=/Users/donbeave/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin \
  ./scripts/verify-native-accessibility.sh
```

Result: strict Release build succeeded; the signed local app launched; the
custom-control runtime audit and the expanded structural identifier inventory
passed.

## Bounds and remaining work

This checkpoint establishes the stable automation contract. Plan 021
checkpoint 14 still owns the canonical Xcode application, XCUITest target, and
behavioral tests that consume these identifiers. Structural grep remains a
policy gate, not a substitute for those user-operable tests.
