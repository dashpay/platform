import SwiftData
import XCTest
import DashSDKFFI

@testable import SwiftDashSDK

/// The host half of native shielded Clear: the wipe callback Rust calls
/// after its own reset.
final class ShieldedStateWipeTests: XCTestCase {
    private let testnetWallet = Data(repeating: 0x61, count: 32)
    private let mainnetWallet = Data(repeating: 0x62, count: 32)

    private func seed(_ container: ModelContainer) throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: testnetWallet, network: .testnet))
        context.insert(PersistentWallet(walletId: mainnetWallet, network: .mainnet))
        for walletId in [testnetWallet, mainnetWallet] {
            context.insert(PersistentShieldedSyncState(
                walletId: walletId, accountIndex: 0, lastSyncedIndex: 42))
            context.insert(PersistentShieldedViewingKey(
                walletId: walletId, accountIndex: 0, fvkBytes: Data(repeating: 1, count: 96)))
        }
        try context.save()
    }

    private func walletIds<T: PersistentModel>(
        _ type: T.Type, _ container: ModelContainer, _ walletId: KeyPath<T, Data>
    ) throws -> [Data] {
        try ModelContext(container).fetch(FetchDescriptor<T>()).map { $0[keyPath: walletId] }
    }

    func testShouldDeclareTheCapabilityAndWireTheCallback() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertEqual(PlatformWalletPersistenceCapabilities.shieldedStateWipe, 1 << 16)
        XCTAssertEqual(
            PlatformWalletPersistenceCapabilities.shieldedStateWipe,
            UInt64(PLATFORM_WALLET_PERSISTENCE_CAPABILITY_SHIELDED_STATE_WIPE))
        XCTAssertNotEqual(
            handler.makePersistenceCapabilities().bits
                & PlatformWalletPersistenceCapabilities.shieldedStateWipe, 0)
        XCTAssertNotNil(handler.makePersistenceCallbacksExtension().on_wipe_shielded_state_fn)
    }

    func testShouldDeleteEveryShieldedRowOfTheNetworkThroughTheShimAndKeepOtherNetworks() throws {
        let container = try DashModelContainer.createInMemory()
        try seed(container)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let wipe = try XCTUnwrap(handler.makePersistenceCallbacksExtension().on_wipe_shielded_state_fn)

        let rc = wipe(Unmanaged.passUnretained(handler).toOpaque(), Network.testnet.ffiValue)

        XCTAssertEqual(rc, 0)
        XCTAssertEqual(
            try walletIds(PersistentShieldedSyncState.self, container, \.walletId), [mainnetWallet])
        XCTAssertEqual(
            try walletIds(PersistentShieldedViewingKey.self, container, \.walletId), [mainnetWallet])
    }
}
