import XCTest

@testable import SwiftExampleApp

// MARK: - Key Derivation Tests
// NOTE: Tests require CoreSDKWrapper, DerivationPath, HDKeyDerivation, WalletFFIBridge
// which are not exposed to the app test target. Re-enable when those types are
// available from SwiftDashSDK or a test helper.

final class KeyDerivationTests: XCTestCase {

  func testKeyDerivationTestsDisabled() throws {
    throw XCTSkip(
      "Key derivation tests require CoreSDKWrapper, DerivationPath, HDKeyDerivation which are not in scope for app tests."
    )
  }
}
