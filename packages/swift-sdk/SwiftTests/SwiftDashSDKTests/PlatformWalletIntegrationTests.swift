import XCTest
@testable import SwiftDashSDK

/// Integration tests for Platform Wallet with real identity data and contact flows
/// These tests require the full FFI stack to be built and linked
class PlatformWalletIntegrationTests: XCTestCase {

    var wallet: PlatformWallet!
    var identityManager: IdentityManager!

    override func setUpWithError() throws {
        try super.setUpWithError()
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        wallet = try PlatformWallet.fromMnemonic(mnemonic)
        identityManager = try wallet.getIdentityManager(for: .testnet)
    }

    // MARK: - Wallet and Identity Manager Integration

    func testWalletToIdentityManagerFlow() throws {
        // Verify we can create wallet and get identity manager
        XCTAssertNotNil(wallet)
        XCTAssertNotNil(identityManager)

        // Verify initial state
        let count = try identityManager.getIdentityCount()
        XCTAssertGreaterThanOrEqual(count, 0, "Should have zero or more identities")

        let ids = try identityManager.getAllIdentityIds()
        XCTAssertEqual(ids.count, count, "ID count should match identity count")
    }

    func testMultipleNetworkIdentityManagers() throws {
        let mainnetManager = try wallet.getIdentityManager(for: .mainnet)
        let testnetManager = try wallet.getIdentityManager(for: .testnet)
        let devnetManager = try wallet.getIdentityManager(for: .devnet)

        XCTAssertNotEqual(mainnetManager.handle, testnetManager.handle)
        XCTAssertNotEqual(testnetManager.handle, devnetManager.handle)
        XCTAssertNotEqual(mainnetManager.handle, devnetManager.handle)
    }

    // MARK: - Error Handling Integration

    func testWalletCreationErrorHandling() {
        // Test invalid mnemonic
        XCTAssertThrowsError(try PlatformWallet.fromMnemonic("invalid mnemonic phrase")) { error in
            XCTAssertTrue(error is PlatformWalletError)
        }

        // Test invalid seed size
        let invalidSeed = Data(count: 10)
        XCTAssertThrowsError(try PlatformWallet.fromSeed(invalidSeed)) { error in
            if case PlatformWalletError.invalidParameter = error {
                // Expected
            } else {
                XCTFail("Expected invalidParameter error, got \(error)")
            }
        }
    }

}
