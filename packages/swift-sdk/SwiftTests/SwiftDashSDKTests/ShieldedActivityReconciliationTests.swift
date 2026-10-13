import Foundation
import SwiftData
import XCTest
@testable import SwiftDashSDK

@MainActor
final class ShieldedActivityReconciliationTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let otherWalletId = Data(repeating: 43, count: 32)
    private let outPointHex = String(repeating: "31", count: 32) + ":0"

    private func activity(_ id: UInt8, wallet: Data? = nil, account: UInt32 = 0,
                          notes: [UInt8] = [1], amount: UInt64 = 100) -> PlatformWalletPersistenceHandler.ShieldedActivitySnapshot {
        .init(walletId: wallet ?? walletId, accountIndex: account,
              entryId: Data(repeating: id, count: 32), kindTag: 6, direction: 0, status: 1,
              amount: amount, fee: 0, hasFee: false, blockHeight: 1, hasBlockHeight: true,
              createdAtMs: 0, minNotePosition: 0, hasMinNotePosition: true,
              identityId: Data(), counterparty: Data(), memo: Data(),
              noteCmxs: notes.reduce(into: Data()) { $0.append(Data(repeating: $1, count: 32)) },
              spentNullifiers: Data())
    }

    private func lock(_ status: Int) -> PlatformWalletPersistenceHandler.AssetLockEntrySnapshot {
        .init(outPointHex: outPointHex, transactionBytes: Data([3, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
              fundingTypeRaw: 5, identityIndexRaw: 0, accountIndexRaw: 0,
              amountDuffs: 1000, statusRaw: status, proofBytes: nil,
              operationId: Data(repeating: 1, count: 16))
    }

    private func seed(_ handler: PlatformWalletPersistenceHandler) {
        handler.beginChangeset(walletId: walletId)
        XCTAssertTrue(handler.persistShieldedActivity(walletId: walletId,
            snapshots: [activity(9, notes: [1, 2], amount: 300), activity(9, account: 1)]))
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [lock(0)], removed: []))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        XCTAssertTrue(handler.persistShieldedActivity(walletId: otherWalletId,
            snapshots: [activity(9, wallet: otherWalletId)]))
    }

    private func replace(_ handler: PlatformWalletPersistenceHandler) throws {
        handler.beginChangeset(walletId: walletId)
        let remove = try XCTUnwrap(handler.makePersistenceCallbacksExtension().on_remove_shielded_activity_fn)
        let removedIds = Data(repeating: 9, count: 32)
        let result = walletId.withUnsafeBytes { walletBytes in
            removedIds.withUnsafeBytes { idBytes in
                remove(Unmanaged.passUnretained(handler).toOpaque(),
                       walletBytes.bindMemory(to: UInt8.self).baseAddress, 0,
                       idBytes.bindMemory(to: UInt8.self).baseAddress, 1)
            }
        }
        XCTAssertEqual(result, 0)
        XCTAssertTrue(handler.persistShieldedActivity(walletId: walletId,
            snapshots: [activity(1), activity(2, notes: [2], amount: 200)]))
        XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [lock(4)], removed: []))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    func testReplacementAndCompletionSurviveReopenWithoutTouchingOtherAccountsOrWallets() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container)
            seed(handler)
            try replace(handler)
            try replace(handler) // An uncertain completion can replay idempotently.
        }
        let container = try DashModelContainer.create(url: url)
        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentShieldedActivity>())
        XCTAssertEqual(rows.count, 4)
        let own = rows.filter { $0.walletId == walletId && $0.accountIndex == 0 }
        XCTAssertEqual(Set(own.map(\.entryId)), Set([Data(repeating: 1, count: 32), Data(repeating: 2, count: 32)]))
        XCTAssertEqual(own.reduce(0) { $0 + $1.amount }, 300)
        XCTAssertEqual(rows.filter { $0.accountIndex == 1 }.count, 1)
        XCTAssertEqual(rows.filter { $0.walletId == otherWalletId }.count, 1)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container)
        XCTAssertEqual(try handler.loadCachedAssetLocks(walletId: walletId).first?.statusRaw, 4)
        let restored = handler.loadShieldedActivity()
        defer { handler.loadShieldedActivityFree(entries: restored.entries.map(UnsafeRawPointer.init)) }
        XCTAssertFalse(restored.errored)
        XCTAssertEqual(restored.count, 4)
    }

    func testActivityUpsertFailureRollsBackCompletionAndAllowsRetryAfterReopen() throws {
        try assertHistoryFailureRollsBack(removal: false)
    }

    func testActivityRemovalFailureRollsBackCompletionAndAllowsRetryAfterReopen() throws {
        try assertHistoryFailureRollsBack(removal: true)
    }

    private func assertHistoryFailureRollsBack(removal: Bool) throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            seed(PlatformWalletPersistenceHandler(modelContainer: container))
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: nil,
                modelFetcher: FetchFaultInjector(faulting: PersistentShieldedActivity.self))
            handler.beginChangeset(walletId: walletId)
            XCTAssertTrue(handler.persistAssetLocks(walletId: walletId, upserts: [lock(4)], removed: []))
            if removal {
                XCTAssertThrowsError(try handler.removeShieldedActivity(walletId: walletId, accountIndex: 0,
                    entryIds: [Data(repeating: 9, count: 32)]))
            } else {
                XCTAssertFalse(handler.persistShieldedActivity(walletId: walletId, snapshots: [activity(1)]))
            }
            XCTAssertFalse(handler.endChangeset(walletId: walletId, success: false))
        }
        let reopened = try DashModelContainer.create(url: url)
        let handler = PlatformWalletPersistenceHandler(modelContainer: reopened)
        XCTAssertEqual(try handler.loadCachedAssetLocks(walletId: walletId).first?.statusRaw, 0)
        let rows = try ModelContext(reopened).fetch(FetchDescriptor<PersistentShieldedActivity>())
        XCTAssertEqual(rows.count, 3)
        XCTAssertEqual(rows.first { $0.walletId == walletId && $0.accountIndex == 0 }?.amount, 300)
        try replace(handler)
        XCTAssertEqual(try handler.loadCachedAssetLocks(walletId: walletId).first?.statusRaw, 4)
    }

    func testActivityRestoreFailsOnUnreadableHistoryAndWalletScope() throws {
        let container = try DashModelContainer.createInMemory()
        for model in [PersistentShieldedActivity.self, PersistentWallet.self] as [any PersistentModel.Type] {
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet,
                modelFetcher: FetchFaultInjector(faulting: model))
            let restored = handler.loadShieldedActivity()
            XCTAssertTrue(restored.errored)
            XCTAssertNil(restored.entries)
            XCTAssertEqual(restored.count, 0)
        }
    }

    func testActivityRestoreRejectsCorruptLinkageInsteadOfReturningPartialHistory() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container)
        seed(handler)
        let context = ModelContext(container)
        let rows = try context.fetch(FetchDescriptor<PersistentShieldedActivity>())
        rows[0].noteCmxs = Data([1])
        try context.save()
        let reader = PlatformWalletPersistenceHandler(modelContainer: container)
        let restored = reader.loadShieldedActivity()
        XCTAssertTrue(restored.errored)
        XCTAssertNil(restored.entries)
        XCTAssertEqual(restored.count, 0)
    }
}
