# Design-lab contracts

## Ownership

`TableRockDesignLab` is a standalone Swift executable and Xcode application.
Its Swift Package target has an empty dependency list. The Xcode target has no
dependency on `TableRock`, `TableRockFeature`, `TableRockBridge`, the UniFFI
XCFramework, Rust artifacts, or production resources.

Lab code may import only Apple UI/foundation frameworks. All displayed data is
invented and immutable. View-local selection and disclosure state is transient
and does not leave the process.

## Stable comparison dimensions

```text
concept:
  native-workbench | query-studio | column-observatory |
  grid-canvas | change-desk

surface:
  connections | setup | data-grid | sql-results | change-review

appearance:
  system | light | dark

accessibility:
  system | reduce-transparency | increase-contrast | reduce-motion
```

Launch arguments use `--concept`, `--surface`, `--appearance`,
`--accessibility`, and `--capture`. Invalid or missing values fall back to
Native Workbench, Data Grid, system appearance, and system accessibility.
Capture mode hides lab controls but does not change concept content.

## Isolation proof

The repository check must fail if the lab source contains any production
module import or forbidden production/backend symbol. It must also assert the
Swift Package and generated Xcode target dependency lists remain empty, then
build and test the lab independently.

Forbidden concepts include:

```text
TableRockBridge  TableRockFeature  tablerock_ffi  UniFFI  Rust
BridgeModel      WorkbenchBackend  persistence    credential
URLSession       Network.framework
```

The word list is defense in depth, not the module boundary itself.

## Capture contract

- Fixed 1440×900 content size at 2× display scale when available.
- One filename per concept, surface, appearance, and accessibility mode.
- At minimum: all five surfaces for each concept in light; the Data Grid and
  SQL + Results surfaces in dark; the lead Data Grid in all three explicit
  accessibility previews.
- No real connection names, credentials, SQL, values, usernames, hostnames, or
  application data may appear.
- Capture manifest records git revision, host, Xcode/SDK, launch arguments,
  dimensions, and checksum.

## Gate contract

Captures and verification open the first gate; they do not close it. Work must
stop with production UI unchanged. Operator concept selection is required,
followed by a separate refined-concept confirmation before migration.
