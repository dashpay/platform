import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for the wallet-changeset apply path after the per-round
/// bulk-prefetch cache (`WalletChangesetRoundCache`) replaced the
/// per-row `ModelContext.fetch` storm:
///
/// * spend linkage stays order-independent (spending tx before funding
///   TXO within one round resolves through the pending-input table);
/// * inputs with unknown funding keep the unconditional pending row —
///   the out-of-order spend-repair mechanism the cache must not regress;
/// * a round issues O(chunks) fetches, not O(rows) (the quadratic
///   per-row-fetch regression guard);
/// * a thrown single-row fallback fetch rejects the round instead of
///   reading as "row absent" and licensing a duplicate insert.
@MainActor
final class WalletChangesetRoundTests: XCTestCase {

    private let walletId = Data(repeating: 0x0A, count: 32)

    /// Lightweight description of one transaction record for the
    /// FFI-struct builder below.
    private struct TestTx {
        var txid: Data
        /// 0=incoming … 3=coinJoin (`TransactionRecordFFI.direction`).
        var direction: UInt32 = 0
        var inputs: [(txid: Data, vout: UInt32)] = []
        /// vouts to emit as `utxos_added` entries for this tx.
        var outputs: [UInt32] = []
    }

    private func makeHandler(
        modelFetcher: ModelFetching = LiveModelFetcher()
    ) throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet,
            modelFetcher: modelFetcher
        )
        // The changeset path drops writes for unknown wallets — seed
        // the row the way the wallet-metadata callback would have.
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        return (handler, container)
    }

    /// `txs` where tx_i spends tx_{i-1}'s only output: the same-round
    /// chain that exercises every pending-input path.
    private func spendChain(count: Int) -> [TestTx] {
        (0..<count).map { i in
            var tx = TestTx(txid: makeTxid(i), outputs: [0])
            if i > 0 { tx.inputs = [(makeTxid(i - 1), 0)] }
            return tx
        }
    }

    /// Build the C changeset for `txs`, run one begin→persist→end
    /// round through `handler`, and free every allocation. Returns
    /// what `endChangeset` reported; `expectPersisted` pins what the
    /// changeset callback itself must have reported.
    private func runRound(
        handler: PlatformWalletPersistenceHandler,
        txs: [TestTx],
        expectPersisted: Bool = true
    ) -> Bool {
        var cStrings: [UnsafeMutablePointer<CChar>] = []
        var inputBuffers: [(UnsafeMutablePointer<OutPointFFI>, Int)] = []
        defer {
            for ptr in cStrings { free(ptr) }
            for (ptr, count) in inputBuffers {
                ptr.deinitialize(count: count)
                ptr.deallocate()
            }
        }

        let txBuffer = UnsafeMutablePointer<TransactionRecordFFI>.allocate(capacity: txs.count)
        let totalOutputs = txs.reduce(0) { $0 + $1.outputs.count }
        let utxoBuffer = UnsafeMutablePointer<UtxoEntryFFI>.allocate(capacity: max(totalOutputs, 1))
        defer {
            txBuffer.deinitialize(count: txs.count)
            txBuffer.deallocate()
            utxoBuffer.deinitialize(count: totalOutputs)
            utxoBuffer.deallocate()
        }

        var utxoCount = 0
        for (i, tx) in txs.enumerated() {
            var record = TransactionRecordFFI()
            record.txid = tuple32(tx.txid)
            record.tx_data = nil
            record.tx_data_len = 0
            record.context = 2 // inBlock — spends may flip `isSpent`
            record.block_height = 1_000 + UInt32(i)
            record.direction = tx.direction
            let typeName = strdup("Standard")!
            cStrings.append(typeName)
            record.transaction_type = typeName
            record.transaction_type_kind = tx.direction == 3 ? 1 : 0
            record.net_amount = 1_000
            record.first_seen = 1_700_000_000
            if tx.inputs.isEmpty {
                record.input_outpoints = nil
                record.input_outpoints_count = 0
            } else {
                let inputs = UnsafeMutablePointer<OutPointFFI>.allocate(capacity: tx.inputs.count)
                for (j, input) in tx.inputs.enumerated() {
                    var op = OutPointFFI()
                    op.txid = tuple32(input.txid)
                    op.vout = input.vout
                    inputs[j] = op
                }
                inputBuffers.append((inputs, tx.inputs.count))
                record.input_outpoints = inputs
                record.input_outpoints_count = UInt(tx.inputs.count)
            }
            txBuffer[i] = record

            for vout in tx.outputs {
                var utxo = UtxoEntryFFI()
                utxo.outpoint = OutPointFFI()
                utxo.outpoint.txid = tuple32(tx.txid)
                utxo.outpoint.vout = vout
                utxo.amount = 5_000
                let address = strdup("addr-\(i)-\(vout)")!
                cStrings.append(address)
                utxo.address = address
                utxo.script_pubkey = nil
                utxo.script_pubkey_len = 0
                utxo.height = 1_000 + UInt32(i)
                utxo.is_confirmed = true
                utxoBuffer[utxoCount] = utxo
                utxoCount += 1
            }
        }

        var account = AccountChangeSetFFI()
        let accountName = strdup("Standard")!
        cStrings.append(accountName)
        account.account_type_name = accountName
        account.account_index = 0
        account.transactions = txBuffer
        account.transactions_count = UInt(txs.count)
        account.utxos_added = utxoCount > 0 ? utxoBuffer : nil
        account.utxos_added_count = UInt(utxoCount)

        return withUnsafeMutablePointer(to: &account) { accountPtr in
            var changeset = WalletChangeSetFFI()
            changeset.accounts = accountPtr
            changeset.accounts_count = 1
            handler.beginChangeset(walletId: walletId)
            let persisted = withUnsafePointer(to: changeset) {
                handler.persistWalletChangeset(walletId: walletId, changeset: $0)
            }
            XCTAssertEqual(persisted, expectPersisted, "changeset callback result")
            // What Rust does with the callback's code: close the round
            // as failed when any per-kind callback reported failure.
            return handler.endChangeset(walletId: walletId, success: persisted)
        }
    }

    private func fetchAll<T: PersistentModel>(
        _ type: T.Type,
        in container: ModelContainer
    ) throws -> [T] {
        try ModelContext(container).fetch(FetchDescriptor<T>())
    }

    // MARK: - Correctness

    /// A same-round chain of spends (tx_i spends tx_{i-1}'s output,
    /// records applied before any UTXO) must resolve every linkage
    /// through the pending-input table and leave no pending rows.
    func testSameRoundSpendChainResolvesAndDrainsPendingRows() throws {
        let (handler, container) = try makeHandler()
        let count = 50
        XCTAssertTrue(runRound(handler: handler, txs: spendChain(count: count)))

        let transactions = try fetchAll(PersistentTransaction.self, in: container)
        XCTAssertEqual(transactions.count, count)

        let txos = try fetchAll(PersistentTxo.self, in: container)
        XCTAssertEqual(txos.count, count)
        for txo in txos {
            let fundingIndex = txo.outpoint.withUnsafeBytes { $0.loadUnaligned(as: UInt64.self) }
            if fundingIndex < UInt64(count - 1) {
                XCTAssertTrue(txo.isSpent, "TXO of tx \(fundingIndex) should be spent")
                XCTAssertEqual(
                    txo.spendingTransaction?.txid,
                    makeTxid(Int(fundingIndex) + 1),
                    "TXO of tx \(fundingIndex) should be linked to its spender"
                )
            } else {
                XCTAssertFalse(txo.isSpent, "tip TXO should stay unspent")
            }
        }

        let pending = try fetchAll(PersistentPendingInput.self, in: container)
        XCTAssertTrue(pending.isEmpty, "all pending rows should have drained, got \(pending.count)")
    }

    /// A transaction spending an outpoint whose funding tx is unknown
    /// must still write the pending-input row — that row is the
    /// out-of-order spend-repair mechanism (gap-limit discovery,
    /// mid-sync restart), and the cache-backed dup-check must not
    /// swallow it.
    func testUnknownFundingInputWritesPendingRow() throws {
        let (handler, container) = try makeHandler()
        let unknownFunding = makeTxid(500)
        XCTAssertTrue(runRound(handler: handler, txs: [
            TestTx(txid: makeTxid(1), inputs: [(unknownFunding, 2)]),
        ]))

        let pending = try fetchAll(PersistentPendingInput.self, in: container)
        XCTAssertEqual(pending.count, 1)
        XCTAssertEqual(
            pending.first?.outpoint,
            PersistentTxo.makeOutpoint(txid: unknownFunding, vout: 2)
        )
        XCTAssertEqual(pending.first?.spendingTxid, makeTxid(1))
    }

    // MARK: - Fetch failure

    /// A thrown single-row fallback fetch must reject the round, not
    /// read as "row absent": the callers take `nil` as license to
    /// insert over a `.unique` column, and that duplicate would only
    /// surface as a failed `save()` at `endChangeset`. Faulting every
    /// `PersistentTransaction` read fails the bulk prefetch (which
    /// demotes the chunk to per-row fetches) and then the first
    /// fallback, so this pins both halves of the contract.
    func testThrownFallbackFetchRejectsTheRound() throws {
        let injector = FetchFaultInjector(faulting: PersistentTransaction.self)
        let (handler, container) = try makeHandler(modelFetcher: injector)

        XCTAssertFalse(
            runRound(handler: handler, txs: spendChain(count: 3), expectPersisted: false),
            "an unreadable transaction table must fail the round"
        )
        XCTAssertTrue(
            injector.observedReads.contains("PersistentTransaction"),
            "the faulted read must be the transaction fetch"
        )
        XCTAssertTrue(
            try fetchAll(PersistentTransaction.self, in: container).isEmpty,
            "nothing from the rejected round may reach the store"
        )
        XCTAssertTrue(try fetchAll(PersistentTxo.self, in: container).isEmpty)
    }

    // MARK: - Scaling

    /// A round must issue O(chunks) fetches, not O(rows): the per-row
    /// implementation re-scanned every staged object on each fetch, so
    /// round cost grew quadratically. Counting reads through the
    /// `ModelFetching` seam pins that deterministically — a reintroduced
    /// per-row fetch (through the seam) scales the count with the
    /// record count. The fixture starts from an empty store so no key
    /// misses the prefetch; a pre-seeded store could add legitimate
    /// fallback reads (an existing TXO whose stored address differs
    /// from the emitted one).
    func testRoundFetchCountIsIndependentOfRecordCount() throws {
        func fetchCount(records: Int) throws -> Int {
            let injector = FetchFaultInjector()
            let (handler, _) = try makeHandler(modelFetcher: injector)
            XCTAssertTrue(runRound(handler: handler, txs: spendChain(count: records)))
            return injector.observedReads.count
        }
        // Per round: the wallet row, the account row, then one bulk
        // fetch per entity (transactions, TXOs, pending inputs, core
        // addresses) per 900-key chunk — `chunked(_:size:)`'s default.
        // Every entity's key set in a spend chain has `records` members.
        func expected(records: Int) -> Int { 2 + 4 * ((records + 899) / 900) }

        XCTAssertEqual(try fetchCount(records: 100), expected(records: 100))
        XCTAssertEqual(try fetchCount(records: 2_000), expected(records: 2_000))
    }
}
