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
    func testClearLocalStateZerosActiveNetworkBalancesInPlaceAndScopesToNetwork() async throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let testnetWalletId = Data(repeating: 0x44, count: 32)
        let mainnetWalletId = Data(repeating: 0x55, count: 32)

        // Active-network (testnet) row — must be ZEROED IN PLACE (kept, so
        // its durable derivation metadata survives for the rescan to
        // re-persist balances against).
        context.insert(
            PersistentPlatformAddress(
                address: "yTestnetPlatformAddr",
                addressType: 0,
                addressHash: Data(repeating: 0x01, count: 20),
                publicKey: Data(repeating: 0xab, count: 33),
                accountIndex: 3,
                addressIndex: 7,
                derivationPath: "m/9'/1'/17'/3'/0'/7",
                isUsed: true,
                balance: 294_627_247_940,
                nonce: 5,
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

        // Other-network (mainnet) rows — must be untouched, since the
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
                isUsed: true,
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

        let service = PlatformBalanceSyncService()
        await service.clearLocalState(
            modelContext: context,
            network: .testnet,
            walletIdsOnNetwork: [testnetWalletId]
        )

        // No address rows are deleted — both networks' rows still exist.
        let addresses = try fetch(PersistentPlatformAddress.self, in: container)
        XCTAssertEqual(addresses.count, 2, "address rows must be preserved, not deleted")

        // Testnet row: volatile fields zeroed, durable metadata preserved.
        let testnetAddr = try XCTUnwrap(addresses.first { $0.walletId == testnetWalletId })
        XCTAssertEqual(testnetAddr.balance, 0, "balance zeroed")
        XCTAssertEqual(testnetAddr.nonce, 0, "nonce zeroed")
        XCTAssertFalse(testnetAddr.isUsed, "isUsed zeroed")
        XCTAssertEqual(testnetAddr.address, "yTestnetPlatformAddr", "durable address preserved")
        XCTAssertEqual(testnetAddr.publicKey, Data(repeating: 0xab, count: 33), "durable public key preserved")
        XCTAssertEqual(testnetAddr.derivationPath, "m/9'/1'/17'/3'/0'/7", "durable derivation path preserved")
        XCTAssertEqual(testnetAddr.accountIndex, 3, "durable account index preserved")
        XCTAssertEqual(testnetAddr.addressIndex, 7, "durable address index preserved")

        // Mainnet row: fully untouched (other network not in scope).
        let mainnetAddr = try XCTUnwrap(addresses.first { $0.walletId == mainnetWalletId })
        XCTAssertEqual(mainnetAddr.balance, 111_111, "other network's balance must be untouched")
        XCTAssertTrue(mainnetAddr.isUsed)

        // Watermark: testnet deleted (forces full rescan), mainnet preserved.
        let states = try fetch(PersistentPlatformAddressesSyncState.self, in: container)
        XCTAssertEqual(
            states.map(\.networkRaw), [Network.mainnet.rawValue],
            "only testnet's sync-state watermark is deleted; mainnet survives"
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
