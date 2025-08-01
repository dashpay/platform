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
        // Binary target using the Unified XCFramework
        .binaryTarget(
            name: "DashSDKFFI",
            path: "../rs-sdk-ffi/build/DashSDK.xcframework"
        ),
        // Swift wrapper target
        .target(
            name: "SwiftDashSDK",
            dependencies: ["DashSDKFFI"],
            path: "Sources/SwiftDashSDK"
        ),
    ]
)