# Evidence 632: native quick switcher

Date: 2026-07-22

## Outcome

Native Query menu now exposes Quick Switcher with `Command-Shift-O`. One
searchable sheet projects current saved connections, query tabs, object tabs,
loaded catalog objects, and saved queries. Favorites and pinned objects rank
first; exact and prefix title matches rank before contains matches.

Every candidate carries a stable profile ID, UUID, catalog-node key, or saved
query ID. Activation resolves that identity against current model state rather
than retaining a row index. Removed or replaced candidates therefore fail
closed instead of opening another item.

## Verification

```text
cd native && rtk swift build -c release
ok (build complete)

mise exec -- cargo test -p tablerock-core --test screen_manifest
1 passed

rtk git diff --check
exit 0
```

XCUITest opens the shipped command with `Command-Shift-O`, searches two
independent query tabs, activates Users, and observes the editor change from
the Orders statement to the Users statement. Hosted Xcode execution remains
required after push.

## Clean-room provenance

TablePro's current public keyboard-shortcut documentation was reviewed only to
confirm broad searchable navigation across tables, views, databases, schemas,
and recent queries. No source, tests, strings, assets, colors, geometry,
measurements, or key bindings were copied. TableRock's shortcut, categories,
ranking, stable-ID resolution, layout, and tests are independently defined.
