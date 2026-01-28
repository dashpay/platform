import SwiftDashSDK
import SwiftData
import XCTest

@testable import SwiftExampleApp

// MARK: - Wallet Integration Tests
// NOTE: Full integration tests require WalletManager(modelContainer:) and WalletViewModel
// which are not available in the current SDK. Use SwiftExampleApp with a running SPV
// for manual integration testing. Re-enable tests when those types/APIs are added.

@MainActor
final class WalletIntegrationTests: XCTestCase {

  func testWalletIntegrationDisabled() throws {
    throw XCTSkip(
      "Wallet integration tests require WalletManager(modelContainer:) and WalletViewModel which are not available in the current SDK. Use SwiftExampleApp with a running SPV for manual integration testing."
    )
  }
}
