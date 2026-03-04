// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "SwiftDashSDK",
    platforms: [
        .iOS(.v17),
        .macOS(.v14)
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
            exclude: ["KeyWallet/README.md"]
        ),
    ],
    swiftLanguageModes: [.v6]
)
