import DashSDKFFI
import Foundation
import SwiftData
import XCTest
@testable import SwiftDashSDK

/// The identity-recipient sidecar of a tracked asset lock: the identity an
/// unbound identity top-up (funding type 2) was built or assigned to pay.
/// Rust stores it with the lock's row and reloads it on restore, so it must
/// survive store reopen, never change once recorded, and fail a load whose
/// row is missing or malformed rather than report "no recipient".
@MainActor
final class AssetLockIdentityRecipientPersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let recipient = Data((0..<32).map { UInt8($0 &+ 1) })
    private let otherRecipient = Data(repeating: 9, count: 32)
    private let outPointHex = String(repeating: "31", count: 32) + ":0"
    private var rawOutPoint: Data { Data(repeating: 0x31, count: 32) + Data(repeating: 0, count: 4) }

    private func snapshot(status: Int) -> PlatformWalletPersistenceHandler.AssetLockEntrySnapshot {
        .init(outPointHex: outPointHex, transactionBytes: Data([3, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
              fundingTypeRaw: 2, identityIndexRaw: 0, accountIndexRaw: 0,
              amountDuffs: 50_500, statusRaw: status, proofBytes: nil)
    }

    /// One store round: the row's upsert, then (optionally) its recipient,
    /// as Rust delivers them.
    private func round(
        _ handler: PlatformWalletPersistenceHandler, status: Int, recipient: Data? = nil
    ) throws {
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [snapshot(status: status)], removed: []))
        if let recipient {
            try handler.persistAssetLockIdentityRecipient(
                walletId: walletId, outPoint: rawOutPoint, identityId: recipient)
        }
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    private func withStoreURL(_ body: (URL) throws -> Void) throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        try body(directory.appendingPathComponent("wallet.store"))
    }

    func testShouldKeepTheRecipientThroughLaterSnapshotsAndAStoreReopen() throws {
        try withStoreURL { url in
            try autoreleasepool {
                let handler = PlatformWalletPersistenceHandler(
                    modelContainer: try DashModelContainer.create(url: url), network: .testnet)
                try round(handler, status: 0, recipient: recipient)
                // Later snapshots carry no recipient (a status advance, a
                // reconstruction); they must not erase it.
                try round(handler, status: 1)
                try round(handler, status: 3)
            }
            let handler = PlatformWalletPersistenceHandler(
                modelContainer: try DashModelContainer.create(url: url), network: .testnet)
            let rows = try handler.loadCachedAssetLocks(walletId: walletId)
            XCTAssertEqual(rows.count, 1)
            XCTAssertEqual(rows.first?.identityRecipient, recipient)
            XCTAssertEqual(rows.first?.statusRaw, 3)
            XCTAssertEqual(
                try handler.loadAssetLockIdentityRecipient(walletId: walletId, outPoint: rawOutPoint), recipient)
        }
    }

    func testShouldAcceptTheSameRecipientAgainAndRefuseADifferentOne() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try round(handler, status: 1, recipient: recipient)
        // An assignment retried with the same identity stores again.
        try round(handler, status: 1, recipient: recipient)

        handler.beginChangeset(walletId: walletId)
        XCTAssertThrowsError(try handler.persistAssetLockIdentityRecipient(
            walletId: walletId, outPoint: rawOutPoint, identityId: otherRecipient))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
        XCTAssertEqual(
            try handler.loadAssetLockIdentityRecipient(walletId: walletId, outPoint: rawOutPoint), recipient)
    }

    /// A snapshot row that names a recipient cannot replace a stored one:
    /// the stored value wins on every upsert.
    func testShouldNotLetAnUpsertReplaceARecordedRecipient() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try round(handler, status: 1, recipient: recipient)
        var conflicting = snapshot(status: 2)
        conflicting.identityRecipient = otherRecipient
        handler.beginChangeset(walletId: walletId)
        XCTAssertFalse(handler.persistAssetLocks(walletId: walletId, upserts: [conflicting], removed: []))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
        XCTAssertEqual(
            try handler.loadAssetLockIdentityRecipient(walletId: walletId, outPoint: rawOutPoint), recipient)
    }

    /// A consumed row is terminal: a stale recipient naming another identity
    /// is dropped for that row instead of failing the round, as the SQLite
    /// store and the in-memory apply do.
    func testShouldSkipAConflictingRecipientOnAConsumedRow() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try round(handler, status: 1, recipient: recipient)
        try round(handler, status: 4)
        try round(handler, status: 4, recipient: otherRecipient)
        XCTAssertEqual(
            try handler.loadAssetLockIdentityRecipient(walletId: walletId, outPoint: rawOutPoint), recipient)
        XCTAssertEqual(try handler.loadCachedAssetLocks(walletId: walletId).first?.statusRaw, 4)
    }

    /// The consumed exception counts only a row consumed before the round. A
    /// round whose upsert consumes a live row and whose sidecar names another
    /// identity is refused as a conflict, as SQLite judges the row before the
    /// update: it must not commit the consumed status with the old recipient.
    func testShouldRefuseAConflictingRecipientInTheRoundThatConsumesALiveRow() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try round(handler, status: 1, recipient: recipient)

        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [snapshot(status: 4)], removed: []))
        XCTAssertThrowsError(try handler.persistAssetLockIdentityRecipient(
            walletId: walletId, outPoint: rawOutPoint, identityId: otherRecipient))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))

        let row = try handler.loadCachedAssetLocks(walletId: walletId).first
        XCTAssertEqual(row?.statusRaw, 1, "the live row is not consumed")
        XCTAssertEqual(
            try handler.loadAssetLockIdentityRecipient(walletId: walletId, outPoint: rawOutPoint), recipient)
    }

    func testShouldFailClosedWithoutARowAChangesetOrAWellFormedValue() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        // No row: a read is a failure, not a lock without a recipient.
        XCTAssertThrowsError(try handler.loadAssetLockIdentityRecipient(walletId: walletId, outPoint: rawOutPoint))
        // Outside a store round nothing is written.
        XCTAssertThrowsError(try handler.persistAssetLockIdentityRecipient(
            walletId: walletId, outPoint: rawOutPoint, identityId: recipient))
        handler.beginChangeset(walletId: walletId)
        // No row to attach it to.
        XCTAssertThrowsError(try handler.persistAssetLockIdentityRecipient(
            walletId: walletId, outPoint: rawOutPoint, identityId: recipient))
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [snapshot(status: 0)], removed: []))
        XCTAssertThrowsError(try handler.persistAssetLockIdentityRecipient(
            walletId: walletId, outPoint: rawOutPoint, identityId: Data(repeating: 1, count: 31)))
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
    }

    func testShouldFailTheLoadOfAMalformedStoredRecipient() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentAssetLock(
            outPointHex: outPointHex, walletId: walletId, transactionBytes: Data([1]),
            fundingTypeRaw: 2, identityIndexRaw: 0, amountDuffs: 50_500, statusRaw: 1,
            identityRecipient: Data(repeating: 1, count: 31)))
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertThrowsError(try handler.loadAssetLockIdentityRecipient(walletId: walletId, outPoint: rawOutPoint))
        XCTAssertThrowsError(try handler.loadCachedAssetLocks(walletId: walletId))
    }

    /// The C shims Rust calls: wired in the extension, writing through to the
    /// row and reading it back; a lock without a recipient reports not found.
    func testShouldRoundTripTheRecipientThroughTheExtensionCallbacks() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let callbacks = handler.makePersistenceCallbacksExtension()
        let persist = try XCTUnwrap(callbacks.on_persist_asset_lock_identity_recipient_fn)
        let load = try XCTUnwrap(callbacks.on_load_asset_lock_identity_recipient_fn)
        let context = Unmanaged.passUnretained(handler).toOpaque()
        try round(handler, status: 0)

        try walletId.withUnsafeBytes { wallet in
            try rawOutPoint.withUnsafeBytes { outPoint in
                let walletBytes = try XCTUnwrap(wallet.bindMemory(to: UInt8.self).baseAddress)
                let outPointBytes = try XCTUnwrap(outPoint.bindMemory(to: UInt8.self).baseAddress)
                var found = true
                var loaded = [UInt8](repeating: 0xEE, count: 32)
                XCTAssertEqual(load(context, walletBytes, outPointBytes, &found, &loaded), 0)
                XCTAssertFalse(found)

                handler.beginChangeset(walletId: walletId)
                let stored = try recipient.withUnsafeBytes { identity in
                    persist(context, walletBytes, outPointBytes,
                            try XCTUnwrap(identity.bindMemory(to: UInt8.self).baseAddress))
                }
                XCTAssertEqual(stored, 0)
                XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

                XCTAssertEqual(load(context, walletBytes, outPointBytes, &found, &loaded), 0)
                XCTAssertTrue(found)
                XCTAssertEqual(Data(loaded), recipient)

                handler.beginChangeset(walletId: walletId)
                let refused = try otherRecipient.withUnsafeBytes { identity in
                    persist(context, walletBytes, outPointBytes,
                            try XCTUnwrap(identity.bindMemory(to: UInt8.self).baseAddress))
                }
                XCTAssertEqual(refused, -1)
                XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
                XCTAssertEqual(load(nil, walletBytes, outPointBytes, &found, &loaded), -1)
            }
        }
    }

    func testShouldDeclareIdentityRecipientsAsBit17() {
        XCTAssertEqual(PlatformWalletPersistenceCapabilities.assetLockIdentityRecipients, 1 << 17)
        XCTAssertEqual(
            PlatformWalletPersistenceCapabilities.assetLockIdentityRecipients,
            UInt64(PLATFORM_WALLET_PERSISTENCE_CAPABILITY_ASSET_LOCK_IDENTITY_RECIPIENTS))
    }
}
