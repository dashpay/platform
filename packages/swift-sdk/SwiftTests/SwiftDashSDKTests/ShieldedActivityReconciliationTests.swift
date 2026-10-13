import Foundation
import DashSDKFFI
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


    private enum RestoreKind: CaseIterable {
        case notes, outgoing, syncStates, viewingKeys, activity

        var model: any PersistentModel.Type {
            switch self {
            case .notes: return PersistentShieldedNote.self
            case .outgoing: return PersistentShieldedOutgoingNote.self
            case .syncStates: return PersistentShieldedSyncState.self
            case .viewingKeys: return PersistentShieldedViewingKey.self
            case .activity: return PersistentShieldedActivity.self
            }
        }
    }

    /// Exercise the actual callbacks Rust consumes, including their ownership
    /// contract: a failed load must expose neither an allocation nor a prefix.
    private func restore(_ kind: RestoreKind, with handler: PlatformWalletPersistenceHandler)
        -> (code: Int32, count: UInt, hasEntries: Bool) {
        let callbacks = handler.makeCallbacks()
        defer { callbacks.release_fn?(callbacks.context) }
        func load<Entry>(
            _ type: Entry.Type,
            read: (UnsafeMutablePointer<UnsafePointer<Entry>?>, UnsafeMutablePointer<UInt>) -> Int32,
            free: (UnsafePointer<Entry>?, UInt) -> Void
        ) -> (code: Int32, count: UInt, hasEntries: Bool) {
            var entries: UnsafePointer<Entry>?
            var count: UInt = 99 // The callback must clear stale output on failure.
            let code = read(&entries, &count)
            defer { if let entries { free(entries, count) } }
            return (code, count, entries != nil)
        }
        switch kind {
        case .notes:
            return load(ShieldedNoteRestoreFFI.self,
                read: { callbacks.on_load_shielded_notes_fn!(callbacks.context, $0, $1) },
                free: { callbacks.on_load_shielded_notes_free_fn!(callbacks.context, $0, $1) })
        case .outgoing:
            return load(ShieldedOutgoingNoteRestoreFFI.self,
                read: { callbacks.on_load_shielded_outgoing_notes_fn!(callbacks.context, $0, $1) },
                free: { callbacks.on_load_shielded_outgoing_notes_free_fn!(callbacks.context, $0, $1) })
        case .syncStates:
            return load(ShieldedSubwalletSyncStateFFI.self,
                read: { callbacks.on_load_shielded_sync_states_fn!(callbacks.context, $0, $1) },
                free: { callbacks.on_load_shielded_sync_states_free_fn!(callbacks.context, $0, $1) })
        case .viewingKeys:
            return load(ShieldedViewingKeyRestoreFFI.self,
                read: { callbacks.on_load_shielded_viewing_keys_fn!(callbacks.context, $0, $1) },
                free: { callbacks.on_load_shielded_viewing_keys_free_fn!(callbacks.context, $0, $1) })
        case .activity:
            return load(ShieldedActivityRestoreFFI.self,
                read: { callbacks.on_load_shielded_activity_fn!(callbacks.context, $0, $1) },
                free: { callbacks.on_load_shielded_activity_free_fn!(callbacks.context, $0, $1) })
        }
    }

    private func assertRestoreFails(_ kind: RestoreKind, with handler: PlatformWalletPersistenceHandler,
                                    file: StaticString = #filePath, line: UInt = #line) {
        let result = restore(kind, with: handler)
        XCTAssertNotEqual(result.code, 0, "\(kind)", file: file, line: line)
        XCTAssertEqual(result.count, 0, "\(kind)", file: file, line: line)
        XCTAssertFalse(result.hasEntries, "\(kind)", file: file, line: line)
    }

    private func note(_ id: UInt8, wallet: Data? = nil) -> PersistentShieldedNote {
        PersistentShieldedNote(walletId: wallet ?? walletId, accountIndex: 0,
            position: UInt64(id), cmx: Data(repeating: id, count: 32),
            nullifier: Data(repeating: id + 50, count: 32), blockHeight: 1,
            isSpent: false, value: UInt64(id) * 100, noteData: Data(repeating: id, count: 115))
    }

    func testMalformedUnrelatedNoteRejectsCallbackAndPreservesAggregateAfterReopen() throws {
        // A is the drain's output. B is an unrelated receipt in the same scan
        // aggregate. No partial snapshot may let A's completion discard B.
        let fields: [ReferenceWritableKeyPath<PersistentShieldedNote, Data>] = [
            \.walletId, \.cmx, \.nullifier, \.noteData,
        ]
        for field in fields {
            let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            defer { try? FileManager.default.removeItem(at: directory) }
            let url = directory.appendingPathComponent("wallet.store")
            try autoreleasepool {
                let container = try DashModelContainer.create(url: url)
                seed(PlatformWalletPersistenceHandler(modelContainer: container))
                let context = ModelContext(container)
                context.insert(PersistentWallet(walletId: walletId, network: .testnet))
                context.insert(note(1))
                let unrelated = note(2)
                unrelated[keyPath: field] = Data([2])
                context.insert(unrelated)
                try context.save()
                let reader = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
                assertRestoreFails(.notes, with: reader)
            }
            try autoreleasepool {
                let reopened = try DashModelContainer.create(url: url)
                let reader = PlatformWalletPersistenceHandler(modelContainer: reopened, network: .testnet)
                assertRestoreFails(.notes, with: reader)
                let context = ModelContext(reopened)
                let histories = try context.fetch(FetchDescriptor<PersistentShieldedActivity>())
                let aggregate = try XCTUnwrap(histories.first { $0.walletId == walletId && $0.accountIndex == 0 })
                XCTAssertEqual(aggregate.entryId, Data(repeating: 9, count: 32))
                XCTAssertEqual(aggregate.amount, 300)
                XCTAssertEqual(aggregate.noteCmxs, Data(repeating: 1, count: 32) + Data(repeating: 2, count: 32))
                XCTAssertEqual(try reader.loadCachedAssetLocks(walletId: walletId).first?.statusRaw, 0)
                let rows = try context.fetch(FetchDescriptor<PersistentShieldedNote>())
                XCTAssertEqual(rows.count, 2)
                let unrelated = try XCTUnwrap(rows.first { $0.position == 2 })
                unrelated[keyPath: field] = note(2)[keyPath: field]
                try context.save()
                let repaired = PlatformWalletPersistenceHandler(modelContainer: reopened, network: .testnet)
                let restored = restore(.notes, with: repaired)
                XCTAssertEqual(restored.code, 0)
                XCTAssertEqual(restored.count, 2)
                XCTAssertTrue(restored.hasEntries)
            }
        }
    }

    func testOutgoingRestoreRejectsMalformedRowsWithoutReturningValidPrefix() throws {
        let fields: [ReferenceWritableKeyPath<PersistentShieldedOutgoingNote, Data>] = [
            \.walletId, \.cmx, \.recipient,
        ]
        for field in fields {
            let container = try DashModelContainer.createInMemory()
            let context = ModelContext(container)
            context.insert(PersistentWallet(walletId: walletId, network: .testnet))
            for id: UInt8 in [1, 2] {
                let row = PersistentShieldedOutgoingNote(walletId: walletId, accountIndex: 0,
                    cmx: Data(repeating: id, count: 32), recipient: Data(repeating: id, count: 43),
                    value: 100, memo: Data(), blockHeight: 1)
                if id == 2 { row[keyPath: field] = Data([2]) }
                context.insert(row)
            }
            try context.save()
            assertRestoreFails(.outgoing, with: PlatformWalletPersistenceHandler(
                modelContainer: container, network: .testnet))
        }
    }

    func testSyncStateRestoreRejectsMalformedWalletBeforeNetworkFiltering() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentShieldedSyncState(walletId: walletId, accountIndex: 0, lastSyncedIndex: 2))
        context.insert(PersistentShieldedSyncState(walletId: Data([2]), accountIndex: 1, lastSyncedIndex: 4))
        try context.save()
        assertRestoreFails(.syncStates, with: PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet))
    }

    func testActivityAndViewingKeyRestoreRejectMalformedWalletBeforeFiltering() throws {
        let container = try DashModelContainer.createInMemory()
        seed(PlatformWalletPersistenceHandler(modelContainer: container))
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let histories = try context.fetch(FetchDescriptor<PersistentShieldedActivity>())
        let history = try XCTUnwrap(histories.first { $0.walletId == walletId })
        history.walletId = Data([2])
        context.insert(PersistentShieldedViewingKey(walletId: walletId, accountIndex: 0,
            fvkBytes: Data(repeating: 1, count: 96)))
        context.insert(PersistentShieldedViewingKey(walletId: Data([2]), accountIndex: 1,
            fvkBytes: Data(repeating: 2, count: 96)))
        try context.save()
        for kind in [RestoreKind.activity, .viewingKeys] {
            assertRestoreFails(kind, with: PlatformWalletPersistenceHandler(
                modelContainer: container, network: .testnet))
        }
    }

    func testShieldedRestorePropagatesTableAndNetworkScopeFetchFailures() throws {
        let container = try DashModelContainer.createInMemory()
        for kind in RestoreKind.allCases {
            for model in [kind.model, PersistentWallet.self] {
                let fetcher = FetchFaultInjector(faulting: model)
                assertRestoreFails(kind, with: PlatformWalletPersistenceHandler(
                    modelContainer: container, network: .testnet, modelFetcher: fetcher))
                XCTAssertTrue(fetcher.observedReads.contains(String(describing: model)))
            }
        }
    }

    func testShieldedRestoreRejectsMalformedWalletScope() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: Data([1]), network: .testnet))
        try context.save()
        for kind in RestoreKind.allCases {
            assertRestoreFails(kind, with: PlatformWalletPersistenceHandler(
                modelContainer: container, network: .testnet))
        }
    }

    func testValidScanRestoreStillScopesByNetworkAndAllowsEmptySnapshots() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentWallet(walletId: otherWalletId, network: .mainnet))
        for (id, wallet) in [(UInt8(1), walletId), (UInt8(2), otherWalletId)] {
            context.insert(note(id, wallet: wallet))
            context.insert(PersistentShieldedOutgoingNote(walletId: wallet, accountIndex: 0,
                cmx: Data(repeating: id, count: 32), recipient: Data(repeating: id, count: 43),
                value: 100, memo: Data(), blockHeight: 1))
            context.insert(PersistentShieldedSyncState(walletId: wallet, accountIndex: 0, lastSyncedIndex: 2))
            context.insert(PersistentShieldedViewingKey(walletId: wallet, accountIndex: 0,
                fvkBytes: Data(repeating: id, count: 96)))
        }
        try context.save()
        for kind in [RestoreKind.notes, .outgoing, .syncStates, .viewingKeys] {
            let scoped = restore(kind, with: PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet))
            XCTAssertEqual(scoped.code, 0)
            XCTAssertEqual(scoped.count, 1)
            XCTAssertTrue(scoped.hasEntries)
            let unscoped = restore(kind, with: PlatformWalletPersistenceHandler(modelContainer: container))
            XCTAssertEqual(unscoped.code, 0)
            XCTAssertEqual(unscoped.count, 2)
            let empty = restore(kind, with: PlatformWalletPersistenceHandler(
                modelContainer: try DashModelContainer.createInMemory(), network: .testnet))
            XCTAssertEqual(empty.code, 0)
            XCTAssertEqual(empty.count, 0)
            XCTAssertFalse(empty.hasEntries)
        }
    }
}
