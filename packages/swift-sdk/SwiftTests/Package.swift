// swift-tools-version:5.5
import PackageDescription

let package = Package(
    name: "SwiftDashSDKTests",
    platforms: [
        .macOS(.v10_15),
        .iOS(.v13)
    ],
    products: [
        .library(
            name: "SwiftDashSDKTests",
            targets: ["SwiftDashSDKTests"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "SwiftDashSDKMock",
            dependencies: [],
            path: "Sources/SwiftDashSDKMock",
            publicHeadersPath: "."
        ),
        .testTarget(
            name: "SwiftDashSDKTests",
            dependencies: ["SwiftDashSDKMock"],
            path: "Tests/SwiftDashSDKTests",
            exclude: ["*.o", "*.d", "*.swiftdeps"]
        ),
    ]
)