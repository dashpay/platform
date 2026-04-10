import XCTest
@testable import SwiftDashSDK

class PlatformWalletTests: XCTestCase {

    var testSeed: Data!
    var testMnemonic: String!

    override func setUp() {
        super.setUp()

        // Create a 64-byte test seed
        testSeed = Data(count: 64)

        // Use a valid BIP39 mnemonic (12 words)
        testMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    }

    // MARK: - Wallet Creation Tests

    func testCreateWalletFromSeed() throws {
        let wallet = try PlatformWallet.fromSeed(testSeed)
        XCTAssertNotNil(wallet, "Wallet should be created from seed")
    }

    func testCreateWalletFromInvalidSeed() {
        let invalidSeed = Data(count: 32) // Wrong size

        XCTAssertThrowsError(try PlatformWallet.fromSeed(invalidSeed)) { error in
            XCTAssertTrue(error is PlatformWalletError)
            if case PlatformWalletError.invalidParameter = error {
                // Expected error
            } else {
                XCTFail("Expected invalidParameter error, got \(error)")
            }
        }
    }

    func testCreateWalletFromMnemonic() throws {
        let wallet = try PlatformWallet.fromMnemonic(testMnemonic)
        XCTAssertNotNil(wallet, "Wallet should be created from mnemonic")
    }

    // MARK: - Identity Manager Tests

    func testGetIdentityManager() throws {
        let wallet = try PlatformWallet.fromSeed(testSeed)
        let manager = try wallet.getIdentityManager(for: .testnet)
        XCTAssertNotNil(manager, "Should get identity manager for testnet")
    }

    func testGetIdentityManagerCaching() throws {
        let wallet = try PlatformWallet.fromSeed(testSeed)
        let manager1 = try wallet.getIdentityManager(for: .testnet)
        let manager2 = try wallet.getIdentityManager(for: .testnet)

        // Should return the same cached instance
        XCTAssertEqual(manager1.handle, manager2.handle, "Should return cached manager")
    }

    func testSetIdentityManager() throws {
        let wallet = try PlatformWallet.fromSeed(testSeed)
        let newManager = try IdentityManager.create()

        try wallet.setIdentityManager(newManager, for: .mainnet)
        let retrievedManager = try wallet.getIdentityManager(for: .mainnet)

        XCTAssertEqual(newManager.handle, retrievedManager.handle, "Should retrieve set manager")
    }

    func testMultipleNetworkManagers() throws {
        let wallet = try PlatformWallet.fromSeed(testSeed)

        let mainnetManager = try wallet.getIdentityManager(for: .mainnet)
        let testnetManager = try wallet.getIdentityManager(for: .testnet)
        let devnetManager = try wallet.getIdentityManager(for: .devnet)

        XCTAssertNotEqual(mainnetManager.handle, testnetManager.handle, "Different networks should have different managers")
        XCTAssertNotEqual(testnetManager.handle, devnetManager.handle, "Different networks should have different managers")
    }
}
