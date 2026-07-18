# Plan 020 — appearance fixtures and accessibility gate

Date: 2026-07-19  
SDK: Xcode 26.6 / macOS 26.5 SDK

## Checkpoint

The app now accepts test-only launch environment fixtures for light/dark,
app-scoped high contrast, opaque reduced-transparency fallback, reduced motion,
and differentiate-without-color labeling. Normal launches receive no overrides
and continue following system settings. Accessibility environment values are
read-only system facts; no global preferences are written or falsified.

`capture-native-appearance.sh` launches fresh app instances through
LaunchServices, identifies the exact process-owned CGWindow without assistive
access, captures eight window-only light/dark × contrast × transparency
variants, terminates only its own process, and writes SHA-256 hashes.

`verify-native-accessibility.sh` enforces semantic labels/values on the custom
AppKit outline, grid, editor, and toolbar actions, and rejects custom visual
effects, blur, GCD, and the legacy observation stack.

## Evidence

| Gate | Observation |
|------|-------------|
| `./scripts/build-native-app.sh` | PASS; strict Swift 6 complete concurrency, warnings-as-errors |
| `./scripts/verify-native-accessibility.sh` | PASS |
| `./scripts/capture-native-appearance.sh` | PASS; eight distinct PNGs + hashes |
| visual inspection | representative standard/high-contrast opaque variants remain legible in light and dark; labels and focused fields remain distinguishable |
| custom material audit | no `NSVisualEffectView`, `.blur`, or custom toolbar background |

Artifacts and hashes: [414-native-appearance](../artifacts/414-native-appearance/).

## Bounds

These are deterministic app-scoped fixtures, not proof that macOS system
accessibility preferences were changed. The real Reduce Transparency, Increase
Contrast, Reduce Motion, VoiceOver traversal, keyboard focus, and large-content
matrix under a separately configured user/VM remains open. macOS assistive
access is still unavailable to automation on this host. No screenshot or
protected expression from TablePro or any other external product influenced
these artifacts; TablePro establishes only the broad workbench workflow concept.
