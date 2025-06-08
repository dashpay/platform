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
                .unsafeFlags([
                    "-L/Users/samuelw/Documents/src/platform/target/release",
                    "-lswift_sdk"
                ])
            ]
        ),
    ]
)