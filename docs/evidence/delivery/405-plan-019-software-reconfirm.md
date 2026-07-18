# Plan 019 residual — software path reconfirm (post residual 404)

Date: 2026-07-18  
Host: macOS arm64, Command Line Tools only  
Commit: `2bd1a93`

## Software gates (PASS)

| Gate | Observation |
|------|-------------|
| `cargo build -p tablerock-ffi --release` | green (~22s) |
| Swift bridge proof | `swift run -c release tablerock-bridge-proof` → `bridge-proof: all checks passed` (panic probe contained, open unreachable reject, page v1 bounds, runtime destroy) |
| `tablerock-tui --lib` | 199 tests green (residual stretch) |

## Packaging gates still STOP

| Gate | Observation |
|------|-------------|
| Full Xcode | `xcode-select` → `/Library/Developer/CommandLineTools` only |
| Developer ID | `security find-identity -v -p codesigning` → **0** valid identities |
| XCFramework / notarize | blocked on Xcode + certs |

## Conclusion

UniFFI/Swift software path remains green. Plan 019 distribution exit remains
operator-blocked. Residual software loop continues on trunk; 020–021 packaging
claims must not start until STOP lifts.

## Resume when unblocked

1. Install full Xcode; `sudo xcode-select -s /Applications/Xcode.app`
2. `./scripts/build-xcframework.sh`
3. Import Developer ID + notary credentials → sign, notarize, staple
4. Clean-machine Gatekeeper transcript
