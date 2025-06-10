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
                .unsafeFlags([
                    "-L/Users/samuelw/Documents/src/platform/packages/rs-sdk-ffi/build/simulator",
                    "-lrs_sdk_ffi"
                ])
            ]
        ),
    ]
)