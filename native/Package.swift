// swift-tools-version: 6.2
import PackageDescription

// Resolve absolute path to cargo release output (repo_root/target/release).
let packageDir = Context.packageDirectory
let cargoReleaseLibDir = packageDir + "/../target/release"

let package = Package(
    name: "TableRockBridge",
    platforms: [
        .macOS(.v26),
    ],
    products: [
        .library(name: "TableRockBridge", targets: ["TableRockBridge"]),
        // Proof harness executable (no XCTest — works with Command Line Tools).
        .executable(name: "tablerock-bridge-proof", targets: ["BridgeProof"]),
    ],
    targets: [
        // System library target wrapping the UniFFI C header + module map.
        .systemLibrary(
            name: "tablerock_ffiFFI",
            path: "Generated",
            pkgConfig: nil
        ),
        .target(
            name: "TableRockBridge",
            dependencies: ["tablerock_ffiFFI"],
            path: "Sources/TableRockBridge",
            linkerSettings: [
                // Link the host release dylib built by cargo.
                // XCFramework packaging is the release distribution path
                // (scripts/build-xcframework.sh) when full Xcode is available.
                .linkedLibrary("tablerock_ffi"),
                .unsafeFlags([
                    "-L\(cargoReleaseLibDir)",
                ]),
            ]
        ),
        .executableTarget(
            name: "BridgeProof",
            dependencies: ["TableRockBridge"],
            path: "Sources/BridgeProof"
        ),
        // Native macOS app (plan 020 checkpoint 1). SwiftUI + AppKit on macOS 26;
        // links the cargo release dylib transitively through TableRockBridge.
        .executableTarget(
            name: "TableRockApp",
            dependencies: ["TableRockBridge"],
            path: "Sources/TableRockApp"
        ),
    ]
)
