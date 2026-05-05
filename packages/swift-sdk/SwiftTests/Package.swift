// swift-tools-version: 6.0
import PackageDescription

let package = Package(
  name: "SwiftDashSDKTests",
  platforms: [
    .macOS(.v10_15),
    .iOS(.v13),
  ],
  products: [
    .library(
      name: "SwiftDashSDKTests",
      targets: ["SwiftDashSDKTests"])
  ],
  dependencies: [
    .package(path: "..")
  ],
  targets: [
    .target(
      name: "SwiftDashSDKMock",
      dependencies: [],
      path: "Sources/SwiftDashSDKMock",
      publicHeadersPath: "."
    ),
    .testTarget(
      name: "SwiftDashSDKTests",
      dependencies: ["SwiftDashSDKMock", .product(name: "SwiftDashSDK", package: "swift-sdk")],
      path: "Tests/SwiftDashSDKTests",
      exclude: ["*.o", "*.d", "*.swiftdeps"]
    ),
  ],
  swiftLanguageVersions: [.v6]
)
