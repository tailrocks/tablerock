# Evidence 631: native external URL authority

Date: 2026-07-22

## Outcome

`TableRock.app` registers one custom `tablerock` scheme and handles only the
`tablerock://open?url=<percent-encoded database URL>` route. Swift validates
the bounded routing envelope; shared Rust then applies the complete database
URL parser. No external event can connect, save, or mutate automatically.

Valid requests open a native authority sheet with password-redacted target
facts. Operator may cancel, review as a new profile, connect a matching saved
profile, or connect temporarily. Temporary connect passes imported TLS intent
to the same backend open path and never persists the draft or secret.

## Root boundary

Apple documents custom URL schemes as an attack vector and requires strict
validation plus limited actions. The implementation follows that constraint:

- one namespaced scheme and route;
- one required `url` parameter, no duplicates;
- 8 KiB outer envelope bound and Rust's 4 KiB inner URL bound;
- redacted summary only;
- explicit operator authority before any action.

Importable feature tests cover percent decoding, wrong scheme/route, path,
fragment, duplicate/extra parameters, empty payload, and pre-parse size bound.

Primary sources:

- <https://developer.apple.com/documentation/swiftui/view/onopenurl(perform:)>
- <https://developer.apple.com/documentation/xcode/defining-a-custom-url-scheme-for-your-app>
- <https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleurltypes>

## Verification

```text
brew info xcodegen
stable 2.46.0

xcodegen generate --spec native/App/project.yml
canonical project regenerated

cd native && rtk swift build -c release
ok (build complete)

rtk git diff --check
exit 0
```

XCUITest launches the shipped surface with the same routed URL, proves no
connection exists before authority, proves the summary excludes the password,
then connects temporarily through the real control. Hosted Xcode execution and
real LaunchServices delivery remain required after push.

## Clean-room provenance

TablePro's public workflow material established only that external connection
handoff is a database-workbench workflow class. No source, tests, strings,
assets, layout measurements, colors, or key bindings were read or copied.
TableRock's route, visual hierarchy, policy, identifiers, and tests are
independent and based on repository requirements plus primary Apple guidance.
