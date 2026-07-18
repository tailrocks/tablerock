# Plan 019 residual — software path reconfirm (no packaging)

Date: 2026-07-18
Host: macOS arm64, Command Line Tools only (`xcode-select` → CLT)
Commit: `cc3aac8` / evidence after `18ae012` line

## Software gates re-run (PASS)

| Gate | Command / observation |
|------|------------------------|
| `tablerock-ffi` release | `cargo build -p tablerock-ffi --release` green |
| Swift bridge proof | `cd native && swift run -c release tablerock-bridge-proof` → `bridge-proof: all checks passed` (panic probe contained, open unreachable reject, page v1 bounds, runtime destroy) |
| Universal staticlib | `scripts/build-universal-staticlib.sh` → `lipo` fat `x86_64 arm64`, ~192 MiB at `target/universal/libtablerock_ffi.a` |
| Swift bindings regen | `generate-swift-bindings.sh` — tree clean after regen (no drift) |

## Packaging gates still STOP

| Gate | Observation |
|------|-------------|
| Full Xcode | No `/Applications/Xcode.app`; `xcodebuild` errors: requires Xcode, active dir is CLT |
| Developer ID | `security find-identity -v -p codesigning` → **0** valid identities |
| XCFramework | `scripts/build-xcframework.sh` needs `xcodebuild -create-xcframework` |
| Notarize / staple / clean-machine | Blocked on certs |

## Conclusion

Software/UniFFI/Swift proof path remains green on CLT. Plan 019 **distribution**
exit is still operator-blocked. Plans 020–021 must not start packaging claims
until STOP lifts.

## Resume

1. Install full Xcode, `sudo xcode-select -s /Applications/Xcode.app`
2. `./scripts/build-xcframework.sh`
3. Import Developer ID + notary credentials → sign, notarize, staple
4. Clean-machine Gatekeeper transcript
