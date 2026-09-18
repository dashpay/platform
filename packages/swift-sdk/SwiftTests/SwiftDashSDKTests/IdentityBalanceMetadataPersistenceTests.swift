import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

final class IdentityBalanceMetadataPersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let identityId = Data(repeating: 7, count: 32)

    private func persist(_ handler: PlatformWalletPersistenceHandler, balance: UInt64,
                         stamp: BlockTime?, success: Bool = true) throws {
        handler.beginChangeset(walletId: walletId)
        handler.persistIdentities(walletId: walletId, upserts: [.init(
            identityId: identityId, balance: balance, revision: 1, identityIndex: 0,
            label: nil, status: 2, walletId: walletId, dpnsNames: [],
            dashpayProfile: nil, contactProfiles: [])], removed: [])
        try handler.persistIdentityBalanceBlockTime(walletId: walletId, identityId: identityId, blockTime: stamp)
        XCTAssertEqual(handler.endChangeset(walletId: walletId, success: success), success)
    }

    private func seedWallet(_ container: ModelContainer) throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
    }

    func testBalanceAndWatermarkCommitAndRollbackTogether() throws {
        let container = try DashModelContainer.createInMemory()
        try seedWallet(container)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try persist(handler, balance: 100, stamp: BlockTime(height: 20, core_height: 10, timestamp: 999))
        try persist(handler, balance: 50, stamp: BlockTime(height: 21, core_height: 11, timestamp: 1000), success: false)
        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentIdentity>())
        XCTAssertEqual(rows.first?.balance, 100)
        let stamp = try XCTUnwrap(handler.loadIdentityBalanceBlockTime(walletId: walletId, identityId: identityId))
        XCTAssertEqual(stamp.height, 20)
        XCTAssertEqual(stamp.timestamp, 999)
    }

    func testMetadataSurvivesClosingAndReopeningStore() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        try writeStore(url)
        let reopened = try DashModelContainer.create(url: url)
        let handler = PlatformWalletPersistenceHandler(modelContainer: reopened, network: .testnet)
        let stamp = try XCTUnwrap(handler.loadIdentityBalanceBlockTime(walletId: walletId, identityId: identityId))
        XCTAssertEqual(stamp.height, UInt64.max)
        XCTAssertEqual(stamp.core_height, UInt32.max)
        XCTAssertEqual(stamp.timestamp, UInt64.max)
        XCTAssertEqual(try ModelContext(reopened).fetch(FetchDescriptor<PersistentIdentity>()).first?.balance, 123)
    }

    private func writeStore(_ url: URL) throws {
        let container = try DashModelContainer.create(url: url)
        try seedWallet(container)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try persist(handler, balance: 123, stamp: BlockTime(height: .max, core_height: .max, timestamp: .max))
    }

    func testAbsentAndZeroMetadataAreDistinctAndScopedByNetworkAndWallet() throws {
        let container = try DashModelContainer.createInMemory()
        try seedWallet(container)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertNil(try handler.loadIdentityBalanceBlockTime(walletId: walletId, identityId: identityId))
        try persist(handler, balance: 0, stamp: BlockTime(height: 0, core_height: 0, timestamp: 0))
        XCTAssertNotNil(try handler.loadIdentityBalanceBlockTime(walletId: walletId, identityId: identityId))
        XCTAssertNil(try handler.loadIdentityBalanceBlockTime(walletId: Data(repeating: 43, count: 32), identityId: identityId))
        let otherNetwork = PlatformWalletPersistenceHandler(modelContainer: container, network: .mainnet)
        XCTAssertNil(try otherNetwork.loadIdentityBalanceBlockTime(walletId: walletId, identityId: identityId))
        try persist(handler, balance: 0, stamp: nil)
        XCTAssertNil(try handler.loadIdentityBalanceBlockTime(walletId: walletId, identityId: identityId))
    }

    func testWalletDeletionRemovesMetadata() throws {
        let container = try DashModelContainer.createInMemory()
        try seedWallet(container)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try persist(handler, balance: 1, stamp: BlockTime(height: 1, core_height: 1, timestamp: 1))
        try handler.deleteWalletData(walletId: walletId)
        XCTAssertNil(try handler.loadIdentityBalanceBlockTime(walletId: walletId, identityId: identityId))
    }

    func testExtensionCallbacksRoundTripAndClearMetadata() throws {
        let container = try DashModelContainer.createInMemory()
        try seedWallet(container)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let callbacks = handler.makePersistenceCallbacksExtension()
        let persist = try XCTUnwrap(callbacks.on_persist_identity_balance_block_time_fn)
        let load = try XCTUnwrap(callbacks.on_load_identity_balance_block_time_fn)
        let context = Unmanaged.passUnretained(handler).toOpaque()
        walletId.withUnsafeBytes { wallet in
            identityId.withUnsafeBytes { identity in
                let walletBytes = wallet.bindMemory(to: UInt8.self).baseAddress!
                let identityBytes = identity.bindMemory(to: UInt8.self).baseAddress!
                var stamp = BlockTime(height: 42, core_height: 7, timestamp: 123)
                handler.beginChangeset(walletId: walletId)
                XCTAssertEqual(persist(context, walletBytes, identityBytes, &stamp), 0)
                XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
                var found = false
                var loaded = BlockTime()
                XCTAssertEqual(load(context, walletBytes, identityBytes, &found, &loaded), 0)
                XCTAssertTrue(found)
                XCTAssertEqual(loaded.height, 42)
                XCTAssertEqual(loaded.core_height, 7)
                XCTAssertEqual(loaded.timestamp, 123)
                handler.beginChangeset(walletId: walletId)
                XCTAssertEqual(persist(context, walletBytes, identityBytes, nil), 0)
                XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
                XCTAssertEqual(load(context, walletBytes, identityBytes, &found, &loaded), 0)
                XCTAssertFalse(found)
                XCTAssertEqual(load(nil, walletBytes, identityBytes, &found, &loaded), -1)
            }
        }
    }

    func testUnavailableBalanceHasDistinctSwiftError() throws {
        XCTAssertEqual(PlatformWalletResultCode(ffi: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_IDENTITY_BALANCE_UNAVAILABLE).rawValue, 58)
    }
}
