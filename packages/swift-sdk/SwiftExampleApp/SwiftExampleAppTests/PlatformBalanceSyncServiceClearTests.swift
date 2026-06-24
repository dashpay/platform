import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

@MainActor
final class PlatformBalanceSyncServiceClearTests: XCTestCase {

    /// The Platform Sync "Clear" button must delete BOTH platform-address
    /// SwiftData stores — the cached per-address balances
    /// (`PersistentPlatformAddress`) and the network-scoped sync-state
    /// watermark (`PersistentPlatformAddressesSyncState`) — so the UI
    /// reads zero and the next sync is a full rescan rather than a ~2s
    /// incremental resume (the reported "Clear didn't work" symptom).
    ///
    /// The Rust-side watermark reset is skipped here because no wallet
    /// manager is configured (the `if let walletManager` guard short-
    /// circuits); that path is covered by the platform-wallet Rust unit
    /// test (`reset_sync_state_clears_watermark_and_seed`) and manual
    /// simulator verification.
    func testClearLocalStateWipesPlatformAddressRows() async throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let walletId = Data(repeating: 0x44, count: 32)

        context.insert(
            PersistentPlatformAddress(
                address: "yTestPlatformAddr",
                addressType: 0,
                addressHash: Data(repeating: 0x01, count: 20),
                accountIndex: 0,
                addressIndex: 0,
                derivationPath: "m/9'/1'/17'/0'/0'/0",
                balance: 294_627_247_940,
                walletId: walletId
            )
        )
        context.insert(
            PersistentPlatformAddressesSyncState(
                walletId: Self.syncStateScopeId(for: .testnet),
                network: .testnet,
                syncHeight: 10,
                syncTimestamp: 20,
                lastKnownRecentBlock: 30
            )
        )
        try context.save()

        // Sanity: both rows present before the clear.
        XCTAssertEqual(try fetch(PersistentPlatformAddress.self, in: container).count, 1)
        XCTAssertEqual(try fetch(PersistentPlatformAddressesSyncState.self, in: container).count, 1)

        let service = PlatformBalanceSyncService()
        await service.clearLocalState(modelContext: context)

        XCTAssertTrue(
            try fetch(PersistentPlatformAddress.self, in: container).isEmpty,
            "cached per-address balances must be deleted"
        )
        XCTAssertTrue(
            try fetch(PersistentPlatformAddressesSyncState.self, in: container).isEmpty,
            "the sync-state watermark must be deleted so the next sync is a full rescan"
        )
    }

    private static func syncStateScopeId(for network: Network) -> Data {
        var data = Data("platform-sync:\(network.networkName)".utf8.prefix(32))
        if data.count < 32 {
            data.append(Data(repeating: 0, count: 32 - data.count))
        }
        return data
    }

    private func fetch<T: PersistentModel>(_ type: T.Type, in container: ModelContainer) throws -> [T] {
        try ModelContext(container).fetch(FetchDescriptor<T>())
    }
}
