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
    func testClearLocalStateWipesActiveNetworkRowsAndPreservesOthers() async throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let testnetWalletId = Data(repeating: 0x44, count: 32)
        let mainnetWalletId = Data(repeating: 0x55, count: 32)

        // Active-network (testnet) rows — these must be deleted.
        context.insert(
            PersistentPlatformAddress(
                address: "yTestnetPlatformAddr",
                addressType: 0,
                addressHash: Data(repeating: 0x01, count: 20),
                accountIndex: 0,
                addressIndex: 0,
                derivationPath: "m/9'/1'/17'/0'/0'/0",
                balance: 294_627_247_940,
                walletId: testnetWalletId
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

        // Other-network (mainnet) rows — these must SURVIVE, since the
        // SwiftData store holds every network's rows at once and Clear is
        // scoped to the active network only.
        context.insert(
            PersistentPlatformAddress(
                address: "XMainnetPlatformAddr",
                addressType: 0,
                addressHash: Data(repeating: 0x02, count: 20),
                accountIndex: 0,
                addressIndex: 0,
                derivationPath: "m/9'/5'/17'/0'/0'/0",
                balance: 111_111,
                walletId: mainnetWalletId
            )
        )
        context.insert(
            PersistentPlatformAddressesSyncState(
                walletId: Self.syncStateScopeId(for: .mainnet),
                network: .mainnet,
                syncHeight: 99,
                syncTimestamp: 88,
                lastKnownRecentBlock: 77
            )
        )
        try context.save()

        // Sanity: both networks' rows present before the clear.
        XCTAssertEqual(try fetch(PersistentPlatformAddress.self, in: container).count, 2)
        XCTAssertEqual(try fetch(PersistentPlatformAddressesSyncState.self, in: container).count, 2)

        let service = PlatformBalanceSyncService()
        await service.clearLocalState(
            modelContext: context,
            network: .testnet,
            walletIdsOnNetwork: [testnetWalletId]
        )

        // Active-network (testnet) rows gone.
        let remainingAddresses = try fetch(PersistentPlatformAddress.self, in: container)
        XCTAssertEqual(
            remainingAddresses.map(\.walletId), [mainnetWalletId],
            "only testnet's cached per-address balances must be deleted"
        )
        let remainingStates = try fetch(PersistentPlatformAddressesSyncState.self, in: container)
        XCTAssertEqual(
            remainingStates.map(\.networkRaw), [Network.mainnet.rawValue],
            "only testnet's sync-state watermark must be deleted; mainnet must survive"
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
