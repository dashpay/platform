import XCTest
@testable import SwiftDashSDK

@MainActor
final class ShieldedAccountSnapshotTests: XCTestCase {
    /// Identity discovery can precede configureShielded. Reading the current
    /// account set must still succeed so the service can perform the first bind.
    func testUnconfiguredShieldedWalletReturnsEmptySnapshot() async throws {
        let sdk = try SDK(network: .testnet)
        let manager = try PlatformWalletManager(sdk: sdk)
        do {
            let wallet = try await manager.createWallet(
                mnemonic: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
                network: .testnet,
                createDefaultAccounts: false)
            XCTAssertEqual(try manager.shieldedAccountIndices(walletId: wallet.walletId), [])
            XCTAssertThrowsError(try manager.shieldedAccountIndices(walletId: Data(repeating: 0x71, count: 32)))
            await manager.shutdown()
        } catch {
            await manager.shutdown()
            throw error
        }
    }
}
