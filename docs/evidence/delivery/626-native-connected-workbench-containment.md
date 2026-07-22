# Evidence 626: native connected-workbench containment

Date: 2026-07-22

## Authoritative failure

Native Checkpoint run `29880423048` tested `e2ccfce`. Rust bridge, generated
bindings, Swift feature tests, and universal XCFramework build passed. The
canonical Xcode plan failed five XCUITests:

- CSV reviewed-import Stage control was not exposed by stable identifier;
- result-cell selection did not open the inspector;
- loaded-result export did not complete;
- loaded-row quick filter could not receive keyboard focus;
- next-page activation did not append its terminal page.

The quick-filter accessibility snapshot located the control at
`{{233,663},{222,26}}` while the workbench window ended at y=654. Export fell
back to center `{429,681}`. These are direct proof that result interactions
were laid out outside the operable window, not independent backend failures.

## Root cause and fix

`ContentView` rendered the complete disconnected `New connection` form even
after `sessionHex` established a connected workbench. Query tabs, result
toolbar, grid, and paging were appended below that form. Lowering the grid's
minimum height could not remove the fixed form height, so the split content
still exceeded the window.

Disconnected setup and connected workbench are now mutually exclusive. The
connection form and connecting label render only without a session. This
removes the layout condition that placed every result action below the window.

CSV Stage/Apply and result Export identifiers now follow their button-style
modifiers so the final styled controls own the stable automation identifiers.

## Local verification

```text
mise exec -- rtk cargo test -p tablerock-core --test screen_manifest
cargo test: 1 passed (1 suite, 0.00s)

cd native && rtk swift build -c release
ok (build complete)

swiftc -parse native/Sources/TableRockApp/TableRockApp.swift
exit 0

rtk git diff --check
exit 0
```

Hosted XCUITest remains required after push; this document does not convert the
failed run into passing evidence.
