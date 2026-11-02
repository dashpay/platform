import Foundation

/// A directory of rs-sdk's recorded offline vectors
/// (packages/rs-sdk/tests/vectors/<name>).
func rsSdkVectors(_ name: String) -> String {
    URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()  // SwiftDashSDKTests
        .deletingLastPathComponent()  // SwiftTests
        .deletingLastPathComponent()  // swift-sdk
        .deletingLastPathComponent()  // packages
        .appendingPathComponent("rs-sdk/tests/vectors/\(name)", isDirectory: true)
        .path
}
