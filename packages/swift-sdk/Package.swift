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
            path: "../../../dashpay-ios/DashPayiOS/Libraries/DashUnifiedSDK.xcframework"
        ),
        // Swift wrapper target
        .target(
            name: "SwiftDashSDK",
            dependencies: ["DashSDKFFI"],
            path: "Sources/SwiftDashSDK"
        ),
    ]
)