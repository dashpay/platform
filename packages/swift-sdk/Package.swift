// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "SwiftDashSDK",
    platforms: [
        .iOS(.v18),
        .macOS(.v15)
    ],
    products: [
        .library(
            name: "SwiftDashSDK",
            targets: ["SwiftDashSDK"]),
    ],
    targets: [
        // Binary target using the Unified XCFramework
        .binaryTarget(
            name: "DashSDKFFI",
            path: "DashSDKFFI.xcframework"
        ),
        // Swift wrapper target
        .target(
            name: "SwiftDashSDK",
            dependencies: ["DashSDKFFI"],
            path: "Sources/SwiftDashSDK",
            exclude: ["KeyWallet/README.md", "PlatformWallet/README.md"],
            linkerSettings: [.linkedFramework("SystemConfiguration")]
        ),

        // Unit tests (offline, hermetic)
        .testTarget(
            name: "SwiftDashSDKTests",
            dependencies: ["SwiftDashSDK"],
            path: "SwiftTests/SwiftDashSDKTests"
        ),

        // Integration tests against a local dashmate devnet.
        // Gated by env var `RUN_INTEGRATION_TESTS=1`
        .testTarget(
            name: "SwiftDashSDKIntegrationTests",
            dependencies: ["SwiftDashSDK"],
            path: "SwiftTests/SwiftDashSDKIntegrationTests",
            swiftSettings: [.unsafeFlags(["-warnings-as-errors"])]
        ),
    ],
    swiftLanguageModes: [.v6]
)
