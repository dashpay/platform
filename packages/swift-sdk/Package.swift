// swift-tools-version: 5.8

import PackageDescription

let package = Package(
    name: "SwiftDashSDK",
    platforms: [
        .iOS(.v16),
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "SwiftDashSDK",
            targets: ["SwiftDashSDK"]),
    ],
    targets: [
        // System library target for the C bindings
        .systemLibrary(
            name: "CSwiftDashSDK",
            path: "Sources/CSwiftDashSDK"
        ),
        // Swift wrapper target
        .target(
            name: "SwiftDashSDK",
            dependencies: ["CSwiftDashSDK"],
            path: "Sources/SwiftDashSDK",
            linkerSettings: [
                .linkedLibrary("swift_sdk", .when(platforms: [.iOS])),
                .unsafeFlags([
                    "-L/Users/samuelw/Documents/src/platform/target/aarch64-apple-ios-sim/release",
                    "-Xlinker", "-force_load",
                    "-Xlinker", "/Users/samuelw/Documents/src/platform/target/aarch64-apple-ios-sim/release/libswift_sdk.a"
                ])
            ]
        ),
    ]
)