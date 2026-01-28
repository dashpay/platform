import SwiftDashSDK
import XCTest

@testable import SwiftExampleApp

// MARK: - Wallet Storage Tests
// NOTE: WalletStorage initializer is internal in SwiftDashSDK, so these tests cannot
// instantiate it. Re-enable or move tests into the SDK target when a test double
// or public initializer is available.

final class WalletStorageTests: XCTestCase {

  func testWalletStorageTestsDisabled() throws {
    throw XCTSkip(
      "WalletStorage initializer is internal in SwiftDashSDK; cannot run storage tests from app test target."
    )
  }
}
