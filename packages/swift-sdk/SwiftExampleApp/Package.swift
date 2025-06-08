// swift-tools-version: 5.8

import PackageDescription

let package = Package(
    name: "SwiftExampleApp",
    platforms: [
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "SwiftExampleApp",
            targets: ["SwiftExampleApp"]),
    ],
    dependencies: [
        .package(path: "../")
    ],
    targets: [
        .target(
            name: "SwiftExampleApp",
            dependencies: [
                .product(name: "SwiftDashSDK", package: "swift-sdk")
            ],
            path: "Sources"
        ),
        .testTarget(
            name: "SwiftExampleAppTests",
            dependencies: ["SwiftExampleApp"],
            path: "Tests"
        ),
    ]
)