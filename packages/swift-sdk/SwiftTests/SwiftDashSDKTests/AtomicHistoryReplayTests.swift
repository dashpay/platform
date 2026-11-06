import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

@MainActor
final class AtomicHistoryReplayTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let txid = Data(repeating: 7, count: 32)

    // TODO(macOS-validation): run the reentrant transaction and rollback tests on an Apple SDK runner.
    func testShouldRollbackWritesMadeByAReentrantReplayOperation() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        handler.beginChangeset(walletId: walletId)
        try handler.persistCoreInstantLock(walletId: walletId, txid: txid, bytes: Data([1]))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        var calls = 0
        let result = handler.replayWallet(walletId: walletId) {
            calls += 1
            do {
                try handler.persistCoreInstantLock(walletId: walletId, txid: txid, bytes: Data([2]))
                XCTAssertEqual(try handler.loadCoreInstantLock(walletId: walletId, txid: txid), Data([2]))
                return 71
            } catch {
                XCTFail("Unexpected staging error: \(error)")
                return 72
            }
        }
        XCTAssertNotEqual(result, 0)
        XCTAssertEqual(calls, 1)
        XCTAssertEqual(try handler.loadCoreInstantLock(walletId: walletId, txid: txid), Data([1]))
    }

    func testShouldCommitReplayAndRejectANestedTransaction() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        var nestedCalls = 0
        let result = handler.replayWallet(walletId: walletId) {
            XCTAssertNotEqual(handler.replayWallet(walletId: walletId) {
                nestedCalls += 1
                return 0
            }, 0)
            do {
                try handler.persistCoreInstantLock(walletId: walletId, txid: txid, bytes: Data([3]))
                return 0
            } catch {
                XCTFail("Unexpected staging error: \(error)")
                return 1
            }
        }
        XCTAssertEqual(result, 0)
        XCTAssertEqual(nestedCalls, 0)
        let reopened = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertEqual(try reopened.loadCoreInstantLock(walletId: walletId, txid: txid), Data([3]))
    }

    func testShouldApplyExactClaimStampAndRollbackAllEffectsOnFailure() throws {
        let container = try DashModelContainer.createInMemory()
        let seed = ModelContext(container)
        let transaction = PersistentTransaction(txid: txid, transactionData: Data([1]))
        seed.insert(transaction)
        let coin = PersistentTxo(transaction: transaction, vout: 0, amount: 42, address: "unused", height: 1)
        coin.walletId = walletId
        seed.insert(coin)
        try seed.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        var claim = ReplayClaimEffectFFI()
        withUnsafeMutableBytes(of: &claim.txid) { $0.copyBytes(from: txid) }
        claim.action = 2
        claim.has_claimant = true
        withUnsafeMutableBytes(of: &claim.claimant) { $0.copyBytes(from: Data(repeating: 9, count: 32)) }
        claim.has_mined_height = true
        claim.mined_height = 123
        let result = handler.replayWallet(walletId: walletId) {
            do {
                try withUnsafePointer(to: &claim) { pointer in
                    var effects = HistoryReplayEffectsFFI()
                    effects.claims = pointer
                    effects.claims_count = 1
                    try handler.applyReplayEffects(walletId: walletId, effects: effects)
                }
                let rows = try handler.loadReplayClaims(walletId: walletId)
                defer {
                    for row in rows {
                        if let script = row.script { UnsafeMutablePointer(mutating: script).deallocate() }
                    }
                }
                let held = rows.filter { $0.is_held }
                XCTAssertEqual(held.count, 1, "the output must not add an unstamped duplicate claim")
                XCTAssertEqual(held.first?.mined_height, 123)
                XCTAssertEqual(held.first?.has_mined_height, true)
                return 71
            } catch {
                XCTFail("Unexpected replay error: \(error)")
                return 72
            }
        }
        XCTAssertNotEqual(result, 0)
        let reopened = ModelContext(container)
        XCTAssertFalse(try XCTUnwrap(reopened.fetch(FetchDescriptor<PersistentTxo>()).first).isSpent)
        XCTAssertTrue(try reopened.fetch(FetchDescriptor<PersistentPendingInput>()).isEmpty)
    }

}
