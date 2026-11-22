import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins which networks' saved contracts a new SDK preloads into its trusted
/// provider: only those of a network whose chain is fixed. The provider
/// caches by contract id, so a saved contract from another chain under a
/// shared id would shadow the chain's own revision.
final class SavedContractPreloadTests: XCTestCase {

    /// Mainnet and testnet are each one chain: a row saved for them belongs
    /// to the chain the SDK talks to, so it is preloaded.
    func testShouldPreloadSavedContractsForMainnetAndTestnet() {
        XCTAssertTrue(AppState.preloadsSavedContracts(for: .mainnet))
        XCTAssertTrue(AppState.preloadsSavedContracts(for: .testnet))
    }

    /// Two devnet configurations (say the user changes the quorum URL) are
    /// both `.devnet`, so the network value cannot tell a row saved on the
    /// old devnet from one on the new; the preload is skipped for every
    /// devnet, and likewise for regtest, whose chain is a local setup.
    func testShouldSkipThePreloadForEveryDevnetAndRegtest() {
        XCTAssertFalse(AppState.preloadsSavedContracts(for: .devnet))
        XCTAssertFalse(AppState.preloadsSavedContracts(for: .regtest))
    }
}
