import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the chain truncations delivered through the persistence
/// extension's `on_persist_wallet_changeset_truncations_fn`.
///
/// A fork that replaces the chain above a height takes back what the
/// wallet recorded there: transactions of the dropped blocks are removed
/// or, when InstantSend-locked, kept as unconfirmed; the coins the removed
/// ones spent are spendable again; the scan resumes from the fork. A
/// mirror that ignores this hands the replaced branch back at the next
/// load.
///
/// Every fixture starts from the same shape: a coin funded at height 100,
/// and a spend of it mined at 101 that pays change back, with the wallet
/// synced to 101.
@MainActor
final class ChainTruncationPersistTests: XCTestCase {

    private let walletId = Data(repeating: 0x01, count: 32)
    private let fundingTxid = Data(repeating: 0x51, count: 32)
    private let spendTxid = Data(repeating: 0x52, count: 32)

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// File-backed variant: a restart is a second container over the same
    /// store, which an in-memory store cannot outlive.
    private func makeHandler(url: URL) throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let configuration = ModelConfiguration(schema: DashModelContainer.schema, url: url)
        let container = try ModelContainer(
            for: DashModelContainer.schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [configuration]
        )
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// The coin at `fundingTxid:0` (height 100), spent by `spendTxid` mined
    /// at 101, which created change at vout 0 and a second output at
    /// vout 1. The wallet is synced to 101.
    private func seedSpendMinedAboveTheFork(in container: ModelContainer) throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet, syncedHeight: 101))

        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 100_000
        )
        let spend = PersistentTransaction(
            txid: spendTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 2,
            blockHeight: 101,
            netAmount: -40_000
        )
        spend.blockHash = Data(repeating: 0x08, count: 32)
        context.insert(funding)
        context.insert(spend)

        let coin = PersistentTxo(
            transaction: funding, vout: 0, amount: 100_000, address: "yFundAddr", height: 100
        )
        coin.walletId = walletId
        coin.isConfirmed = true
        coin.isSpent = true
        coin.spendingTransaction = spend
        coin.spendingInputIndex = 0
        context.insert(coin)

        for vout: UInt32 in [0, 1] {
            let output = PersistentTxo(
                transaction: spend, vout: vout, amount: 30_000, address: "yChangeAddr", height: 101
            )
            output.walletId = walletId
            output.isConfirmed = true
            context.insert(output)
        }

        try context.save()
    }

    private struct Truncation {
        var height: UInt32
        var removed: [Data] = []
        var unconfirmed: [Data] = []
        var restored: [(txid: Data, vout: UInt32)] = []
    }

    private func rawTxids(_ txids: [Data]) -> UnsafeMutablePointer<FFIByteTuple32> {
        let buffer = UnsafeMutablePointer<FFIByteTuple32>.allocate(capacity: max(txids.count, 1))
        for (i, txid) in txids.enumerated() {
            var tuple: FFIByteTuple32 = (0, 0, 0, 0, 0, 0, 0, 0,
                                         0, 0, 0, 0, 0, 0, 0, 0,
                                         0, 0, 0, 0, 0, 0, 0, 0,
                                         0, 0, 0, 0, 0, 0, 0, 0)
            Swift.withUnsafeMutableBytes(of: &tuple) { dst in
                txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            buffer.advanced(by: i).initialize(to: tuple)
        }
        return buffer
    }

    /// The truncations callback alone, inside whatever bracket the caller
    /// opened. The nested buffers are allocated explicitly and freed after
    /// the call, as the sweep tests do.
    @discardableResult
    private func stageTruncation(
        _ handler: PlatformWalletPersistenceHandler,
        _ truncation: Truncation
    ) -> Bool {
        let removed = rawTxids(truncation.removed)
        let unconfirmed = rawTxids(truncation.unconfirmed)
        let restored = UnsafeMutablePointer<OutPointFFI>.allocate(
            capacity: max(truncation.restored.count, 1)
        )
        for (i, outpoint) in truncation.restored.enumerated() {
            var entry = OutPointFFI()
            Swift.withUnsafeMutableBytes(of: &entry.txid) { dst in
                outpoint.txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            entry.vout = outpoint.vout
            restored.advanced(by: i).initialize(to: entry)
        }
        defer {
            removed.deinitialize(count: truncation.removed.count)
            removed.deallocate()
            unconfirmed.deinitialize(count: truncation.unconfirmed.count)
            unconfirmed.deallocate()
            restored.deinitialize(count: truncation.restored.count)
            restored.deallocate()
        }

        var entry = ChainTruncationFFI()
        entry.height = truncation.height
        entry.txids = UnsafePointer(removed)
        entry.txids_count = UInt(truncation.removed.count)
        entry.unconfirmed_txids = UnsafePointer(unconfirmed)
        entry.unconfirmed_txids_count = UInt(truncation.unconfirmed.count)
        entry.restored_outpoints = UnsafePointer(restored)
        entry.restored_outpoints_count = UInt(truncation.restored.count)
        return withUnsafePointer(to: &entry) { entryPtr in
            handler.persistWalletChangesetTruncations(
                walletId: walletId,
                truncations: entryPtr,
                count: 1
            )
        }
    }

    /// One committed round carrying a single truncation.
    @discardableResult
    private func truncationRound(
        _ handler: PlatformWalletPersistenceHandler,
        _ truncation: Truncation,
        success: Bool = true
    ) -> Bool {
        handler.beginChangeset(walletId: walletId)
        let applied = stageTruncation(handler, truncation)
        _ = handler.endChangeset(walletId: walletId, success: success && applied)
        return applied
    }

    /// Stage `spendTxid` mined at `blockHeight`, spending the funded coin
    /// and creating output 0 again, together with the synced height — the
    /// part of a round that follows its truncation. The caller brackets it.
    private func stageSpendMinedAgain(
        _ handler: PlatformWalletPersistenceHandler,
        blockHeight: UInt32,
        blockHash: Data
    ) {
        let name = strdup("Standard { index: 0 }")
        let address = strdup("yChangeAddr")
        defer {
            free(name)
            free(address)
        }

        var input = OutPointFFI()
        Swift.withUnsafeMutableBytes(of: &input.txid) { dst in
            fundingTxid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        input.vout = 0

        var record = TransactionRecordFFI()
        Swift.withUnsafeMutableBytes(of: &record.txid) { dst in
            spendTxid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        Swift.withUnsafeMutableBytes(of: &record.block_hash) { dst in
            blockHash.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        record.context = 2
        record.block_height = blockHeight

        var utxo = UtxoEntryFFI()
        Swift.withUnsafeMutableBytes(of: &utxo.outpoint.txid) { dst in
            spendTxid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        utxo.outpoint.vout = 0
        utxo.amount = 30_000
        utxo.address = address
        utxo.height = blockHeight
        utxo.is_confirmed = true

        withUnsafeMutablePointer(to: &input) { inputPtr in
            record.input_outpoints = inputPtr
            record.input_outpoints_count = 1
            withUnsafeMutablePointer(to: &record) { recordPtr in
                withUnsafeMutablePointer(to: &utxo) { utxoPtr in
                    var account = AccountChangeSetFFI()
                    account.account_type_name = name
                    account.transactions = recordPtr
                    account.transactions_count = 1
                    account.utxos_added = utxoPtr
                    account.utxos_added_count = 1
                    withUnsafeMutablePointer(to: &account) { accountPtr in
                        var cs = WalletChangeSetFFI()
                        cs.has_chain = true
                        cs.chain.has_synced_height = true
                        cs.chain.synced_height = blockHeight
                        cs.accounts = accountPtr
                        cs.accounts_count = 1
                        withUnsafePointer(to: &cs) { csPtr in
                            _ = handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                        }
                    }
                }
            }
        }
    }

    private func transaction(_ container: ModelContainer, txid: Data) -> PersistentTransaction? {
        let context = ModelContext(container)
        let descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == txid }
        )
        return try? context.fetch(descriptor).first
    }

    private func txo(_ container: ModelContainer, txid: Data, vout: UInt32) -> PersistentTxo? {
        let outpoint = PersistentTxo.makeOutpoint(txid: txid, vout: vout)
        let context = ModelContext(container)
        let descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        return try? context.fetch(descriptor).first
    }

    private func syncedHeight(_ container: ModelContainer) throws -> UInt32 {
        let walletId = self.walletId
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        return try XCTUnwrap(try ModelContext(container).fetch(descriptor).first).syncedHeight
    }

    func testShouldRemoveATransactionOfADroppedBlockAndRestoreTheCoinItSpent() throws {
        let (handler, container) = try makeHandler()
        try seedSpendMinedAboveTheFork(in: container)

        let applied = truncationRound(
            handler,
            Truncation(height: 100, removed: [spendTxid], restored: [(txid: fundingTxid, vout: 0)])
        )

        XCTAssertTrue(applied)
        XCTAssertNil(transaction(container, txid: spendTxid))
        XCTAssertNil(txo(container, txid: spendTxid, vout: 0), "the removed transaction's own output is gone")
        XCTAssertNil(txo(container, txid: spendTxid, vout: 1))
        let restored = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(restored.isSpent, "the coin the removed transaction spent is spendable again")
        XCTAssertNil(restored.spendingTransaction)
        XCTAssertNotNil(transaction(container, txid: fundingTxid), "a transaction the fork keeps stays")
        XCTAssertEqual(try syncedHeight(container), 100)
    }

    func testShouldKeepAnInstantLockedTransactionUnconfirmedWhenItsBlockIsDropped() throws {
        let (handler, container) = try makeHandler()
        try seedSpendMinedAboveTheFork(in: container)

        truncationRound(handler, Truncation(height: 100, unconfirmed: [spendTxid]))

        let kept = try XCTUnwrap(transaction(container, txid: spendTxid))
        XCTAssertEqual(
            kept.context,
            TransactionContextType.instantSend.rawValue,
            "unconfirmed, and still InstantSend-locked"
        )
        XCTAssertEqual(kept.blockHeight, 0)
        XCTAssertNil(kept.blockHash)
        let output = try XCTUnwrap(txo(container, txid: spendTxid, vout: 0))
        XCTAssertFalse(output.isConfirmed)
        XCTAssertTrue(output.isInstantLocked)
        XCTAssertEqual(output.height, 0)
        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent, "what it spent stays spent")
        XCTAssertEqual(coin.spendingTransaction?.txid, spendTxid)
        XCTAssertEqual(try syncedHeight(container), 100)
    }

    /// The round that removes a transaction also records it in its new
    /// block: one row per key, carrying the new block, before and after a
    /// restart.
    func testShouldStoreATransactionOnceInItsNewBlockWhenTheSameRoundMinesItAgain() throws {
        let storeURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("truncation-mined-again-\(UUID().uuidString).store")
        defer { try? FileManager.default.removeItem(at: storeURL) }
        let newBlock = Data(repeating: 0x09, count: 32)

        do {
            let (handler, container) = try makeHandler(url: storeURL)
            try seedSpendMinedAboveTheFork(in: container)

            // Rust's order: the truncation first, then the round as the
            // wallet holds it after the fork.
            handler.beginChangeset(walletId: walletId)
            XCTAssertTrue(stageTruncation(
                handler,
                Truncation(height: 100, removed: [spendTxid], restored: [(txid: fundingTxid, vout: 0)])
            ))
            stageSpendMinedAgain(handler, blockHeight: 102, blockHash: newBlock)
            XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        }

        let (_, container) = try makeHandler(url: storeURL)

        let context = ModelContext(container)
        let spendTxid = self.spendTxid
        let rows = try context.fetch(
            FetchDescriptor<PersistentTransaction>(predicate: #Predicate { $0.txid == spendTxid })
        )
        XCTAssertEqual(rows.count, 1, "the transaction is stored once")
        let mined = try XCTUnwrap(rows.first)
        XCTAssertEqual(mined.blockHeight, 102)
        XCTAssertEqual(mined.blockHash, newBlock)

        let output = try XCTUnwrap(
            txo(container, txid: spendTxid, vout: 0),
            "the output the new block's record brought back is stored"
        )
        XCTAssertEqual(output.height, 102)
        XCTAssertFalse(output.isSpent)
        XCTAssertNil(
            txo(container, txid: spendTxid, vout: 1),
            "an output the round did not create again is gone"
        )
        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent, "spent again, by the transaction in its new block")
        XCTAssertEqual(coin.spendingTransaction?.txid, spendTxid)
        XCTAssertEqual(try syncedHeight(container), 102)
    }

    func testShouldLeaveASyncedHeightBelowTheForkAlone() throws {
        let (handler, container) = try makeHandler()
        try seedSpendMinedAboveTheFork(in: container)

        truncationRound(handler, Truncation(height: 150))

        XCTAssertEqual(try syncedHeight(container), 101)
    }

    func testShouldRollATruncationBackWithItsRound() throws {
        let (handler, container) = try makeHandler()
        try seedSpendMinedAboveTheFork(in: container)

        truncationRound(
            handler,
            Truncation(height: 100, removed: [spendTxid], restored: [(txid: fundingTxid, vout: 0)]),
            success: false
        )

        XCTAssertNotNil(transaction(container, txid: spendTxid))
        XCTAssertNotNil(txo(container, txid: spendTxid, vout: 0))
        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent)
        XCTAssertEqual(coin.spendingTransaction?.txid, spendTxid)
        XCTAssertEqual(try syncedHeight(container), 101)
    }

    func testShouldTreatATruncationForAMissingWalletAsASuccessfulNoOp() throws {
        let (handler, _) = try makeHandler()

        XCTAssertTrue(truncationRound(handler, Truncation(height: 100, removed: [spendTxid])))
    }
}
