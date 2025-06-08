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
        // System library target for the rs-sdk-ffi bindings
        .systemLibrary(
            name: "CDashSDKFFI",
            path: "Sources/CDashSDKFFI"
        ),
        // Swift wrapper target
        .target(
            name: "SwiftDashSDK",
            dependencies: ["CDashSDKFFI"],
            path: "Sources/SwiftDashSDK",
            linkerSettings: [
                .linkedLibrary("rs_sdk_ffi", .when(platforms: [.iOS])),
                .unsafeFlags([
                    "-L/Users/samuelw/Documents/src/platform/target/aarch64-apple-ios-sim/release",
                    "-Xlinker", "-force_load",
                    "-Xlinker", "/Users/samuelw/Documents/src/platform/target/aarch64-apple-ios-sim/release/librs_sdk_ffi.a"
                ])
            ]
        ),
    ]
)