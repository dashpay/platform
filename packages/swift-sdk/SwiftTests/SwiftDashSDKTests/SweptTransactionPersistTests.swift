import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the one subtractive part of the changeset path: the sweep
/// batches delivered through the persistence extension's
/// `on_persist_wallet_changeset_sweeps_fn` alongside each round's
/// `WalletChangeSetFFI`.
///
/// A swept transaction was a recorded spend that a later, final transaction
/// provably beat to one of its inputs, so it can never confirm and Rust has
/// already dropped it. Everything else the round carries is additive, so a
/// mirror that ignores the sweeps keeps the dead row, hands it back at the
/// next load, and re-creates a balance the wallet has already corrected —
/// the bug the upstream sweep exists to fix, one layer up.
///
/// The fixtures model the shape that makes the coins tricky: an unconfirmed
/// loser — upstream sweeps nothing else — spends A and B, and the winner
/// takes only A. Because the loser never reached a block, this store never
/// flipped `isSpent` on either coin, so both are one deleted row away from
/// re-entering the restore set, and only the released set upstream carries
/// says which of them belongs there.
///
/// Loser rows carry REAL consensus bytes (`serializedTransaction`): the
/// sweep keys its hold on the loser's decoded inputs, not on the links
/// the row happens to hold, so a fixture with undecodable bytes would only
/// exercise the link-keyed fallback.
@MainActor
final class SweptTransactionPersistTests: XCTestCase {

    private let walletId = Data(repeating: 0x01, count: 32)
    private let fundingTxid = Data(repeating: 0x41, count: 32)
    private let sweptTxid = Data(repeating: 0x42, count: 32)
    private let winnerTxid = Data(repeating: 0x44, count: 32)

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// File-backed variant of `makeHandler()` — an in-memory store can't
    /// outlive its own `ModelContainer`, so simulating a restart (a fresh
    /// load/persister over the same on-disk store) needs a real file two
    /// separate containers can both point at.
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

    /// Serialize a plain version-2 transaction spending `inputs` (empty
    /// scriptSigs) with `outputs` empty-script outputs, in the form
    /// `TransactionDecoder` parses — which is what `applySweptTransaction`
    /// decodes a loser's inputs from. The txid of these bytes is NOT the
    /// fixture's row key; nothing in the store compares the two.
    private func serializedTransaction(
        inputs: [(txid: Data, vout: UInt32)],
        outputs: Int = 1
    ) -> Data {
        var bytes = Data()
        bytes.append(contentsOf: withUnsafeBytes(of: UInt32(2).littleEndian) { Data($0) })
        bytes.append(UInt8(inputs.count))
        for input in inputs {
            bytes.append(input.txid)
            bytes.append(contentsOf: withUnsafeBytes(of: input.vout.littleEndian) { Data($0) })
            bytes.append(0x00) // empty scriptSig
            bytes.append(contentsOf: [0xff, 0xff, 0xff, 0xff]) // sequence
        }
        bytes.append(UInt8(outputs))
        for _ in 0..<outputs {
            bytes.append(contentsOf: withUnsafeBytes(of: UInt64(1_000).littleEndian) { Data($0) })
            bytes.append(0x00) // empty scriptPubKey
        }
        bytes.append(contentsOf: [0x00, 0x00, 0x00, 0x00]) // locktime
        return bytes
    }

    /// A mempool-context loser row spending `inputs`, the only kind of
    /// record upstream sweeps, with decodable bytes.
    private func loserRow(
        txid: Data,
        spending inputs: [(txid: Data, vout: UInt32)],
        netAmount: Int64 = -100_000
    ) -> PersistentTransaction {
        PersistentTransaction(
            txid: txid,
            transactionData: serializedTransaction(inputs: inputs),
            context: 0,
            blockHeight: 0,
            netAmount: netAmount
        )
    }

    /// Seed the shape a confirmed spend leaves behind: a funding transaction
    /// with two outputs, a spending transaction that claimed both (linked
    /// and flagged spent), and the change that spend created.
    ///
    /// `winnerTakesA` models a wallet-relevant winner that already
    /// re-pointed A at itself, which is what the additive half of the round
    /// does before the sweep runs.
    private func seedSpend(in container: ModelContainer, winnerTakesA: Bool) throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 140_000
        )
        // Mempool context: the only kind of record upstream sweeps.
        let swept = loserRow(
            txid: sweptTxid,
            spending: [(txid: fundingTxid, vout: 0), (txid: fundingTxid, vout: 1)],
            netAmount: -140_000
        )
        context.insert(funding)
        context.insert(swept)

        let winner: PersistentTransaction?
        if winnerTakesA {
            let row = PersistentTransaction(
                txid: winnerTxid,
                transactionData: serializedTransaction(inputs: [(txid: fundingTxid, vout: 0)]),
                context: 2,
                blockHeight: 102,
                netAmount: -100_000
            )
            context.insert(row)
            winner = row
        } else {
            winner = nil
        }

        // A — the coin the winner also takes. When the winner is
        // wallet-relevant its confirmed record owns the link and the flag;
        // otherwise A is left where the unconfirmed loser put it, linked and
        // unspent, which is what makes it indistinguishable from B.
        let coinA = PersistentTxo(
            transaction: funding,
            vout: 0,
            amount: 100_000,
            address: "yFundAddr",
            height: 100
        )
        coinA.walletId = walletId
        coinA.isSpent = winner != nil
        coinA.spendingTransaction = winner ?? swept
        context.insert(coinA)

        // B — named only by the loser, and so still unspent.
        let coinB = PersistentTxo(
            transaction: funding,
            vout: 1,
            amount: 40_000,
            address: "yFundAddr",
            height: 100
        )
        coinB.walletId = walletId
        coinB.spendingTransaction = swept
        context.insert(coinB)

        let change = PersistentTxo(
            transaction: swept,
            vout: 0,
            amount: 60_000,
            address: "yChangeAddr",
            height: 0
        )
        change.walletId = walletId
        context.insert(change)

        try context.save()
    }

    /// Drive one changeset round of sweeps through the same entry point the
    /// Rust persister calls.
    /// One sweep batch: the transactions it removed, the winner it is
    /// attributed to, the winner's finality context, and the coins it
    /// freed.
    private struct Batch {
        var losers: [Data]
        var winner: Data
        /// The winner's own mined block height — `SweepBatchFFI`'s
        /// `has_winner_mined_height`/`winner_mined_height` pair. Non-nil
        /// models a block-context sweep (the winner is mined, tombstones
        /// are written and stamped with this height); `nil` models a
        /// mempool-context sweep (the winner is IS-locked and not yet
        /// mined, and no tombstone may be created). Deliberately
        /// undefaulted so every test states which world it is in.
        var winnerMinedHeight: UInt32?
        var released: [(txid: Data, vout: UInt32)] = []
    }

    /// Drive a changeset of sweep batches through the same entry point the
    /// Rust persister calls, preserving their order.
    ///
    /// The nested buffers are allocated explicitly and freed after the call.
    /// `withUnsafeMutableBufferPointer` only guarantees its pointer for the
    /// duration of its own closure, so storing `baseAddress` in a struct the
    /// FFI reads later would hand the consumer a dangling pointer.
    @discardableResult
    private func sweep(
        _ handler: PlatformWalletPersistenceHandler,
        _ batches: [Batch]
    ) -> Bool {
        sweep(handler, batches, walletId: walletId)
    }

    /// `walletId`-parameterized form for the multi-wallet tests below,
    /// where the same shared loser row needs a separate callback per wallet
    /// — each carrying that wallet's own `released` set, the way two real
    /// `persistWalletChangeset` calls would.
    @discardableResult
    private func sweep(
        _ handler: PlatformWalletPersistenceHandler,
        _ batches: [Batch],
        walletId: Data
    ) -> Bool {
        var applied = false
        round(handler, walletId: walletId) {
            applied = stageSweeps(handler, batches, walletId: walletId)
            return applied
        }
        return applied
    }

    /// One begin/end bracket, the way every Rust `store()` round is
    /// delivered. `body` returns the round's success, which `endChangeset`
    /// commits or rolls back on.
    private func round(
        _ handler: PlatformWalletPersistenceHandler,
        walletId: Data? = nil,
        _ body: () -> Bool
    ) {
        let walletId = walletId ?? self.walletId
        handler.beginChangeset(walletId: walletId)
        let success = body()
        _ = handler.endChangeset(walletId: walletId, success: success)
    }

    /// The sweeps callback alone, inside whatever bracket the caller
    /// opened — so a test can stage a record and a sweep between ONE
    /// `beginChangeset`/`endChangeset` pair, the shape Rust produces when
    /// it folds a winner's detection and the loser's sweep into one round.
    @discardableResult
    private func stageSweeps(
        _ handler: PlatformWalletPersistenceHandler,
        _ batches: [Batch],
        walletId: Data
    ) -> Bool {
        typealias RawTxid = (
            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
        )

        var txidBuffers: [UnsafeMutablePointer<RawTxid>] = []
        var releasedBuffers: [UnsafeMutablePointer<OutPointFFI>] = []
        var ffiBatches: [SweepBatchFFI] = []
        defer {
            for (i, buf) in txidBuffers.enumerated() {
                buf.deinitialize(count: batches[i].losers.count)
                buf.deallocate()
            }
            for (i, buf) in releasedBuffers.enumerated() {
                buf.deinitialize(count: batches[i].released.count)
                buf.deallocate()
            }
        }

        for batch in batches {
            let txids = UnsafeMutablePointer<RawTxid>.allocate(capacity: max(batch.losers.count, 1))
            for (i, loser) in batch.losers.enumerated() {
                var tuple: RawTxid = (0, 0, 0, 0, 0, 0, 0, 0,
                                      0, 0, 0, 0, 0, 0, 0, 0,
                                      0, 0, 0, 0, 0, 0, 0, 0,
                                      0, 0, 0, 0, 0, 0, 0, 0)
                Swift.withUnsafeMutableBytes(of: &tuple) { dst in
                    loser.withUnsafeBytes { src in dst.copyMemory(from: src) }
                }
                txids.advanced(by: i).initialize(to: tuple)
            }
            txidBuffers.append(txids)

            let freed = UnsafeMutablePointer<OutPointFFI>.allocate(
                capacity: max(batch.released.count, 1)
            )
            for (i, outpoint) in batch.released.enumerated() {
                var entry = OutPointFFI()
                Swift.withUnsafeMutableBytes(of: &entry.txid) { dst in
                    outpoint.txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
                }
                entry.vout = outpoint.vout
                freed.advanced(by: i).initialize(to: entry)
            }
            releasedBuffers.append(freed)

            var entry = SweepBatchFFI()
            entry.txids = UnsafePointer(txids)
            entry.txids_count = UInt(batch.losers.count)
            entry.released_outpoints = UnsafePointer(freed)
            entry.released_outpoints_count = UInt(batch.released.count)
            Swift.withUnsafeMutableBytes(of: &entry.superseded_by) { dst in
                batch.winner.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            // The winner's finality context: `has_winner_mined_height`
            // false is the mempool path (IS-locked, unmined winner —
            // no tombstone may be created), true carries the winner's
            // own mined block.
            entry.has_winner_mined_height = batch.winnerMinedHeight != nil
            entry.winner_mined_height = batch.winnerMinedHeight ?? 0
            ffiBatches.append(entry)
        }

        let sweeps = UnsafeMutablePointer<SweepBatchFFI>.allocate(
            capacity: max(ffiBatches.count, 1)
        )
        sweeps.initialize(from: ffiBatches, count: ffiBatches.count)
        defer {
            sweeps.deinitialize(count: ffiBatches.count)
            sweeps.deallocate()
        }

        // The extension entry point, not a `WalletChangeSetFFI` field: the
        // Rust persister delivers sweeps through the size-negotiated
        // `on_persist_wallet_changeset_sweeps_fn` in the same round as the
        // changeset callback, and this drives the Swift side of exactly
        // that call.
        return handler.persistWalletChangesetSweeps(
            walletId: walletId,
            sweeps: UnsafePointer(sweeps),
            count: UInt(ffiBatches.count)
        )
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

    /// The row and everything it created go; the funding transaction and its
    /// coins stay.
    func testSweptTransactionAndItsOutputsAreDeleted() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)

        sweep(handler, [
            Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 1)])
        ])

        XCTAssertNil(transaction(container, txid: sweptTxid), "the swept row is gone")
        XCTAssertNil(txo(container, txid: sweptTxid, vout: 0), "the change it created is gone with it")
        XCTAssertNotNil(transaction(container, txid: fundingTxid), "the funding transaction is untouched")
    }

    /// The released set is applied verbatim: the coin it names comes back,
    /// and the one it does not stays out — the winner took that one.
    func testSweepFreesOnlyTheInputsTheWinnerDidNotTake() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)

        sweep(handler, [
            Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 1)])
        ])

        let takenByWinner = txo(container, txid: fundingTxid, vout: 0)
        XCTAssertNotNil(takenByWinner)
        XCTAssertTrue(takenByWinner!.isSpent, "the coin the winner took stays spent")
        XCTAssertEqual(takenByWinner!.spendingTransaction?.txid, winnerTxid)

        let losersOwn = txo(container, txid: fundingTxid, vout: 1)
        XCTAssertNotNil(losersOwn)
        XCTAssertFalse(losersOwn!.isSpent, "the loser's own input is free again")
        XCTAssertNil(losersOwn!.spendingTransaction)
    }

    /// The winner does not have to reach this store at all: it can spend our
    /// coin while paying only to outside addresses, and then no record for it
    /// is ever written here. Nothing on hand could separate the coin it took
    /// from the loser's own — upstream can, and says so through the released
    /// set, which is the entire reason that set is carried.
    func testAnAbsentWinnerStillKeepsItsOwnInputSpent() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)

        sweep(handler, [
            Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 1)])
        ])

        XCTAssertNil(transaction(container, txid: sweptTxid), "the swept row still goes")

        let takenByWinner = txo(container, txid: fundingTxid, vout: 0)
        XCTAssertNotNil(takenByWinner)
        XCTAssertTrue(
            takenByWinner!.isSpent,
            "a coin the chain has already spent must not come back"
        )
        XCTAssertNil(takenByWinner!.spendingTransaction, "and no spender is invented for it")
        XCTAssertEqual(
            takenByWinner!.supersededByTxid,
            winnerTxid,
            "the hold is attributed to the winner — SQLite's spent_in_txid, mirrored"
        )

        let losersOwn = txo(container, txid: fundingTxid, vout: 1)
        XCTAssertNotNil(losersOwn)
        XCTAssertFalse(
            losersOwn!.isSpent,
            "the loser's own input is free, winner record or not"
        )
    }

    /// A MATERIALISED coin the wallet hands back as unspent follows the
    /// wallet, stamped hold or not. This test used to pin the opposite —
    /// "the recovery clear refuses stamped rows" — on the reasoning that a
    /// restore-rescan re-finds the funding output blind to an unconfirmed
    /// winner. That reasoning only holds for a coin the wallet has never
    /// materialised (the tombstone's job, see the drain tests below): a
    /// coin the wallet knows is one whose every network-final spender is
    /// wallet-relevant by BIP158 prevout matching, so the wallet's own scan
    /// re-discovers the spend and its view is authoritative — and refusing
    /// the re-delivery locks a real coin out forever after a reorg of the
    /// winner, since a row with `isSpent == true` is never restored to Rust
    /// again. The reference store's upsert valve was narrowed to
    /// never-materialised placeholders for exactly this reason; this is the
    /// same rule.
    func testWalletReDeliveringAMaterialisedHeldCoinFreesIt() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)
        sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])
        let held = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(held.isSpent, "sanity: the sweep held the coin")
        XCTAssertEqual(held.supersededByTxid, winnerTxid)

        redeliverCoinB(handler)

        let freed = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertFalse(freed.isSpent, "the wallet re-delivering a coin it knows frees it")
        XCTAssertNil(freed.supersededByTxid, "the stamp clears with the hold")
        XCTAssertNil(freed.spendingTransaction)
    }

    /// The one shape the re-delivery does not free: a coin linked to a
    /// spender with context at or above InstantSend-locked. Confirmed
    /// evidence on record is never displaced by a re-delivery — the spend
    /// emit and the record pass own that link.
    func testWalletReDeliveringACoinLinkedToASettledSpenderKeepsItSpent() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)
        let context = ModelContext(container)
        let coinB = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 1)
        let row = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentTxo>(
                predicate: #Predicate { $0.outpoint == coinB }
            )).first
        )
        let winnerTxid = self.winnerTxid
        row.isSpent = true
        row.spendingTransaction = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == winnerTxid }
            )).first
        )
        try context.save()

        redeliverCoinB(handler)

        let kept = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(kept.isSpent, "a coin an in-block spender holds stays spent")
        XCTAssertEqual(kept.spendingTransaction?.txid, winnerTxid)
    }

    /// The winner's own record can reach this store only after the sweep
    /// and the funding TXO already did — IS-locked, not yet in a block.
    /// Both writers it flows through resolved the in-block gate to false
    /// and wrote it outright: `resolveInputOutpoint` on the record pass,
    /// then `markUtxoSpent` on the `utxos_spent` emit riding the same
    /// round. Either flipped the durable stamped hold back into the
    /// restore set until the winner confirmed — contradicting the verdict
    /// the sweep already recorded (and the handler's own "winner is
    /// already final" reasoning).
    func testAWinnersLateRecordDoesNotDowngradeAStampedHold() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let l = loserRow(txid: sweptTxid, spending: [(txid: fundingTxid, vout: 0)])
        context.insert(l)
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: sweptTxid,
            spendingTransaction: l,
            walletId: walletId
        ))
        try context.save()

        // The sweep holds the claim; the funding TXO then materializes it
        // as a stamped hold.
        sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])
        deliverFundingUtxo(handler, vout: 0, amount: 100_000)
        XCTAssertTrue(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)

        // The winner's own record finally arrives, IS-locked (context 1 <
        // in-block), with the spent emit riding along the way a real round
        // delivers both.
        deliverRecordWithSpentEmit(
            handler,
            txid: winnerTxid,
            context: 1,
            inputOutpoint: (txid: fundingTxid, vout: 0)
        )

        let held = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(
            held.isSpent,
            "the winner's own unconfirmed arrival must not downgrade the stamped hold"
        )
        XCTAssertEqual(held.supersededByTxid, winnerTxid)
        XCTAssertEqual(
            held.spendingTransaction?.txid,
            winnerTxid,
            "the spender is linked all the same"
        )
    }

    /// The record-only half of the scenario above: a flush can deliver the
    /// winner's record without a `utxos_spent` emit (the wallet had no live
    /// UTXO to classify — the coin sits as a stamped hold), so
    /// `resolveInputOutpoint`'s own monotonic guard must carry the hold by
    /// itself. Pinned separately because the combined test's spent emit
    /// re-applies the hold through `markUtxoSpent`'s guard, masking a
    /// regression in the record pass alone.
    func testAWinnersLateRecordAloneDoesNotDowngradeAStampedHold() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let l = loserRow(txid: sweptTxid, spending: [(txid: fundingTxid, vout: 0)])
        context.insert(l)
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: sweptTxid,
            spendingTransaction: l,
            walletId: walletId
        ))
        try context.save()

        sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])
        deliverFundingUtxo(handler, vout: 0, amount: 100_000)
        XCTAssertTrue(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)

        deliverRecordWithSpentEmit(
            handler,
            txid: winnerTxid,
            context: 1,
            inputOutpoint: (txid: fundingTxid, vout: 0),
            includeSpentEmit: false
        )

        let held = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(
            held.isSpent,
            "the record pass alone must not downgrade the stamped hold"
        )
        XCTAssertEqual(held.supersededByTxid, winnerTxid)
        XCTAssertEqual(held.spendingTransaction?.txid, winnerTxid)
    }

    /// One changeset round carrying a transaction record and — unless the
    /// caller opts out to pin the record pass alone — the `utxos_spent`
    /// emit for the input it consumed, the shape a real round takes when
    /// the wallet classifies the spend in the same flush as the record.
    private func deliverRecordWithSpentEmit(
        _ handler: PlatformWalletPersistenceHandler,
        txid: Data,
        context: UInt32,
        inputOutpoint: (txid: Data, vout: UInt32),
        includeSpentEmit: Bool = true
    ) {
        let name = strdup("Standard { index: 0 }")
        defer { free(name) }

        var input = OutPointFFI()
        Swift.withUnsafeMutableBytes(of: &input.txid) { dst in
            inputOutpoint.txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        input.vout = inputOutpoint.vout

        var record = TransactionRecordFFI()
        Swift.withUnsafeMutableBytes(of: &record.txid) { dst in
            txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        record.context = context
        record.block_height = 0

        var spent = SpentOutPointFFI()
        spent.outpoint = input
        Swift.withUnsafeMutableBytes(of: &spent.spending_txid) { dst in
            txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }

        handler.beginChangeset(walletId: walletId)
        withUnsafeMutablePointer(to: &input) { inputPtr in
            record.input_outpoints = inputPtr
            record.input_outpoints_count = 1
            withUnsafeMutablePointer(to: &record) { recordPtr in
                withUnsafeMutablePointer(to: &spent) { spentPtr in
                    var account = AccountChangeSetFFI()
                    account.account_type_name = name
                    account.transactions = recordPtr
                    account.transactions_count = 1
                    if includeSpentEmit {
                        account.utxos_spent = spentPtr
                        account.utxos_spent_count = 1
                    }
                    withUnsafeMutablePointer(to: &account) { accountPtr in
                        var cs = WalletChangeSetFFI()
                        cs.accounts = accountPtr
                        cs.accounts_count = 1
                        withUnsafePointer(to: &cs) { csPtr in
                            handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                        }
                    }
                }
            }
        }
        _ = handler.endChangeset(walletId: walletId, success: true)
    }

    /// Multi-input record delivery with no spent emit — the shape a
    /// wallet-relevant loser takes when its inputs were never classified
    /// against live UTXOs (`input_outpoints` carries every raw input either
    /// way).
    private func deliverRecord(
        _ handler: PlatformWalletPersistenceHandler,
        walletId: Data? = nil,
        txid: Data,
        context: UInt32,
        inputOutpoints: [(txid: Data, vout: UInt32)]
    ) {
        let walletId = walletId ?? self.walletId
        let name = strdup("Standard { index: 0 }")
        defer { free(name) }

        var inputs: [OutPointFFI] = inputOutpoints.map { outpoint in
            var input = OutPointFFI()
            Swift.withUnsafeMutableBytes(of: &input.txid) { dst in
                outpoint.txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            input.vout = outpoint.vout
            return input
        }

        var record = TransactionRecordFFI()
        Swift.withUnsafeMutableBytes(of: &record.txid) { dst in
            txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        record.context = context
        record.block_height = 0

        handler.beginChangeset(walletId: walletId)
        inputs.withUnsafeMutableBufferPointer { inputsPtr in
            record.input_outpoints = inputsPtr.baseAddress
            record.input_outpoints_count = UInt(inputsPtr.count)
            withUnsafeMutablePointer(to: &record) { recordPtr in
                var account = AccountChangeSetFFI()
                account.account_type_name = name
                account.transactions = recordPtr
                account.transactions_count = 1
                withUnsafeMutablePointer(to: &account) { accountPtr in
                    var cs = WalletChangeSetFFI()
                    cs.accounts = accountPtr
                    cs.accounts_count = 1
                    withUnsafePointer(to: &cs) { csPtr in
                        handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                    }
                }
            }
        }
        _ = handler.endChangeset(walletId: walletId, success: true)
    }

    /// The pruned-finalized-release defect, on this store's terms: an
    /// InstantSend-locked spender F — settled under DIP-10, and one upstream
    /// no longer sees after a restart — is linked to a coin a later loser L
    /// reuses (alongside an attacker-owned input) while paying this wallet,
    /// so L's sweep names F's coin wrongly in the released set. F's row and
    /// its `spendingTransaction` link survive HERE: the settled-link guard
    /// keeps L's record pass from stealing the attribution, and the release
    /// veto (`releaseIsVetoed`) refuses the release the link contradicts,
    /// while the coin only L claimed still comes free in the same batch.
    ///
    /// F is seeded IS-locked (context 1) with `isSpent == false` — an
    /// unmined spender leaves the flag down — so the first assertion is
    /// carried by the guard alone: a mempool arrival against a linked
    /// spender that is merely flagged spent was already refused by the
    /// pre-existing `isSpent` branch, which this fixture deliberately does
    /// not exercise.
    func testAReleaseNamingACoinASettledSpenderStillClaimsIsRefused() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let finalizedTxid = Data(repeating: 0x46, count: 32)
        let attackerTxid = Data(repeating: 0x47, count: 32)

        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 200_000
        )
        // F: the IS-locked spender of the settled coin — upstream holds no
        // history for it after a restart; this store keeps the row and the
        // link.
        let finalized = PersistentTransaction(
            txid: finalizedTxid,
            transactionData: serializedTransaction(inputs: [(txid: fundingTxid, vout: 0)]),
            context: 1,
            blockHeight: 0,
            netAmount: -100_000
        )
        context.insert(funding)
        context.insert(finalized)

        let settledCoin = PersistentTxo(
            transaction: funding,
            vout: 0,
            amount: 100_000,
            address: "yFundAddr",
            height: 100
        )
        settledCoin.walletId = walletId
        settledCoin.isSpent = false
        settledCoin.spendingTransaction = finalized
        context.insert(settledCoin)

        let losersOwnCoin = PersistentTxo(
            transaction: funding,
            vout: 1,
            amount: 100_000,
            address: "yFundAddr",
            height: 100
        )
        losersOwnCoin.walletId = walletId
        context.insert(losersOwnCoin)
        try context.save()

        // L: arrives after F's pruning — pays this wallet, reuses F's input
        // alongside the attacker's and one coin of its own. Its record pass
        // must NOT steal F's link.
        deliverRecord(
            handler,
            txid: sweptTxid,
            context: 0,
            inputOutpoints: [
                (txid: fundingTxid, vout: 0),
                (txid: attackerTxid, vout: 0),
                (txid: fundingTxid, vout: 1),
            ]
        )
        XCTAssertEqual(
            try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).spendingTransaction?.txid,
            finalizedTxid,
            "a settled spender's link is not stolen by a conflicting record"
        )
        XCTAssertEqual(
            try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1)).spendingTransaction?.txid,
            sweptTxid,
            "the loser's own coin links normally"
        )

        // W (final) beats L on the attacker input alone. Upstream's release
        // set — computed from live records that no longer include F —
        // wrongly names F's coin alongside the loser's own.
        sweep(handler, [Batch(
            losers: [sweptTxid],
            winner: winnerTxid,
            winnerMinedHeight: 400,
            released: [(txid: fundingTxid, vout: 0), (txid: fundingTxid, vout: 1)]
        )])

        let settled = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(
            settled.isSpent,
            "a released coin a settled stored spender still claims is held spent"
        )
        XCTAssertEqual(settled.spendingTransaction?.txid, finalizedTxid)
        let freed = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertFalse(freed.isSpent, "a coin only the swept loser claimed must come free")
        XCTAssertNil(freed.spendingTransaction)
        XCTAssertNil(freed.supersededByTxid)
    }

    /// The case `settledSpenderLinkIsKept` exists for, which the mempool
    /// variant above does not reach: a plain in-block arrival (context 2)
    /// against a spender that is only IS-locked (context 1, unmined). Under
    /// DIP-10 the lock already settled the input, so an in-block
    /// double-spend of it is the losing side of a conflict, not newer
    /// evidence — the link stays with F. The one sanctioned takeover is
    /// chainlock-over-IS-lock, pinned in the second half.
    func testAnInBlockArrivalDoesNotTakeTheLinkFromAnInstantSendLockedSpender() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let finalizedTxid = Data(repeating: 0x46, count: 32)
        let chainlockedTxid = Data(repeating: 0x48, count: 32)

        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 200_000
        )
        let finalized = PersistentTransaction(
            txid: finalizedTxid,
            transactionData: serializedTransaction(inputs: [(txid: fundingTxid, vout: 0)]),
            context: 1,
            blockHeight: 0,
            netAmount: -100_000
        )
        context.insert(funding)
        context.insert(finalized)
        let settledCoin = PersistentTxo(
            transaction: funding,
            vout: 0,
            amount: 100_000,
            address: "yFundAddr",
            height: 100
        )
        settledCoin.walletId = walletId
        settledCoin.isSpent = false
        settledCoin.spendingTransaction = finalized
        context.insert(settledCoin)
        try context.save()

        // L arrives IN A BLOCK, spending the coin F holds under its lock.
        deliverRecord(
            handler,
            txid: sweptTxid,
            context: 2,
            inputOutpoints: [(txid: fundingTxid, vout: 0)]
        )
        let held = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertEqual(
            held.spendingTransaction?.txid,
            finalizedTxid,
            "an in-block arrival does not take the link from an IS-locked spender"
        )
        XCTAssertTrue(held.isSpent, "but the coin is spent either way — the flag never lowers")

        // L is swept (its block lost to the lock) with the coin in the
        // release set upstream computed from live records that no longer
        // include F: the surviving link vetoes the release.
        sweep(handler, [Batch(
            losers: [sweptTxid],
            winner: winnerTxid,
            winnerMinedHeight: 400,
            released: [(txid: fundingTxid, vout: 0)]
        )])
        let afterSweep = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(afterSweep.isSpent, "the coin an IS-locked spender consumed stays spent")
        XCTAssertEqual(afterSweep.spendingTransaction?.txid, finalizedTxid)

        // A chainlocked arrival is the one thing that outranks the lock.
        deliverRecord(
            handler,
            txid: chainlockedTxid,
            context: 3,
            inputOutpoints: [(txid: fundingTxid, vout: 0)]
        )
        XCTAssertEqual(
            try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).spendingTransaction?.txid,
            chainlockedTxid,
            "chainlock-over-IS-lock is the sanctioned takeover"
        )
    }

    /// The backstop for rows written before holds named their winner: a
    /// coin held spent with neither a spender nor a `supersededByTxid`
    /// stamp has nothing durable behind it, so the wallet re-delivering it
    /// as a UTXO — the authority on what it holds — still lifts the mark.
    /// Every hold written today is stamped; this pins the migration path
    /// for the ones already on disk.
    func testAPreStampHoldStillFreesOnRedelivery() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 40_000
        )
        context.insert(funding)
        let coinB = PersistentTxo(
            transaction: funding,
            vout: 1,
            amount: 40_000,
            address: "yFundAddr",
            height: 100
        )
        coinB.walletId = walletId
        coinB.isSpent = true
        context.insert(coinB)
        try context.save()

        redeliverCoinB(handler)

        let freed = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertFalse(freed.isSpent, "a hold with nothing durable behind it frees on re-delivery")
        XCTAssertNil(freed.spendingTransaction)
    }

    /// Hand coin B back through the ordinary account changeset, the way a
    /// rescan that re-finds the funding transaction does.
    private func redeliverCoinB(_ handler: PlatformWalletPersistenceHandler) {
        let name = strdup("Standard { index: 0 }")
        let address = strdup("yFundAddr")
        defer {
            free(name)
            free(address)
        }

        var utxo = UtxoEntryFFI()
        Swift.withUnsafeMutableBytes(of: &utxo.outpoint.txid) { dst in
            fundingTxid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        utxo.outpoint.vout = 1
        utxo.amount = 40_000
        utxo.address = address
        utxo.height = 100
        utxo.is_confirmed = true

        handler.beginChangeset(walletId: walletId)
        withUnsafeMutablePointer(to: &utxo) { utxoPtr in
            var account = AccountChangeSetFFI()
            account.account_type_name = name
            account.utxos_added = utxoPtr
            account.utxos_added_count = 1
            withUnsafeMutablePointer(to: &account) { accountPtr in
                var cs = WalletChangeSetFFI()
                cs.accounts = accountPtr
                cs.accounts_count = 1
                withUnsafePointer(to: &cs) { csPtr in
                    handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                }
            }
        }
        _ = handler.endChangeset(walletId: walletId, success: true)
    }

    /// Two sweeps in one round, the later disagreeing with the earlier.
    ///
    /// The first frees coin B; a second transaction spends it; the second
    /// sweep removes that spender and frees nothing, because its own winner
    /// took B. The later answer is the true one — and it only sticks because
    /// the batches are applied in sequence. Folding their release sets would
    /// leave the first "B is free" outliving the last "B is spent".
    func testALaterSweepKeepingACoinSpentOverridesAnEarlierRelease() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)

        // A second transaction takes coin B after the first sweep freed it.
        let secondLoser = Data(repeating: 0x55, count: 32)
        let context = ModelContext(container)
        let reclaimer = loserRow(
            txid: secondLoser,
            spending: [(txid: fundingTxid, vout: 1)],
            netAmount: -40_000
        )
        context.insert(reclaimer)
        let coinB = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 1)
        let descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.outpoint == coinB }
        )
        let row = try XCTUnwrap(try context.fetch(descriptor).first)
        row.spendingTransaction = reclaimer
        try context.save()

        sweep(handler, [
            Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 1)]),
            // Its winner consumed B, so this batch frees nothing.
            Batch(losers: [secondLoser], winner: Data(repeating: 0x56, count: 32), winnerMinedHeight: 400),
        ])

        let contested = txo(container, txid: fundingTxid, vout: 1)
        XCTAssertNotNil(contested)
        XCTAssertTrue(
            contested!.isSpent,
            "the later sweep kept the coin spent, so it must not come back"
        )
    }

    /// Seed the review finding's exact shape: one loser transaction shared
    /// by two wallets, spending a coin from each. `walletA` owns P, `walletB`
    /// owns Q; neither wallet's `PersistentTransaction` row for the winner is
    /// ever created here, matching the "winner can pay only outside
    /// addresses" case the released set exists to handle. The two coins live
    /// in the same funding transaction only for setup convenience — nothing
    /// about the fix depends on that; what makes `loser` shared is that its
    /// `row.inputs` spans two different owning wallets.
    private func seedSharedLoserAcrossTwoWallets(
        in container: ModelContainer,
        walletA: Data,
        walletB: Data,
        loserTxid: Data
    ) throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletA, network: .testnet))
        context.insert(PersistentWallet(walletId: walletB, network: .testnet))

        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 140_000
        )
        context.insert(funding)

        let loser = loserRow(
            txid: loserTxid,
            spending: [(txid: fundingTxid, vout: 0), (txid: fundingTxid, vout: 1)],
            netAmount: -140_000
        )
        context.insert(loser)

        // P — wallet A's coin, claimed only by the shared loser.
        let coinP = PersistentTxo(
            transaction: funding, vout: 0, amount: 100_000, address: "yWalletA", height: 100
        )
        coinP.walletId = walletA
        coinP.spendingTransaction = loser
        context.insert(coinP)

        // Q — wallet B's coin, also claimed only by the shared loser.
        let coinQ = PersistentTxo(
            transaction: funding, vout: 1, amount: 40_000, address: "yWalletB", height: 100
        )
        coinQ.walletId = walletB
        coinQ.spendingTransaction = loser
        context.insert(coinQ)

        try context.save()
    }

    /// The BLOCKING finding's exact shape, built on top of
    /// `seedSharedLoserAcrossTwoWallets`: the shared loser also created an
    /// output of its own — phantom money, since a transaction that never
    /// confirms funded nothing — and was `involvedAccounts`-linked to an
    /// account under `walletA` from back when it was still a live candidate
    /// (the ordinary `upsertTransaction` path does this before a later round
    /// ever learns the tx lost a double-spend). That link is what makes this
    /// fixture actually exercise the fix: were the row to survive,
    /// `walletOwnsTransaction` would find `walletA` through
    /// `involvedAccounts` alone, regardless of what happens to P.
    private func seedSharedLoserWithOutputAndInvolvedAccount(
        in container: ModelContainer,
        walletA: Data,
        walletB: Data,
        loserTxid: Data
    ) throws {
        try seedSharedLoserAcrossTwoWallets(
            in: container, walletA: walletA, walletB: walletB, loserTxid: loserTxid
        )
        let context = ModelContext(container)
        let walletRecord = try XCTUnwrap(
            try context.fetch(
                FetchDescriptor<PersistentWallet>(predicate: #Predicate { $0.walletId == walletA })
            ).first
        )
        let account = PersistentAccount(
            wallet: walletRecord, accountType: 0, accountIndex: 0, accountTypeName: "Standard"
        )
        context.insert(account)

        let loserDescriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == loserTxid }
        )
        let loser = try XCTUnwrap(try context.fetch(loserDescriptor).first)
        loser.involvedAccounts.append(account)

        let phantomChange = PersistentTxo(
            transaction: loser, vout: 2, amount: 60_000, address: "yLoserChange", height: 0
        )
        phantomChange.walletId = walletA
        context.insert(phantomChange)

        try context.save()
    }

    /// The hold is global, the release is per wallet — order 1: wallet B's
    /// callback, the one that releases nothing, runs first. It is the first
    /// callback to see the sweep, so it holds EVERY wallet's coins the
    /// loser claimed (P is wallet A's, and is held all the same: a released
    /// set is only ever true of the wallet that computed it, and B's says
    /// nothing about P) and deletes the shared row outright. Wallet A's
    /// later callback finds no row and still applies its release by
    /// outpoint, freeing P; B's hold on Q is untouched by it.
    func testSharedLoserAppliesBothWalletsReleaseSetsRegardlessOfOrder_BThenA() throws {
        let (handler, container) = try makeHandler()
        let loserTxid = Data(repeating: 0x81, count: 32)
        let winner = Data(repeating: 0x82, count: 32)
        let walletB = Data(repeating: 0x02, count: 32)
        try seedSharedLoserAcrossTwoWallets(
            in: container, walletA: walletId, walletB: walletB, loserTxid: loserTxid
        )

        // Wallet B first: its own released set names nothing.
        sweep(handler, [Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400)], walletId: walletB)

        XCTAssertNil(
            transaction(container, txid: loserTxid),
            "the first callback to see the sweep deletes the row — hold before delete"
        )
        let heldP = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(heldP.isSpent, "wallet A's coin is held by wallet B's callback — the hold is global")
        XCTAssertEqual(heldP.supersededByTxid, winner)
        XCTAssertNil(heldP.spendingTransaction, "the link to the dead loser is gone")

        // Wallet A second: its own released set names P.
        sweep(handler, [
            Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 0)])
        ], walletId: walletId)

        let p = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(p.isSpent, "wallet A's own release frees its own coin, row or no row")
        XCTAssertNil(p.supersededByTxid)
        XCTAssertNil(p.spendingTransaction)

        let q = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(q.isSpent, "wallet B's hold on Q must survive wallet A's callback")
        XCTAssertEqual(q.supersededByTxid, winner)
        XCTAssertNil(q.spendingTransaction)
    }

    /// Order 2: wallet A — the one that releases P — runs first. It frees
    /// its own P, holds wallet B's Q (global hold) and deletes the row;
    /// wallet B's callback then finds no row and, releasing nothing, leaves
    /// its hold on Q as it is. Order-independent: the exact same end state
    /// as the B-then-A ordering above.
    func testSharedLoserAppliesBothWalletsReleaseSetsRegardlessOfOrder_AThenB() throws {
        let (handler, container) = try makeHandler()
        let loserTxid = Data(repeating: 0x91, count: 32)
        let winner = Data(repeating: 0x92, count: 32)
        let walletB = Data(repeating: 0x02, count: 32)
        try seedSharedLoserAcrossTwoWallets(
            in: container, walletA: walletId, walletB: walletB, loserTxid: loserTxid
        )

        // Wallet A first: releases P.
        sweep(handler, [
            Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 0)])
        ], walletId: walletId)

        XCTAssertNil(
            transaction(container, txid: loserTxid),
            "the first callback to see the sweep deletes the row — hold before delete"
        )
        let heldQ = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(heldQ.isSpent, "wallet B's coin is held by wallet A's callback — the hold is global")
        XCTAssertEqual(heldQ.supersededByTxid, winner)
        XCTAssertNil(heldQ.spendingTransaction)

        // Wallet B second: releases nothing.
        sweep(handler, [Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400)], walletId: walletB)

        let p = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(p.isSpent, "wallet A's earlier release must survive wallet B's callback")
        XCTAssertNil(p.supersededByTxid)
        XCTAssertNil(p.spendingTransaction)

        let q = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(q.isSpent, "wallet B's coin stays held — its own callback released nothing")
        XCTAssertEqual(q.supersededByTxid, winner)
        XCTAssertNil(q.spendingTransaction)
    }

    /// The BLOCKING review finding: a shared loser's own output, and its
    /// reachability through `walletCoreTxids`, must not survive across a
    /// restart when only ONE wallet's callback ever commits and the other's
    /// never arrives at all — a crash, a rejection, or simply never coming.
    ///
    /// `commit_batch` calls `store()` once per wallet and each commits
    /// independently. A row held back for another wallet's still-pending
    /// callback is a row that survives forever when that callback never
    /// comes — with its phantom output and its `involvedAccounts` link to
    /// wallet A fully live, so `walletCoreTxids` would hand the dead
    /// transaction back to wallet A as its own after every future restart.
    /// So the first callback to see the sweep deletes the row and its
    /// outputs for every wallet, after holding every wallet's inputs; a
    /// surviving swept row is a shape that no longer exists, and no reader
    /// needs a guard against it.
    ///
    /// Only wallet B's callback ever runs here, and it releases nothing —
    /// the worst case for the old deferred delete.
    func testSharedLoserOutputAndEnumerationAreExcludedAfterOnlyOneWalletsCallbackCommits() throws {
        let storeURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("swept-shared-durability-\(UUID().uuidString).store")
        defer { try? FileManager.default.removeItem(at: storeURL) }
        let loserTxid = Data(repeating: 0xA1, count: 32)
        let winner = Data(repeating: 0xA2, count: 32)
        let walletB = Data(repeating: 0x02, count: 32)

        do {
            let (handler, container) = try makeHandler(url: storeURL)
            try seedSharedLoserWithOutputAndInvolvedAccount(
                in: container, walletA: walletId, walletB: walletB, loserTxid: loserTxid
            )

            // Only wallet B's callback ever runs, and it releases nothing —
            // wallet A's own callback (which would release P) never arrives
            // in this test at all.
            sweep(handler, [Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400)], walletId: walletB)

            XCTAssertNil(
                transaction(container, txid: loserTxid),
                "the row is deleted by whichever wallet's callback sees the sweep first"
            )
            XCTAssertNil(
                txo(container, txid: loserTxid, vout: 2),
                "the loser's own output must not survive even a single committed callback, "
                    + "regardless of which wallet's callback that was"
            )
            let p = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
            XCTAssertTrue(p.isSpent, "wallet A's coin is held for the winner until A's own release")
            XCTAssertEqual(p.supersededByTxid, winner)
        }

        // Restart: a fresh handler/container over the same file. Wallet A's
        // callback never happens in this test, simulating a crash or a
        // rejection that stops it from ever arriving — the exact scenario
        // the finding describes.
        let (handler, container) = try makeHandler(url: storeURL)

        XCTAssertNil(
            txo(container, txid: loserTxid, vout: 2),
            "the phantom output must not resurrect across a restart"
        )
        XCTAssertNil(transaction(container, txid: loserTxid), "nor the row")
        let (txidsA, erroredA) = handler.walletCoreTxids(walletId: walletId)
        XCTAssertFalse(erroredA)
        XCTAssertFalse(
            txidsA.contains { $0.txid == loserTxid },
            "wallet A must not be able to enumerate the swept loser as its own transaction "
                + "after a restart, even though its own callback never ran"
        )
        let p = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(p.isSpent, "and the hold on its coin is durable")
        XCTAssertEqual(p.supersededByTxid, winner)
    }

    /// Cross-round reinstatement. The sweep and its reinstating record land
    /// in two SEPARATE `persistWalletChangeset` rounds: round 1 deletes the
    /// shared row and holds wallet A's coin for the winner; round 2's record
    /// — upstream's newer word, per `CoreChangeSet::merge`'s documented
    /// IS-lock-precedence sequence (swept by an IS-locked conflict, then
    /// returns chainlocked and sweeps that conflict in turn) — arrives like
    /// any freshly detected transaction, inserts a fresh row, re-adopts the
    /// held coin's link and brings its output back through the
    /// `utxos_added` riding alongside. Verified across a restart: the
    /// reinstatement has to be durable, not merely visible in the context
    /// that just applied it.
    func testAReinstatingRecordInALaterRoundRevivesASweptTransactionAndItsOutputs() throws {
        let storeURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("swept-reinstatement-\(UUID().uuidString).store")
        defer { try? FileManager.default.removeItem(at: storeURL) }
        let loserTxid = Data(repeating: 0xB1, count: 32)
        let winner = Data(repeating: 0xB2, count: 32)
        let walletB = Data(repeating: 0x02, count: 32)

        do {
            let (handler, container) = try makeHandler(url: storeURL)
            try seedSharedLoserWithOutputAndInvolvedAccount(
                in: container, walletA: walletId, walletB: walletB, loserTxid: loserTxid
            )

            // Round 1: only wallet B's own sweep callback runs, releasing
            // nothing. The row and its phantom output go, and wallet A's
            // coin P is held for the winner.
            sweep(handler, [Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400)], walletId: walletB)

            XCTAssertNil(transaction(container, txid: loserTxid), "sanity: the row is gone after round 1")
            XCTAssertNil(
                txo(container, txid: loserTxid, vout: 2),
                "sanity: the loser's own output is gone after round 1"
            )
            XCTAssertTrue(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)

            // Round 2, a SEPARATE callback (not coalesced with round 1's
            // sweep — the cross-round shape the merge-level fix in
            // `CoreChangeSet::merge` cannot reach): the wallet returns
            // chainlocked and sweeps the erstwhile winner in turn. Arrives
            // here exactly like any freshly-detected transaction would —
            // nothing marks it as "the reinstating one" — with its own
            // output riding along in the same round the way a transaction's
            // outputs ordinarily do.
            deliverReinstatingRecord(
                handler,
                walletId: walletId,
                txid: loserTxid,
                context: 3, // inChainLockedBlock
                blockHeight: 200,
                inputOutpoints: [(txid: fundingTxid, vout: 0)],
                outputVout: 2,
                outputAmount: 60_000,
                outputAddress: "yLoserChange"
            )

            let reinstated = try XCTUnwrap(
                transaction(container, txid: loserTxid),
                "the reinstating record must not be discarded"
            )
            XCTAssertEqual(reinstated.blockHeight, 200)

            let revivedOutput = try XCTUnwrap(
                txo(container, txid: loserTxid, vout: 2),
                "the reinstated transaction's own output must come back"
            )
            XCTAssertEqual(revivedOutput.amount, 60_000)

            let p = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
            XCTAssertTrue(p.isSpent, "wallet A reclaims its input once its own record is live again")
            XCTAssertEqual(p.spendingTransaction?.txid, loserTxid)

            let (txidsA, erroredA) = handler.walletCoreTxids(walletId: walletId)
            XCTAssertFalse(erroredA)
            XCTAssertTrue(
                txidsA.contains { $0.txid == loserTxid },
                "wallet A must be able to enumerate the reinstated transaction as its own again"
            )
        }

        // Restart: a fresh handler/container over the same file. The
        // reinstatement has to be durable, not just visible to the context
        // that applied it.
        let (handler, container) = try makeHandler(url: storeURL)

        XCTAssertNotNil(transaction(container, txid: loserTxid), "the reinstatement must survive a restart")
        XCTAssertNotNil(
            txo(container, txid: loserTxid, vout: 2),
            "the revived output must survive a restart"
        )
        let p = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(p.isSpent, "the reclaimed input must survive a restart")
        XCTAssertEqual(p.spendingTransaction?.txid, loserTxid)

        let (txidsA, erroredA) = handler.walletCoreTxids(walletId: walletId)
        XCTAssertFalse(erroredA)
        XCTAssertTrue(
            txidsA.contains { $0.txid == loserTxid },
            "the reinstated transaction must still enumerate as wallet A's own after a restart"
        )
    }

    /// A failed wallet lookup must fail the round, not read as "no such
    /// wallet".
    ///
    /// `try?` collapsed the two: a thrown SwiftData fetch returned success
    /// without applying the sweep, Rust discarded the subtractive event, and
    /// a later round could then persist a height beyond a removal that never
    /// landed. Driving the real failure is awkward, so this pins the
    /// distinction that makes it impossible — a wallet that genuinely is not
    /// there is still a successful no-op.
    func testAMissingWalletIsASuccessfulNoOp() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)

        // Delete the wallet row, leaving the fetch to succeed and find
        // nothing — the branch that must stay a success.
        let context = ModelContext(container)
        let walletId = self.walletId
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        for row in try context.fetch(descriptor) {
            context.delete(row)
        }
        try context.save()

        let applied = sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])

        XCTAssertTrue(applied, "a stale post-deletion callback is not a failure")
        XCTAssertNotNil(
            transaction(container, txid: sweptTxid),
            "and it must not have applied anything either"
        )
    }

    /// Companion to `testAMissingWalletIsASuccessfulNoOp` above, which its
    /// own doc admits does not distinguish the fix from the old `try?`
    /// behavior — a successful empty fetch reads identically either way.
    /// This drives a genuinely THROWING fetch instead, using a real seam
    /// rather than a mock: a file-backed store (so the container's SQLite
    /// connection is live and long-lived, unlike the in-memory variant) is
    /// truncated on disk, out from under that open connection, between
    /// seeding and the sweep. `fetchWalletRecord`'s `context.fetch` then has
    /// to perform real I/O against a file that is no longer a valid SQLite
    /// database, which is the only way found to make it throw without
    /// adding a test-only injection point to production code.
    func testAThrowingWalletLookupFailsTheRound() throws {
        let storeURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("swept-throwing-lookup-\(UUID().uuidString).store")
        defer { try? FileManager.default.removeItem(at: storeURL) }

        let (handler, _) = try makeHandler(url: storeURL)

        // Corrupt the on-disk store out from under the still-open container
        // BEFORE any context — including a seed helper's — reads or writes
        // through it: SwiftData's row cache is scoped to the persistent
        // store coordinator, not to any one `ModelContext`, so a row
        // touched by a throwaway seeding context would still be served from
        // that shared cache here and never reach disk at all. With nothing
        // cached yet, `fetchWalletRecord`'s fetch is the first real read
        // this store ever performs, and it hits the truncated file — well
        // short of a valid SQLite header — directly.
        let handle = try FileHandle(forWritingTo: storeURL)
        handle.truncateFile(atOffset: 16)
        try handle.close()

        let applied = sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])

        XCTAssertFalse(applied, "a genuinely failed wallet lookup must fail the round")
    }

    /// Two wallets, each holding an unresolved pending input on the same
    /// shared loser, and a winner that took neither — so upstream names
    /// both coins released in BOTH wallets' views (a released set is
    /// computed from the loser's and the winner's inputs, the same for
    /// every wallet). A release only ever touches the releasing wallet's
    /// own rows: the first callback (A's) deletes its own released row,
    /// holds B's — A's released set is not the authority on B's claim —
    /// and deletes the loser; B's callback then applies its own release by
    /// outpoint against the tombstone. Nothing is left behind: no row, no
    /// pending entry of either wallet's.
    func testEachWalletsReleaseReachesItsOwnPendingRowOnASharedLoser() throws {
        let (handler, container) = try makeHandler()
        let walletB = Data(repeating: 0x02, count: 32)
        try seedSharedLoserAcrossTwoWallets(
            in: container, walletA: walletId, walletB: walletB, loserTxid: sweptTxid
        )

        // Each wallet has one pending input on the loser, and each will be
        // released by its own wallet's sweep. The loser's bytes name both.
        let context = ModelContext(container)
        let loserTxid = sweptTxid
        var descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == loserTxid }
        )
        descriptor.fetchLimit = 1
        let loser = try XCTUnwrap(try context.fetch(descriptor).first)
        loser.transactionData = serializedTransaction(inputs: [
            (txid: fundingTxid, vout: 0), (txid: fundingTxid, vout: 1),
            (txid: fundingTxid, vout: 8), (txid: fundingTxid, vout: 9),
        ])
        let pendingA = PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 8),
            inputIndex: 0,
            spendingTxid: loserTxid,
            spendingTransaction: loser,
            walletId: walletId
        )
        let pendingB = PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 9),
            inputIndex: 1,
            spendingTxid: loserTxid,
            spendingTransaction: loser,
            walletId: walletB
        )
        context.insert(pendingA)
        context.insert(pendingB)
        try context.save()

        let releasedInBothViews = [(txid: fundingTxid, vout: UInt32(8)), (txid: fundingTxid, vout: UInt32(9))]
        sweep(handler, [
            Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400, released: releasedInBothViews)
        ])
        XCTAssertNil(transaction(container, txid: sweptTxid), "the first callback deletes the row")
        XCTAssertTrue(
            try pendingRows(container, spentTxid: fundingTxid, vout: 8).isEmpty,
            "wallet A's released row is deleted outright — never a released tombstone"
        )
        let heldB = try XCTUnwrap(
            try pendingRows(container, spentTxid: fundingTxid, vout: 9).first,
            "wallet B's row is held for the winner by wallet A's callback"
        )
        XCTAssertTrue(heldB.isSweptTombstone)
        XCTAssertEqual(heldB.spendingTxid, winnerTxid)
        XCTAssertEqual(heldB.walletId, walletB)

        sweep(
            handler,
            [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400, released: releasedInBothViews)],
            walletId: walletB
        )
        XCTAssertTrue(
            try pendingRows(container, spentTxid: fundingTxid, vout: 9).isEmpty,
            "wallet B's own release reaches its tombstone with the row already gone"
        )
    }

    /// A txid the store has never seen is not an error: sweeps are
    /// idempotent, and a round can name a transaction this mirror never
    /// recorded in the first place.
    func testSweepingAnUnknownTransactionIsANoOp() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)

        let applied = sweep(handler, [
            Batch(losers: [Data(repeating: 0x99, count: 32)], winner: winnerTxid, winnerMinedHeight: 400)
        ])

        XCTAssertTrue(applied, "an absent row is a successful no-op, not a failed round")
        XCTAssertNotNil(transaction(container, txid: sweptTxid))
        XCTAssertNotNil(transaction(container, txid: fundingTxid))
    }

    /// The loser can be persisted before its own funding output ever is —
    /// `upsertTransaction` parks a spend like that as a `PersistentPendingInput`
    /// rather than a `PersistentTxo` update (see `resolveInputOutpoint`).
    /// When the sweep holds that input (it's not in `released`), there is no
    /// `PersistentTxo` row to mark — the only record of the claim is the
    /// pending row, which cascades away with the loser it names unless
    /// `applySweptTransaction` rescues it first. This is the regression the
    /// review finding described: seed the pending spend, sweep it, restart
    /// the store, and only then let the funding UTXO arrive. The coin must
    /// come back spent, attributed to the winner, not as a fresh unspent row.
    func testSpendBeforeFundingSweptThenRestartedThenFundedStaysSpent() throws {
        let storeURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("swept-pending-input-\(UUID().uuidString).store")
        defer { try? FileManager.default.removeItem(at: storeURL) }

        do {
            let (handler, container) = try makeHandler(url: storeURL)
            let context = ModelContext(container)
            context.insert(PersistentWallet(walletId: walletId, network: .testnet))
            let swept = loserRow(txid: sweptTxid, spending: [(txid: fundingTxid, vout: 0)])
            context.insert(swept)
            // What `resolveInputOutpoint` would have written: the funding
            // TXO for (fundingTxid, 0) has never been seen here.
            context.insert(PersistentPendingInput(
                outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
                inputIndex: 0,
                spendingTxid: sweptTxid,
                spendingTransaction: swept,
                walletId: walletId
            ))
            try context.save()
            XCTAssertNil(
                txo(container, txid: fundingTxid, vout: 0),
                "sanity: the funding TXO has not arrived yet"
            )

            sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])

            XCTAssertNil(transaction(container, txid: sweptTxid), "the loser is gone")
        }

        // Restart: a fresh persister loading the same on-disk store.
        let (handler, container) = try makeHandler(url: storeURL)
        deliverFundingUtxo(handler, vout: 0, amount: 100_000)

        let coin = try XCTUnwrap(
            txo(container, txid: fundingTxid, vout: 0),
            "the funding UTXO's own upsert must still create the row"
        )
        XCTAssertTrue(
            coin.isSpent,
            "the winner's claim must survive the loser's deletion, a restart, "
                + "and the funding UTXO's own arrival"
        )
        XCTAssertEqual(coin.supersededByTxid, winnerTxid)
    }

    /// Records precede sweeps within a round, so a wallet-relevant winner
    /// whose own funding side is ALSO unobserved stages an ordinary pending
    /// row for the same outpoint moments before the sweep repoints the
    /// loser's row into a tombstone — and the tombstone keeps the loser's
    /// original, older `createdAt`. A newest-wins pick over all rows once
    /// selected the winner's ordinary row, took the gated branch (`isSpent`
    /// stays false until the winner confirms — never, for an IS-locked
    /// unconfirmed winner), skipped the `supersededByTxid` stamp, and
    /// deleted every pending row including the tombstone: the durable hold
    /// evaporated and the consumed coin re-entered the restore set. The
    /// tombstone supplies the stamp regardless of age; the winner's own row
    /// supplies the link beside it.
    func testAWinnersOwnPendingRowDoesNotEvaporateTheSweepTombstone() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let outpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)

        // The doomed spend arrived before its funding output — parked as a
        // pending row, exactly what `resolveInputOutpoint` writes. Backdated
        // so the winner's row below is strictly newer, as it always is in
        // reality (the loser's record preceded the winner's by definition).
        let loser = loserRow(txid: sweptTxid, spending: [(txid: fundingTxid, vout: 0)])
        context.insert(loser)
        let losersClaim = PersistentPendingInput(
            outpoint: outpoint,
            inputIndex: 0,
            spendingTxid: sweptTxid,
            spendingTransaction: loser,
            walletId: walletId
        )
        losersClaim.createdAt = Date(timeIntervalSinceNow: -10)
        context.insert(losersClaim)

        // The winner's own record — IS-locked, still unconfirmed — lands in
        // the same round as the sweep, records first, and stages its own
        // ordinary pending row for the same still-unfunded outpoint.
        let winner = PersistentTransaction(
            txid: winnerTxid,
            transactionData: serializedTransaction(inputs: [(txid: fundingTxid, vout: 0)]),
            context: 1,
            blockHeight: 0,
            netAmount: -100_000
        )
        context.insert(winner)
        context.insert(PersistentPendingInput(
            outpoint: outpoint,
            inputIndex: 0,
            spendingTxid: winnerTxid,
            spendingTransaction: winner,
            walletId: walletId
        ))
        try context.save()

        sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])

        // Sanity: the coexisting pair this regression is about — the
        // winner's ordinary row plus the repointed tombstone.
        let pendingDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        let rows = try context.fetch(pendingDescriptor)
        XCTAssertEqual(rows.count, 2)
        XCTAssertEqual(rows.filter(\.isSweptTombstone).count, 1)

        deliverFundingUtxo(handler, vout: 0, amount: 100_000)

        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(
            coin.isSpent,
            "the sweep's hold must survive the winner's own coexisting pending row"
        )
        XCTAssertEqual(coin.supersededByTxid, winnerTxid)
        XCTAssertEqual(
            coin.spendingTransaction?.txid, winnerTxid,
            "and the winner's own row supplies the attribution the tombstone cannot"
        )
    }

    /// Chained-sweep continuation of `testSpendBeforeFundingSweptThenRestartedThenFundedStaysSpent`
    /// above: L spends P; W spends P and Q and sweeps L, holding P (still
    /// unfunded); X spends Q and sweeps W, this time releasing P. The
    /// tombstone `applySweptTransaction` wrote for P when L was swept
    /// already detached from `spendingTransaction`, so the second sweep of
    /// W cannot find it through `row.pendingInputs` the way the first sweep
    /// did — it can only be found by the scalar `spendingTxid` it now
    /// carries. This is the review finding: without that second lookup, the
    /// second sweep's release of P is silently dropped, and P's funding TXO
    /// resurrects the coin attributed to the wrong (already deleted)
    /// transaction instead of coming back spendable.
    func testChainedSweepBeforeFundingReleasesAnEarlierTombstoneOnASecondSweep() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let firstLoser = Data(repeating: 0x61, count: 32) // L
        let secondLoser = Data(repeating: 0x62, count: 32) // W
        let finalWinner = Data(repeating: 0x63, count: 32) // X

        let l = loserRow(txid: firstLoser, spending: [(txid: fundingTxid, vout: 0)])
        context.insert(l)
        // P (fundingTxid:0) has never been observed as a TXO — parked as a
        // pending input, the same as `testSpendBeforeFundingSweptThenRestartedThenFundedStaysSpent`.
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: firstLoser,
            spendingTransaction: l,
            walletId: walletId
        ))
        try context.save()

        // First sweep: W beats L, holding P (still unfunded).
        sweep(handler, [Batch(losers: [firstLoser], winner: secondLoser, winnerMinedHeight: 400)])

        let pOutpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)
        let tombstoneDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == pOutpoint }
        )
        let tombstone = try XCTUnwrap(try context.fetch(tombstoneDescriptor).first)
        XCTAssertTrue(tombstone.isSweptTombstone, "the first sweep must tombstone the pending row")
        XCTAssertEqual(tombstone.spendingTxid, secondLoser)
        XCTAssertNil(tombstone.spendingTransaction, "must have detached from the doomed loser's FK")

        // W's own row, plus a materialized claim on Q, needed for the
        // second sweep to find W at all — the same requirement any sweep of
        // a wallet-relevant loser has.
        let w = loserRow(
            txid: secondLoser,
            spending: [(txid: fundingTxid, vout: 0), (txid: Data(repeating: 0x65, count: 32), vout: 0)],
            netAmount: -90_000
        )
        context.insert(w)
        let qFunding = PersistentTransaction(
            txid: Data(repeating: 0x65, count: 32),
            transactionData: Data(repeating: 0x09, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 40_000
        )
        context.insert(qFunding)
        let coinQ = PersistentTxo(
            transaction: qFunding,
            vout: 0,
            amount: 40_000,
            address: "yFundAddr",
            height: 100
        )
        coinQ.walletId = walletId
        coinQ.spendingTransaction = w
        context.insert(coinQ)
        try context.save()

        // Second sweep: X beats W, this time releasing P.
        sweep(handler, [
            Batch(losers: [secondLoser], winner: finalWinner, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 0)])
        ])

        let survivingTombstones = try context.fetch(tombstoneDescriptor)
        XCTAssertTrue(
            survivingTombstones.isEmpty,
            "a released outpoint's tombstone must not survive a chained sweep"
        )

        deliverFundingUtxo(handler, vout: 0, amount: 50_000)

        let coin = try XCTUnwrap(
            txo(container, txid: fundingTxid, vout: 0),
            "the funding UTXO's own upsert must still create the row"
        )
        XCTAssertFalse(
            coin.isSpent,
            "the final sweep released this coin, so it must come back spendable even "
                + "though an earlier sweep in the chain had tombstoned it"
        )
        XCTAssertNil(coin.supersededByTxid)
    }

    /// The held (not released) half of the chained scenario above: the
    /// second sweep keeps P spent instead of releasing it, and the
    /// tombstone must end up attributed to the NEW winner rather than the
    /// intermediate one that no longer has a row.
    func testChainedSweepBeforeFundingRepointsAnEarlierTombstoneToTheNewWinner() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let firstLoser = Data(repeating: 0x71, count: 32) // L
        let secondLoser = Data(repeating: 0x72, count: 32) // W
        let finalWinner = Data(repeating: 0x73, count: 32) // X

        let l = loserRow(txid: firstLoser, spending: [(txid: fundingTxid, vout: 0)])
        context.insert(l)
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: firstLoser,
            spendingTransaction: l,
            walletId: walletId
        ))
        try context.save()

        // First sweep: W beats L, holding P.
        sweep(handler, [Batch(losers: [firstLoser], winner: secondLoser, winnerMinedHeight: 400)])

        // W's own row — this time claiming ONLY P, so the second sweep has
        // no other input to reason about.
        let w = loserRow(txid: secondLoser, spending: [(txid: fundingTxid, vout: 0)])
        context.insert(w)
        try context.save()

        // Second sweep: X beats W, still holding the same input.
        sweep(handler, [Batch(losers: [secondLoser], winner: finalWinner, winnerMinedHeight: 400)])

        let pOutpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)
        let tombstoneDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == pOutpoint }
        )
        let tombstone = try XCTUnwrap(try context.fetch(tombstoneDescriptor).first)
        XCTAssertTrue(tombstone.isSweptTombstone)
        XCTAssertEqual(
            tombstone.spendingTxid,
            finalWinner,
            "the tombstone must be repointed at the FINAL winner, not the intermediate "
                + "one the second sweep already removed"
        )

        deliverFundingUtxo(handler, vout: 0, amount: 50_000)

        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(
            coin.isSpent,
            "the final winner's claim must survive both sweeps and the funding UTXO's own arrival"
        )
        XCTAssertEqual(coin.supersededByTxid, finalWinner)
    }

    /// The multi-loser batch shape upstream's descendant closure always
    /// produces — parent P and child C removed together — which no fixture
    /// here ever exercised: C spends P:0, still unfunded, so the claim
    /// lives as a pending row. Upstream never releases a loser-funded
    /// outpoint, so without a co-swept check the sweep tombstones the
    /// claim to the winner — and P's chainlocked reinstatement then
    /// re-delivers P:0 straight into the tombstone-outranks drain:
    /// `isSpent = true`, `supersededByTxid = winner`, recovery clear
    /// refusing stamped holds. Permanently unspendable. A dead parent's
    /// output is nobody's coin; the claim must be deleted with the batch.
    func testABatchSweepingParentAndChildDeletesTheChildsClaimOnTheParentsOutput() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        // P is `fundingTxid` (so the redelivery helper reaches it) and its
        // record was never persisted — the weaker-preconditions shape. C's
        // claim on P:0 is parked as a pending row, exactly what
        // `resolveInputOutpoint` writes.
        let childTxid = Data(repeating: 0xB5, count: 32) // C
        let winner = Data(repeating: 0xB6, count: 32) // W
        let pOutpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)

        let c = loserRow(txid: childTxid, spending: [(txid: fundingTxid, vout: 0)], netAmount: -50_000)
        context.insert(c)
        context.insert(PersistentPendingInput(
            outpoint: pOutpoint,
            inputIndex: 0,
            spendingTxid: childTxid,
            spendingTransaction: c,
            walletId: walletId
        ))
        try context.save()

        // One batch removes both; upstream excludes P:0 from the released
        // set because its funder is itself a loser.
        sweep(handler, [Batch(losers: [fundingTxid, childTxid], winner: winner, winnerMinedHeight: 400)])

        let pendingDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == pOutpoint }
        )
        XCTAssertTrue(
            try context.fetch(pendingDescriptor).isEmpty,
            "a claim on a co-swept parent's output must be deleted, not tombstoned"
        )

        // The chainlocked return: P reinstated with its output re-delivered
        // must land spendable — nothing the batch left behind may hold it.
        deliverFundingUtxo(handler, vout: 0, amount: 50_000)

        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(
            coin.isSpent,
            "the reinstated parent's output must not be wedged by its dead child's claim"
        )
        XCTAssertNil(coin.supersededByTxid)
    }

    /// The whole chain inside ONE round: a single sweeps callback can carry
    /// two batches where the second sweeps the first's winner, so the
    /// tombstone the first batch just wrote — staged, unsaved, retargeted by
    /// nothing but in-memory mutation — must reach the second batch. Pins
    /// the once-per-round tombstone map being re-keyed in memory as batches
    /// run: a second store fetch would not see the re-point, and a
    /// store-side predicate on the mutable column would test the stale
    /// saved value and miss the row entirely.
    func testChainedSweepAcrossTwoBatchesInOneRoundReleasesTheFreshTombstone() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let firstLoser = Data(repeating: 0xA1, count: 32) // L
        let secondLoser = Data(repeating: 0xA2, count: 32) // W — batch 1's winner
        let finalWinner = Data(repeating: 0xA3, count: 32) // X

        let l = loserRow(txid: firstLoser, spending: [(txid: fundingTxid, vout: 0)], netAmount: -50_000)
        context.insert(l)
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: firstLoser,
            spendingTransaction: l,
            walletId: walletId
        ))
        try context.save()

        // One callback, two batches: W beats L holding the unfunded coin,
        // then X beats W and frees it.
        sweep(handler, [
            Batch(losers: [firstLoser], winner: secondLoser, winnerMinedHeight: 400),
            Batch(
                losers: [secondLoser],
                winner: finalWinner,
                winnerMinedHeight: 400,
                released: [(txid: fundingTxid, vout: 0)]
            ),
        ])

        let pOutpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)
        let pendingDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == pOutpoint }
        )
        XCTAssertTrue(
            try context.fetch(pendingDescriptor).isEmpty,
            "the second batch must find and release the tombstone the first batch just wrote"
        )

        deliverFundingUtxo(handler, vout: 0, amount: 50_000)
        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(coin.isSpent, "the released coin funds as spendable")
        XCTAssertNil(coin.supersededByTxid)
    }

    /// The funding-BEFORE-release ordering of the chained scenario above:
    /// the funding TXO arrives between the sweep that held the coin and the
    /// sweep that frees it, so the tombstone drains into
    /// `PersistentTxo.supersededByTxid` and the pending row is gone by the
    /// time the release runs. With the intermediate winner's own record on
    /// hand the drain links `spendingTransaction` too, so the release
    /// reaches the row through the winner's decoded inputs — and must clear
    /// the marker with the hold: a released coin keeping its dead winner's
    /// marker would read as a durable claim on every later channel.
    func testAReleasedCoinDropsItsDeadWinnersMarker() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let firstLoser = Data(repeating: 0x91, count: 32) // L
        let secondLoser = Data(repeating: 0x92, count: 32) // W
        let finalWinner = Data(repeating: 0x93, count: 32) // X

        let l = loserRow(txid: firstLoser, spending: [(txid: fundingTxid, vout: 0)], netAmount: -50_000)
        context.insert(l)
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: firstLoser,
            spendingTransaction: l,
            walletId: walletId
        ))
        try context.save()

        // First sweep: W beats L, holding the still-unfunded coin.
        sweep(handler, [Batch(losers: [firstLoser], winner: secondLoser, winnerMinedHeight: 400)])

        // W's own record lands before the funding TXO does — through the
        // record pass, which stages W's own ordinary claim row beside the
        // tombstone — so the drain below links `spendingTransaction` as
        // well as stamping the marker.
        deliverRecord(handler, txid: secondLoser, context: 0, inputOutpoints: [(txid: fundingTxid, vout: 0)])

        deliverFundingUtxo(handler, vout: 0, amount: 50_000)

        let stamped = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(stamped.isSpent, "sanity: the drained claim holds the coin")
        XCTAssertEqual(stamped.supersededByTxid, secondLoser)
        XCTAssertEqual(stamped.spendingTransaction?.txid, secondLoser, "sanity: linked through W's own row")

        // Second sweep: X beats W, and this time upstream frees the coin.
        sweep(handler, [
            Batch(losers: [secondLoser], winner: finalWinner, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 0)])
        ])

        let freed = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(freed.isSpent, "the released coin is spendable again")
        XCTAssertNil(freed.spendingTransaction)
        XCTAssertNil(
            freed.supersededByTxid,
            "the dead winner's marker goes with the hold it carried"
        )
    }

    /// The unreachable-claim variant of the same ordering: the claim
    /// drained into `PersistentTxo.supersededByTxid`, its pending row is
    /// gone, and the winner it names was NEVER recorded here — so when that
    /// winner is swept in turn there is no `row` to fetch, no `row.inputs`
    /// to walk, and no tombstone left for the scalar reconciliation to
    /// find. Only an outpoint-keyed release — the form Kotlin's
    /// `releaseByOutpoint` and SQLite's outpoint-matched UPDATE both
    /// implement — can reach the coin; without it the release is silently
    /// dropped and the coin stays spent forever.
    func testAReleaseReachesAClaimDrainedToTheTxoWhenTheWinnerWasNeverRecorded() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let firstLoser = Data(repeating: 0x94, count: 32) // L
        let unrecordedWinner = Data(repeating: 0x95, count: 32) // W — never a row here
        let finalWinner = Data(repeating: 0x96, count: 32) // X

        let l = loserRow(txid: firstLoser, spending: [(txid: fundingTxid, vout: 0)], netAmount: -50_000)
        context.insert(l)
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: firstLoser,
            spendingTransaction: l,
            walletId: walletId
        ))
        try context.save()

        // First sweep: W beats L, holding the still-unfunded coin.
        sweep(handler, [Batch(losers: [firstLoser], winner: unrecordedWinner, winnerMinedHeight: 400)])

        // The funding TXO arrives with W still unrecorded: the drain stamps
        // the marker but has no row to link.
        deliverFundingUtxo(handler, vout: 0, amount: 50_000)

        let stamped = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(stamped.isSpent, "sanity: the drained claim holds the coin")
        XCTAssertEqual(stamped.supersededByTxid, unrecordedWinner)
        XCTAssertNil(stamped.spendingTransaction, "sanity: no relationship to reach it by")

        // Second sweep: X beats the never-recorded W, freeing the coin.
        sweep(handler, [
            Batch(
                losers: [unrecordedWinner],
                winner: finalWinner,
                winnerMinedHeight: 400,
                released: [(txid: fundingTxid, vout: 0)]
            )
        ])

        let freed = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(
            freed.isSpent,
            "the release must reach a drained claim even with no row and no tombstone left"
        )
        XCTAssertNil(freed.supersededByTxid)
    }

    /// The multi-wallet continuation of the chained scenarios above. A
    /// shared loser L spends one still-unfunded coin of wallet A's and two
    /// of wallet B's, so the first sweep leaves each wallet's claims as
    /// detached tombstones pointing at winner W. W's own record then
    /// arrives through wallet A, staging A's own ordinary claim rows beside
    /// B's tombstones (rows are per wallet). When X — spending only B's
    /// second coin — sweeps W, both wallets release the other two coins;
    /// wallet A's callback runs first, frees its own coin, holds both of
    /// B's for X and deletes W's row. Wallet B's independently committed
    /// callback then runs against a row that no longer exists and must
    /// still apply its own release by outpoint: without it B's released
    /// coin would later come back spent under X, and B's held coin's
    /// tombstone could not follow any further sweep.
    func testSharedWinnerDeletedByAnotherWalletsCallbackStillReconcilesThisWalletsTombstones() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        let walletB = Data(repeating: 0x02, count: 32)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentWallet(walletId: walletB, network: .testnet))

        let sharedLoser = Data(repeating: 0xC1, count: 32) // L
        let sharedWinner = Data(repeating: 0xC2, count: 32) // W
        let finalWinner = Data(repeating: 0xC3, count: 32) // X

        let l = loserRow(
            txid: sharedLoser,
            spending: [(txid: fundingTxid, vout: 0), (txid: fundingTxid, vout: 1), (txid: fundingTxid, vout: 2)],
            netAmount: -140_000
        )
        context.insert(l)
        // None of the three coins L claims has been funded here yet: one of
        // wallet A's (vout 0) and two of wallet B's (vouts 1 and 2), all
        // parked as pending inputs the way `resolveInputOutpoint` does.
        for (vout, owner) in [(UInt32(0), walletId), (1, walletB), (2, walletB)] {
            context.insert(PersistentPendingInput(
                outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: vout),
                inputIndex: vout,
                spendingTxid: sharedLoser,
                spendingTransaction: l,
                walletId: owner
            ))
        }
        try context.save()

        // First sweep, one independently committed callback per wallet: W
        // beats L, holding everything (nothing funded, nothing released).
        sweep(handler, [Batch(losers: [sharedLoser], winner: sharedWinner, winnerMinedHeight: 400)], walletId: walletId)
        sweep(handler, [Batch(losers: [sharedLoser], winner: sharedWinner, winnerMinedHeight: 400)], walletId: walletB)
        XCTAssertNil(transaction(container, txid: sharedLoser), "L is gone once both wallets ran")

        // W's own record arrives through wallet A, claiming all three
        // outpoints: wallet A's ordinary claim rows are staged beside B's
        // tombstones (the tombstone does not occupy A's key on vout 0, and
        // B's tombstones are not A's rows on vouts 1 and 2).
        deliverReinstatingRecord(
            handler,
            walletId: walletId,
            txid: sharedWinner,
            context: 0,
            blockHeight: 0,
            inputOutpoints: [
                (txid: fundingTxid, vout: 0),
                (txid: fundingTxid, vout: 1),
                (txid: fundingTxid, vout: 2),
            ],
            outputVout: 0,
            outputAmount: 120_000,
            outputAddress: "yWinnerChange"
        )

        // Second sweep: X beats W on vout 1 alone, so upstream releases
        // vouts 0 and 2 in both wallets' views. Wallet A's callback runs
        // first, frees its own coin, holds B's for X and deletes the shared
        // row.
        sweep(handler, [
            Batch(
                losers: [sharedWinner],
                winner: finalWinner,
                winnerMinedHeight: 400,
                released: [(txid: fundingTxid, vout: 0), (txid: fundingTxid, vout: 2)]
            )
        ], walletId: walletId)
        XCTAssertNil(
            transaction(container, txid: sharedWinner),
            "sanity: wallet A's callback deleted the shared winner row — the premise "
                + "wallet B's callback below has to survive"
        )

        // Wallet B's callback arrives after the row is gone, releasing one
        // of its two coins and holding the other.
        sweep(handler, [
            Batch(
                losers: [sharedWinner],
                winner: finalWinner,
                winnerMinedHeight: 400,
                released: [(txid: fundingTxid, vout: 0), (txid: fundingTxid, vout: 2)]
            )
        ], walletId: walletB)

        let heldOutpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 1)
        let heldDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == heldOutpoint }
        )
        let heldTombstone = try XCTUnwrap(
            try context.fetch(heldDescriptor).first { $0.walletId == walletB },
            "wallet B's held tombstone must survive the row's absence"
        )
        XCTAssertEqual(
            heldTombstone.spendingTxid,
            finalWinner,
            "the held tombstone must follow the chain to X even though W's row was "
                + "already deleted by wallet A's callback"
        )
        let releasedOutpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 2)
        let releasedDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == releasedOutpoint }
        )
        XCTAssertTrue(
            try context.fetch(releasedDescriptor).isEmpty,
            "wallet B's release decision must reach its tombstone even though W's row "
                + "was already deleted by wallet A's callback"
        )

        // The funding TXOs finally arrive, one per owning wallet.
        deliverFundingUtxo(handler, walletId: walletId, vout: 0, amount: 100_000)
        deliverFundingUtxo(handler, walletId: walletB, vout: 1, amount: 40_000)
        deliverFundingUtxo(handler, walletId: walletB, vout: 2, amount: 20_000)

        let coinA = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(coinA.isSpent, "wallet A's released coin comes back spendable")
        let heldB = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(heldB.isSpent, "wallet B's held coin stays spent")
        XCTAssertEqual(
            heldB.supersededByTxid,
            finalWinner,
            "the held coin must be attributed to the final winner, not the deleted W"
        )
        let releasedB = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 2))
        XCTAssertFalse(
            releasedB.isSpent,
            "wallet B's released coin must not resurrect spent under the obsolete winner"
        )
        XCTAssertNil(releasedB.supersededByTxid)
    }

    /// Hand a UTXO for `(fundingTxid, vout)` back through the ordinary
    /// account changeset — the same entry point `redeliverCoinB` drives, but
    /// generalized so a fresh outpoint can be delivered rather than the one
    /// baked into `seedSpend`.
    private func deliverFundingUtxo(
        _ handler: PlatformWalletPersistenceHandler,
        vout: UInt32,
        amount: UInt64
    ) {
        deliverFundingUtxo(handler, walletId: walletId, vout: vout, amount: amount)
    }

    /// `walletId`-parameterized form for the multi-wallet tests, where each
    /// wallet's own funding UTXO has to arrive through that wallet's own
    /// changeset — the drain in `upsertUtxo` resolves the tombstone by
    /// outpoint, but the round itself is wallet-scoped like every real one.
    private func deliverFundingUtxo(
        _ handler: PlatformWalletPersistenceHandler,
        walletId: Data,
        vout: UInt32,
        amount: UInt64
    ) {
        let name = strdup("Standard { index: 0 }")
        let address = strdup("yFundAddr")
        defer {
            free(name)
            free(address)
        }

        var utxo = UtxoEntryFFI()
        Swift.withUnsafeMutableBytes(of: &utxo.outpoint.txid) { dst in
            fundingTxid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        utxo.outpoint.vout = vout
        utxo.amount = amount
        utxo.address = address
        utxo.height = 100
        utxo.is_confirmed = true

        handler.beginChangeset(walletId: walletId)
        withUnsafeMutablePointer(to: &utxo) { utxoPtr in
            var account = AccountChangeSetFFI()
            account.account_type_name = name
            account.utxos_added = utxoPtr
            account.utxos_added_count = 1
            withUnsafeMutablePointer(to: &account) { accountPtr in
                var cs = WalletChangeSetFFI()
                cs.accounts = accountPtr
                cs.accounts_count = 1
                withUnsafePointer(to: &cs) { csPtr in
                    handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                }
            }
        }
        _ = handler.endChangeset(walletId: walletId, success: true)
    }

    /// Deliver a plain transaction record — with a fresh output of its own
    /// riding along in the same round — through the ordinary account
    /// changeset entry point. Models the reinstating event the BLOCKING
    /// finding describes: upstream reports a previously-swept txid to
    /// `records` exactly the way it reports any freshly-detected
    /// transaction, with nothing on the wire flagging it as "the one that
    /// used to be swept" — `upsertTransaction` has to infer that entirely
    /// from the row it finds already sitting in the store.
    private func deliverReinstatingRecord(
        _ handler: PlatformWalletPersistenceHandler,
        walletId: Data,
        txid: Data,
        context: UInt32,
        blockHeight: UInt32,
        inputOutpoints: [(txid: Data, vout: UInt32)],
        outputVout: UInt32,
        outputAmount: UInt64,
        outputAddress: String
    ) {
        let name = strdup("Standard { index: 0 }")
        let address = strdup(outputAddress)
        defer {
            free(name)
            free(address)
        }

        let inputs = UnsafeMutablePointer<OutPointFFI>.allocate(
            capacity: max(inputOutpoints.count, 1)
        )
        for (i, input) in inputOutpoints.enumerated() {
            var entry = OutPointFFI()
            Swift.withUnsafeMutableBytes(of: &entry.txid) { dst in
                input.txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            entry.vout = input.vout
            inputs.advanced(by: i).initialize(to: entry)
        }
        defer {
            inputs.deinitialize(count: inputOutpoints.count)
            inputs.deallocate()
        }

        var record = TransactionRecordFFI()
        Swift.withUnsafeMutableBytes(of: &record.txid) { dst in
            txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        record.context = context
        record.block_height = blockHeight
        record.input_outpoints = inputs
        record.input_outpoints_count = UInt(inputOutpoints.count)

        var utxo = UtxoEntryFFI()
        Swift.withUnsafeMutableBytes(of: &utxo.outpoint.txid) { dst in
            txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        utxo.outpoint.vout = outputVout
        utxo.amount = outputAmount
        utxo.address = address
        utxo.height = blockHeight
        utxo.is_confirmed = true

        handler.beginChangeset(walletId: walletId)
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
                    cs.accounts = accountPtr
                    cs.accounts_count = 1
                    withUnsafePointer(to: &cs) { csPtr in
                        handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                    }
                }
            }
        }
        _ = handler.endChangeset(walletId: walletId, success: true)
    }

    // MARK: - Bounded tombstone lifetime

    /// The block-context winner's mined height used across the bounded-
    /// lifetime tests — the stamp every tombstone carries, and the exact
    /// boundary value at which it collects.
    private static let winnerHeight: UInt32 = 400

    /// One committed round carrying chain progress: the synced height,
    /// (unless the caller opts out) opaque chainlock bytes, and — when
    /// `chainLockHeight` is supplied — the NUMERIC chainlock height
    /// through the extension's dedicated slot, fired inside the same
    /// begin/end bracket after the changeset callback exactly the way the
    /// Rust persister fires it. The bytes and the number are deliberately
    /// independent knobs: the reviewer's point is precisely that bytes
    /// alone must not enable collection.
    private func heightsRound(
        _ handler: PlatformWalletPersistenceHandler,
        synced: UInt32,
        chainLock: Bool = true,
        chainLockHeight: UInt32? = nil
    ) {
        handler.beginChangeset(walletId: walletId)
        var cs = WalletChangeSetFFI()
        cs.has_chain = true
        cs.chain.has_synced_height = true
        cs.chain.synced_height = synced
        var clBytes = [UInt8](repeating: 9, count: 84)
        clBytes.withUnsafeMutableBufferPointer { buf in
            if chainLock {
                cs.last_applied_chain_lock_bytes = buf.baseAddress
                cs.last_applied_chain_lock_bytes_len = UInt(buf.count)
            }
            withUnsafePointer(to: &cs) { csPtr in
                _ = handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
            }
        }
        if let chainLockHeight {
            _ = handler.persistWalletChangesetChainLockHeight(
                walletId: walletId,
                height: chainLockHeight
            )
        }
        _ = handler.endChangeset(walletId: walletId, success: true)
    }

    /// Record a loser spending `(spentTxid, 0)` with the funding side
    /// unobserved, then sweep it in the given winner context —
    /// `winnerMinedHeight` non-nil leaves the stamped tombstone the
    /// collection tests reason about; `nil` (an IS-locked, unmined winner)
    /// must leave nothing.
    private func seedSweptTombstone(
        _ handler: PlatformWalletPersistenceHandler,
        _ container: ModelContainer,
        winnerMinedHeight: UInt32?,
        spentTxid: Data? = nil,
        loser: Data? = nil,
        winner: Data? = nil
    ) throws {
        let loser = loser ?? sweptTxid
        let context = ModelContext(container)
        let swept = loserRow(txid: loser, spending: [(txid: spentTxid ?? fundingTxid, vout: 0)])
        context.insert(swept)
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: spentTxid ?? fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: loser,
            spendingTransaction: swept,
            walletId: walletId
        ))
        try context.save()
        sweep(handler, [Batch(
            losers: [loser],
            winner: winner ?? winnerTxid,
            winnerMinedHeight: winnerMinedHeight
        )])
    }

    private func pendingRows(
        _ container: ModelContainer,
        spentTxid: Data? = nil,
        vout: UInt32 = 0
    ) throws -> [PersistentPendingInput] {
        let outpoint = PersistentTxo.makeOutpoint(txid: spentTxid ?? fundingTxid, vout: vout)
        let descriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        return try ModelContext(container).fetch(descriptor)
    }

    /// Every pending-input row this wallet holds, regardless of outpoint —
    /// the attacker-growth metric the mempool-context tests measure.
    private func walletPendingRows(
        _ container: ModelContainer
    ) throws -> [PersistentPendingInput] {
        let walletId = self.walletId
        let descriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        return try ModelContext(container).fetch(descriptor)
    }

    /// This wallet's persisted row, for asserting on the stored numeric
    /// chainlock height.
    private func walletRow(_ container: ModelContainer) throws -> PersistentWallet? {
        let walletId = self.walletId
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        return try ModelContext(container).fetch(descriptor).first
    }

    /// The attacker-shaped row's lawful cousin: a block-context sweep's
    /// tombstone stores the WINNER'S own mined height and is collected
    /// exactly when the finality boundary `min(chainlockHeight,
    /// syncedHeight)` reaches it — upstream key-wallet's
    /// `prune_finalized_observed_spends` condition verbatim, no
    /// observation-age margin. At that boundary the funding transaction of
    /// the guarded outpoint (necessarily mined at or below the winner's
    /// height) has been filter-scanned with no false negatives, so an
    /// undrained tombstone is provably not guarding the wallet's coin.
    func testASweptTombstoneIsCollectedAtFinalityAndNotBefore() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        try seedSweptTombstone(handler, container, winnerMinedHeight: Self.winnerHeight)

        let tombstone = try XCTUnwrap(try pendingRows(container).first)
        XCTAssertTrue(tombstone.isSweptTombstone, "sanity: the sweep flagged the row")
        XCTAssertEqual(
            tombstone.winnerMinedHeight, Self.winnerHeight,
            "the tombstone is stamped with the WINNER'S own mined height — "
                + "not any observation watermark"
        )

        heightsRound(
            handler,
            synced: Self.winnerHeight - 1,
            chainLockHeight: Self.winnerHeight - 1
        )
        XCTAssertEqual(
            try pendingRows(container).count, 1,
            "boundary \(Self.winnerHeight - 1) has not reached the winner's "
                + "height \(Self.winnerHeight) — the hold stays"
        )

        heightsRound(handler, synced: Self.winnerHeight, chainLockHeight: Self.winnerHeight)
        XCTAssertTrue(
            try pendingRows(container).isEmpty,
            "the boundary reaching the winner's height collects the row — no margin"
        )
    }

    /// The reviewer's "weaker still" point, named: synced-height progress
    /// plus even PRESENT chainlock BYTES must not collect — the bincode
    /// blob proves a chainlock was once applied, but says nothing about
    /// how far finality reaches. Only the NUMERIC chainlock height
    /// delivered through the extension slot supplies the boundary's
    /// chainlock half, mirroring upstream's (and the SQLite store's)
    /// "no-op until a chainlock height has been persisted".
    func testASweptTombstoneOutlivesSyncProgressWithoutANumericChainLockHeight() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        try seedSweptTombstone(handler, container, winnerMinedHeight: Self.winnerHeight)

        heightsRound(handler, synced: 10_000, chainLock: true)
        XCTAssertEqual(
            try pendingRows(container).count, 1,
            "chainlock BYTES exist and the synced height is far past the "
                + "stamp — but no numeric chainlock height has ever been "
                + "stored, so no finality boundary exists and the hold stays"
        )

        heightsRound(handler, synced: 10_000, chainLockHeight: 10_000)
        XCTAssertTrue(
            try pendingRows(container).isEmpty,
            "the first NUMERIC chainlock height supplies the boundary and "
                + "the long-aged stamp collects"
        )
    }

    /// The genuine claim the tombstone exists for: its funding TXO arrives,
    /// the drain moves the hold onto the TXO row (`supersededByTxid`) and
    /// deletes the pending rows — so no amount of later boundary progress
    /// may touch the materialised hold.
    func testADrainedClaimIsImmuneToTheCollector() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        try seedSweptTombstone(handler, container, winnerMinedHeight: Self.winnerHeight)
        XCTAssertEqual(
            try XCTUnwrap(try pendingRows(container).first).winnerMinedHeight,
            Self.winnerHeight,
            "sanity: held, undrained, stamped with the winner's height"
        )

        deliverFundingUtxo(handler, vout: 0, amount: 100_000)
        XCTAssertTrue(
            try pendingRows(container).isEmpty,
            "sanity: the drain consumed the pending rows"
        )

        heightsRound(handler, synced: 10_000, chainLockHeight: 10_000)
        let coin = try XCTUnwrap(
            txo(container, txid: fundingTxid, vout: 0),
            "the materialised claim's row survives collection"
        )
        XCTAssertTrue(coin.isSpent, "still held spent by the winner's claim")
        XCTAssertEqual(coin.supersededByTxid, winnerTxid)
    }

    /// A held tombstone with a nil winner-height stamp is never collected.
    /// The mempool-context sweep path writes exactly this shape — an
    /// IS-locked, unmined winner has no finality horizon to stamp — and
    /// legacy rows read identically. With no proof of finality the safe
    /// reading is to hold it forever rather than guess.
    /// Replaces the rejected back-fill design, which stamped such a row
    /// with the current height and thereby fabricated a finality horizon.
    func testATombstoneWithoutAWinnerHeightIsNeverCollected() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        // The real writer: an IS-context sweep of a loser whose funding
        // TXO never arrived.
        try seedSweptTombstone(handler, container, winnerMinedHeight: nil)

        // Two rounds, not one: a back-filling collector (the rejected
        // design) would stamp the row on the first round and collect it on
        // the second.
        heightsRound(handler, synced: 1_000_000, chainLockHeight: 1_000_000)
        heightsRound(handler, synced: 1_000_010, chainLockHeight: 1_000_010)

        let row = try XCTUnwrap(
            try pendingRows(container).first,
            "no winner height, no proof of finality — the hold outlasts any boundary"
        )
        XCTAssertTrue(row.isSweptTombstone)
        XCTAssertNil(
            row.winnerMinedHeight,
            "and the stamp is never back-filled — that would fabricate the horizon"
        )
    }

    /// A chained sweep that re-points a still-unfunded claim to a new
    /// BLOCK-context winner also re-stamps it with THAT winner's mined
    /// height: the claim now belongs to a spend anchored at a later block,
    /// and its collection horizon moves with it.
    func testARepointedTombstoneIsRestampedToTheLaterSweep() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        try seedSweptTombstone(handler, container, winnerMinedHeight: Self.winnerHeight)
        XCTAssertEqual(
            try XCTUnwrap(try pendingRows(container).first).winnerMinedHeight,
            Self.winnerHeight,
            "sanity: stamped with the first winner's mined height"
        )

        // The first winner is itself swept — by a winner mined 50 blocks
        // later — the chained-sweep continuation that re-points the
        // earlier tombstone (no row needed: the tombstone is found by the
        // scalar `spendingTxid` it carries).
        let finalWinner = Data(repeating: 0x66, count: 32)
        sweep(handler, [Batch(
            losers: [winnerTxid],
            winner: finalWinner,
            winnerMinedHeight: Self.winnerHeight + 50
        )])

        let row = try XCTUnwrap(try pendingRows(container).first)
        XCTAssertTrue(row.isSweptTombstone)
        XCTAssertEqual(row.spendingTxid, finalWinner)
        XCTAssertEqual(
            row.winnerMinedHeight, Self.winnerHeight + 50,
            "re-pointed to a later block-context winner ⇒ re-stamped to "
                + "THAT winner's mined height"
        )

        // And the horizon moved with it: the old height no longer collects,
        // the new one does.
        heightsRound(
            handler,
            synced: Self.winnerHeight + 49,
            chainLockHeight: Self.winnerHeight + 49
        )
        XCTAssertEqual(
            try pendingRows(container).count, 1,
            "the boundary reaching only the FIRST winner's height must no "
                + "longer collect the re-stamped claim"
        )
        heightsRound(
            handler,
            synced: Self.winnerHeight + 50,
            chainLockHeight: Self.winnerHeight + 50
        )
        XCTAssertTrue(try pendingRows(container).isEmpty)
    }

    /// A mempool-context sweep — an InstantSend-locked winner that has not
    /// mined — preserves an UNSTAMPED tombstone for every held-but-unfunded
    /// input. Under DIP-10 the IS lock alone settles those inputs: upstream
    /// deletes the loser and retains them in the account's
    /// `spent_outpoints`, a hold with no height that no record survives to
    /// rebuild (the winner need not be wallet-relevant). The tombstone is
    /// that hold's only durable carrier — `CORE_SWEEP_REMOVAL` requires
    /// every non-released input to keep a durable spend claim before its
    /// funding TXO materializes — and it is unstamped because an IS-locked
    /// winner has no mining deadline, so no boundary may ever collect it.
    func testAMempoolContextSweepPreservesAnUnstampedTombstone() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()

        for i in 0..<3 {
            let spent = Data(repeating: UInt8(0x70 + i), count: 32)
            try seedSweptTombstone(
                handler,
                container,
                winnerMinedHeight: nil,
                spentTxid: spent,
                loser: Data(repeating: UInt8(0x80 + i), count: 32),
                winner: Data(repeating: UInt8(0x90 + i), count: 32)
            )
            let row = try XCTUnwrap(
                try pendingRows(container, spentTxid: spent).first,
                "an unmined IS-locked winner must leave a held tombstone for input #\(i)"
            )
            XCTAssertTrue(row.isSweptTombstone)
            XCTAssertNil(row.winnerMinedHeight, "and it carries no finality stamp")
        }
        // Arbitrary chainlock/height advancement never collects an
        // unstamped hold — two rounds, so a back-filling collector would
        // be caught too.
        heightsRound(handler, synced: 1_000_000, chainLockHeight: 1_000_000)
        heightsRound(handler, synced: 1_000_010, chainLockHeight: 1_000_010)
        XCTAssertEqual(
            try walletPendingRows(container).count, 3,
            "every unstamped hold outlasts any boundary — only funding "
                + "materialization, a block-context re-stamp, or a release "
                + "resolves one"
        )
    }

    /// The mempool-context sweep still spend-marks a coin that HAS
    /// materialised — that path is unchanged: the row carries real funding
    /// data and `supersededByTxid` is its durable hold. The
    /// never-materialised claim the same loser carries survives too, as an
    /// unstamped tombstone — the pending row is the only durable carrier
    /// of a hold upstream keeps in `spent_outpoints` and cannot rebuild
    /// after the loser's record is gone.
    func testAMempoolContextSweepStillSpendMarksAMaterialisedCoin() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)

        // The same loser also claims an input whose funding side was never
        // observed — the shape that would have become a tombstone. Its
        // bytes name that input too, the way a real record's would.
        let unfundedTxid = Data(repeating: 0x77, count: 32)
        let context = ModelContext(container)
        let loserTxid = sweptTxid
        let loser = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == loserTxid }
            )).first
        )
        loser.transactionData = serializedTransaction(inputs: [
            (txid: fundingTxid, vout: 0), (txid: fundingTxid, vout: 1), (txid: unfundedTxid, vout: 0),
        ])
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: unfundedTxid, vout: 0),
            inputIndex: 2,
            spendingTxid: sweptTxid,
            spendingTransaction: loser,
            walletId: walletId
        ))
        try context.save()

        sweep(handler, [Batch(
            losers: [sweptTxid],
            winner: winnerTxid,
            winnerMinedHeight: nil
        )])

        let coinB = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(
            coinB.isSpent,
            "a materialised coin is spend-marked by the IS-locked winner exactly as before"
        )
        XCTAssertEqual(coinB.supersededByTxid, winnerTxid)
        let claim = try XCTUnwrap(
            try pendingRows(container, spentTxid: unfundedTxid).first,
            "while the never-materialised claim survives as a tombstone"
        )
        XCTAssertTrue(claim.isSweptTombstone)
        XCTAssertEqual(claim.spendingTxid, winnerTxid, "re-pointed at the winner")
        XCTAssertNil(claim.winnerMinedHeight, "unstamped — the winner is unmined")
    }

    /// The reviewer's named regression: an IS-locked winner sweeps on the
    /// mempool path and never mines, the app restarts, chainlocks and
    /// heights advance arbitrarily, and only then is the funding output
    /// delivered. Under DIP-10 the IS lock already settled that input —
    /// upstream deleted the loser and retained the hold in the account's
    /// `spent_outpoints`, a set rebuilt from records on load that no
    /// surviving record can reconstruct. The unstamped tombstone is the
    /// claim's only durable carrier, so the funding delivery must drain
    /// INTO it and land spent: crediting the coin would hand coin
    /// selection an outpoint the network has provably consumed.
    func testAFundingOutputArrivingAfterAMempoolSweepAndRestartLandsSpent() throws {
        let storeURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("mempool-sweep-restart-\(UUID().uuidString).store")
        defer { try? FileManager.default.removeItem(at: storeURL) }

        do {
            let (handler, container) = try makeHandler(url: storeURL)
            let context = ModelContext(container)
            context.insert(PersistentWallet(walletId: walletId, network: .testnet))
            try context.save()
            try seedSweptTombstone(handler, container, winnerMinedHeight: nil)
            XCTAssertNil(transaction(container, txid: sweptTxid), "sanity: the loser is gone")
            let tombstone = try XCTUnwrap(
                try walletPendingRows(container).first,
                "sanity: the mempool sweep left the hold behind"
            )
            XCTAssertTrue(tombstone.isSweptTombstone)
            XCTAssertNil(tombstone.winnerMinedHeight, "unstamped — no finality horizon exists")
        }

        // Restart: a fresh persister loading the same on-disk store, then
        // arbitrary chainlock/height advancement while the winner stays
        // unmined — none of it may collect the unstamped hold — and only
        // then the funding delivery.
        let (handler, container) = try makeHandler(url: storeURL)
        heightsRound(handler, synced: 25_000, chainLockHeight: 25_000)
        XCTAssertEqual(
            try walletPendingRows(container).count, 1,
            "the unstamped hold survives the restart and every boundary"
        )
        deliverFundingUtxo(handler, vout: 0, amount: 100_000)

        let coin = try XCTUnwrap(
            txo(container, txid: fundingTxid, vout: 0),
            "the funding UTXO's own upsert must still create the row"
        )
        XCTAssertTrue(
            coin.isSpent,
            "an input the IS-locked winner consumed must never come back "
                + "spendable — the sweep's claim outlives the restart"
        )
        XCTAssertEqual(coin.supersededByTxid, winnerTxid, "held by the winner the sweep named")
        XCTAssertTrue(
            try walletPendingRows(container).isEmpty,
            "the claim drained into the TXO row"
        )
    }

    /// The unrelated-advancement scenario, block-context half: the
    /// chainlock can run arbitrarily far ahead, but while `syncedHeight`
    /// sits below the winner's mined height the boundary has not reached
    /// the spend and the hold must survive — the funding output could
    /// still be delivered by the unscanned range. It collects the moment
    /// the synced height catches up.
    func testABlockContextTombstoneOutlivesUnrelatedAdvancementBelowItsWinnersHeight() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        try seedSweptTombstone(handler, container, winnerMinedHeight: Self.winnerHeight)

        // Chainlocks race ahead by thousands of blocks; the filter scan
        // has only reached one block short of the winner.
        heightsRound(
            handler,
            synced: Self.winnerHeight - 1,
            chainLockHeight: Self.winnerHeight + 10_000
        )
        XCTAssertEqual(
            try pendingRows(container).count, 1,
            "min(chainlock, synced) = \(Self.winnerHeight - 1) is below the "
                + "winner's height — any amount of unrelated chainlock "
                + "progress must not collect the hold"
        )

        // No fresh chainlock this round: the changeset-path collector runs
        // off the STORED numeric height.
        heightsRound(handler, synced: Self.winnerHeight)
        XCTAssertTrue(
            try pendingRows(container).isEmpty,
            "the scan reaching the winner's height completes the boundary and collects"
        )
    }

    /// The other direction of the chained case: an UNSTAMPED hold
    /// (IS-context sweep) re-pointed by a later BLOCK-context sweep gains
    /// that winner's stamp — the claim now belongs to a spend anchored in
    /// a real block, so it enters the collectible set and the boundary
    /// reaching the new winner's height collects it. One of the three
    /// resolution channels that bound the unstamped population.
    func testAnUnstampedTombstoneRestampedByABlockContextSweepBecomesCollectible() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        // IS-context sweep: the hold lands unstamped.
        try seedSweptTombstone(handler, container, winnerMinedHeight: nil)
        XCTAssertNil(
            try XCTUnwrap(try pendingRows(container).first).winnerMinedHeight,
            "sanity: held and unstamped"
        )

        // The IS-locked first winner is itself beaten by a mined conflict
        // still claiming the unfunded input — the chained-sweep
        // continuation finds the tombstone by its scalar `spendingTxid`.
        let finalWinner = Data(repeating: 0x66, count: 32)
        sweep(handler, [Batch(
            losers: [winnerTxid],
            winner: finalWinner,
            winnerMinedHeight: Self.winnerHeight + 50
        )])

        let row = try XCTUnwrap(try pendingRows(container).first)
        XCTAssertTrue(row.isSweptTombstone)
        XCTAssertEqual(row.spendingTxid, finalWinner)
        XCTAssertEqual(
            row.winnerMinedHeight, Self.winnerHeight + 50,
            "the block-context re-point stamps the previously unstamped hold"
        )

        heightsRound(
            handler,
            synced: Self.winnerHeight + 50,
            chainLockHeight: Self.winnerHeight + 50
        )
        XCTAssertTrue(
            try pendingRows(container).isEmpty,
            "once stamped, the ordinary finality boundary collects the row"
        )
    }

    /// The IS-locked half of the chained case: an unmined winner re-points
    /// the claim but must NOT disturb the earlier block-context stamp —
    /// upstream's observed-spend entry is never retracted by an
    /// unconfirmed conflict. Collection at the retained height stays sound
    /// (the funding output is mined at or below the FIRST spender's height
    /// regardless of who claims the coin now), so the row still collects
    /// at that boundary.
    func testAMempoolRepointedTombstoneKeepsItsBlockContextStamp() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        try seedSweptTombstone(handler, container, winnerMinedHeight: Self.winnerHeight)

        // The first winner is evicted by an IS-locked, unmined conflict.
        let finalWinner = Data(repeating: 0x66, count: 32)
        sweep(handler, [Batch(
            losers: [winnerTxid],
            winner: finalWinner,
            winnerMinedHeight: nil
        )])

        let row = try XCTUnwrap(try pendingRows(container).first)
        XCTAssertEqual(row.spendingTxid, finalWinner)
        XCTAssertEqual(
            row.winnerMinedHeight, Self.winnerHeight,
            "an unmined winner re-points the claim without touching the "
                + "earlier block-context stamp"
        )

        heightsRound(handler, synced: Self.winnerHeight, chainLockHeight: Self.winnerHeight)
        XCTAssertTrue(
            try pendingRows(container).isEmpty,
            "the retained stamp still bounds the row: the funding output "
                + "sits at or below the first spender's height, so the "
                + "boundary reaching it proves delivery-or-never"
        )
    }

    /// The chainlock-height extension callback stores monotonic-max on the
    /// wallet row: chain locks only move forward, and a late or re-emitted
    /// lower height must not walk the finality boundary backwards.
    func testTheChainLockHeightCallbackStoresMonotonicMaxOnTheWalletRow() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        XCTAssertNil(
            try XCTUnwrap(try walletRow(container)).lastAppliedChainLockHeight,
            "sanity: fresh row, no numeric chainlock height yet"
        )

        heightsRound(handler, synced: 10, chainLockHeight: 500)
        XCTAssertEqual(
            try XCTUnwrap(try walletRow(container)).lastAppliedChainLockHeight, 500,
            "the first height lands as stored"
        )

        heightsRound(handler, synced: 11, chainLockHeight: 300)
        XCTAssertEqual(
            try XCTUnwrap(try walletRow(container)).lastAppliedChainLockHeight, 500,
            "a lower height must not walk the watermark backwards"
        )

        heightsRound(handler, synced: 12, chainLockHeight: 700)
        XCTAssertEqual(
            try XCTUnwrap(try walletRow(container)).lastAppliedChainLockHeight, 700,
            "a higher height advances it"
        )
    }

    // MARK: - Review round: hold by outpoint, per-wallet rows, one-round shapes

    /// The hold is keyed by the loser's decoded inputs, not by the links
    /// its row happens to hold. Coin A's link already moved to a surviving
    /// mempool spender M when L is swept: A is not released, so it is held
    /// for the winner, and M's link — not the loser's — is kept; coin B,
    /// linked to the loser, is detached. A link-keyed walk never saw A.
    func testASweepHoldsEveryDecodedInputAndDetachesOnlyTheLosersOwnLinks() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)

        let survivorTxid = Data(repeating: 0x4A, count: 32)
        let context = ModelContext(container)
        let survivor = loserRow(txid: survivorTxid, spending: [(txid: fundingTxid, vout: 0)])
        context.insert(survivor)
        let coinA = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)
        let a = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentTxo>(
                predicate: #Predicate { $0.outpoint == coinA }
            )).first
        )
        a.spendingTransaction = survivor
        try context.save()

        sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])

        let held = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(held.isSpent, "an input the loser's bytes name is held even with its link elsewhere")
        XCTAssertEqual(held.supersededByTxid, winnerTxid)
        XCTAssertEqual(held.spendingTransaction?.txid, survivorTxid, "a link that is not the loser's is kept")

        let detached = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(detached.isSpent)
        XCTAssertNil(detached.spendingTransaction, "the loser's own link is detached")
    }

    /// A held input with no `PersistentTxo` and no pending row of this
    /// wallet's gets its tombstone created: the loser's stored bytes name
    /// the coin, and the claim must not depend on a row `resolveInputOutpoint`
    /// happened to leave behind.
    func testASweepCreatesTheTombstoneForAHeldInputWithNoClaimRow() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(loserRow(txid: sweptTxid, spending: [(txid: fundingTxid, vout: 0)]))
        try context.save()
        XCTAssertTrue(try pendingRows(container).isEmpty, "sanity: no claim row at all")

        sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])

        let tombstone = try XCTUnwrap(try pendingRows(container).first, "the hold is created from the bytes")
        XCTAssertTrue(tombstone.isSweptTombstone)
        XCTAssertEqual(tombstone.spendingTxid, winnerTxid)
        XCTAssertEqual(tombstone.walletId, walletId)
        XCTAssertEqual(tombstone.winnerMinedHeight, 400)

        deliverFundingUtxo(handler, vout: 0, amount: 100_000)
        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent)
        XCTAssertEqual(coin.supersededByTxid, winnerTxid)
    }

    /// Pending rows are per (outpoint, spending txid, wallet): a second
    /// wallet recording the same transaction gets its own claim row. Every
    /// sweep decision on a pending row is scoped by that tag, so a claim
    /// tagged with the first recorder alone would let one wallet's released
    /// set decide the other wallet's coin.
    func testASecondWalletRecordingTheSameSpendGetsItsOwnPendingRow() throws {
        let (handler, container) = try makeHandler()
        let walletB = Data(repeating: 0x02, count: 32)
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentWallet(walletId: walletB, network: .testnet))
        try context.save()

        deliverRecord(handler, walletId: walletId, txid: sweptTxid, context: 0, inputOutpoints: [(txid: fundingTxid, vout: 0)])
        deliverRecord(handler, walletId: walletB, txid: sweptTxid, context: 0, inputOutpoints: [(txid: fundingTxid, vout: 0)])

        let rows = try pendingRows(container)
        XCTAssertEqual(Set(rows.map(\.walletId)), [walletId, walletB], "one claim row per recording wallet")
        XCTAssertEqual(rows.count, 2)

        // And a re-upsert by the same wallet still does not duplicate.
        deliverRecord(handler, walletId: walletB, txid: sweptTxid, context: 0, inputOutpoints: [(txid: fundingTxid, vout: 0)])
        XCTAssertEqual(try pendingRows(container).count, 2)
    }

    /// At drain time the tombstone tagged with the delivering wallet wins
    /// over another wallet's, whatever their ages: the stamp is that
    /// wallet's own sweep verdict on its own coin.
    func testTheDrainPrefersTheTombstoneTaggedWithTheDeliveringWallet() throws {
        let (handler, container) = try makeHandler()
        let walletB = Data(repeating: 0x02, count: 32)
        let winnerForA = Data(repeating: 0x5A, count: 32)
        let winnerForB = Data(repeating: 0x5B, count: 32)
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentWallet(walletId: walletB, network: .testnet))
        let outpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)
        // B's tombstone is the OLDER one, so a newest-wins pick would take A's.
        let forB = PersistentPendingInput(
            outpoint: outpoint, inputIndex: 0, spendingTxid: winnerForB, spendingTransaction: nil, walletId: walletB
        )
        forB.isSweptTombstone = true
        forB.createdAt = Date(timeIntervalSinceNow: -10)
        let forA = PersistentPendingInput(
            outpoint: outpoint, inputIndex: 0, spendingTxid: winnerForA, spendingTransaction: nil, walletId: walletId
        )
        forA.isSweptTombstone = true
        context.insert(forB)
        context.insert(forA)
        try context.save()

        deliverFundingUtxo(handler, walletId: walletB, vout: 0, amount: 100_000)

        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent)
        XCTAssertEqual(coin.supersededByTxid, winnerForB, "the delivering wallet's own tombstone supplies the stamp")
        XCTAssertTrue(try pendingRows(container).isEmpty, "every pending row on the outpoint is consumed by the drain")
    }

    /// A drained tombstone stamps and nothing more. Its `inputIndex` is the
    /// LOSER'S vin (L spent F at vin 0), and the winner it names spends F —
    /// if at all — somewhere else; copying the index onto the winner's link
    /// mislabelled the winner's own inputs, and minting the link from a
    /// tombstone attributed a coin to a transaction that need not spend it.
    /// W's row exists here precisely so an old drain WOULD have linked it.
    func testADrainedTombstoneStampsWithoutMintingALinkOrAVinIndex() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(loserRow(txid: winnerTxid, spending: [(txid: Data(repeating: 0x58, count: 32), vout: 0)]))
        let tombstone = PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0),
            inputIndex: 0,
            spendingTxid: winnerTxid,
            spendingTransaction: nil,
            walletId: walletId
        )
        tombstone.isSweptTombstone = true
        tombstone.winnerMinedHeight = 400
        context.insert(tombstone)
        try context.save()

        deliverFundingUtxo(handler, vout: 0, amount: 100_000)

        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent, "the stamp holds the coin")
        XCTAssertEqual(coin.supersededByTxid, winnerTxid)
        XCTAssertNil(coin.spendingTransaction, "no link is minted from a tombstone")
        XCTAssertNil(coin.spendingInputIndex, "and no vin index — the tombstone's is the loser's")
    }

    /// The winner's own ordinary claim row beside the tombstone is what
    /// carries the link and the RIGHT vin index: W spends X at vin 0 and F
    /// at vin 1, while the loser had spent F at vin 0.
    func testTheWinnersOwnPendingRowSuppliesTheLinkAndVinIndexBesideATombstone() throws {
        let (handler, container) = try makeHandler()
        let otherCoinTxid = Data(repeating: 0x58, count: 32)
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let winner = loserRow(
            txid: winnerTxid,
            spending: [(txid: otherCoinTxid, vout: 0), (txid: fundingTxid, vout: 0)]
        )
        context.insert(winner)
        let outpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)
        let tombstone = PersistentPendingInput(
            outpoint: outpoint, inputIndex: 0, spendingTxid: winnerTxid, spendingTransaction: nil, walletId: walletId
        )
        tombstone.isSweptTombstone = true
        tombstone.createdAt = Date(timeIntervalSinceNow: -10)
        context.insert(tombstone)
        context.insert(PersistentPendingInput(
            outpoint: outpoint, inputIndex: 1, spendingTxid: winnerTxid, spendingTransaction: winner, walletId: walletId
        ))
        try context.save()

        deliverFundingUtxo(handler, vout: 0, amount: 100_000)

        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent)
        XCTAssertEqual(coin.supersededByTxid, winnerTxid, "the tombstone supplies the stamp")
        XCTAssertEqual(coin.spendingTransaction?.txid, winnerTxid, "the ordinary row supplies the link")
        XCTAssertEqual(coin.spendingInputIndex, 1, "and the winner's own vin index, not the loser's")
    }

    /// One bracket carrying the winner's record and the loser's sweep — the
    /// shape Rust produces when it folds `TransactionDetected(W)` and
    /// `TransactionsSwept{[L]}` into one `store()`. The record pass moves
    /// coin A's link from L to W; the sweep must then find L through the
    /// round index rather than a store-only fetch, because a store-only
    /// refetch of L resets its `inputs` inverse to the saved `[A, B]` and
    /// with it A's freshly written link. After the commit W must still own
    /// A: `walletFundedTransaction(W)` reads exactly that link, and a
    /// chainlock promotion never re-emits the record.
    func testAWinnerRecordedAndItsLoserSweptInOneRoundKeepsTheWinnersInputLink() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)

        round(handler) {
            stageRecord(handler, txid: winnerTxid, context: 1, inputOutpoints: [(txid: fundingTxid, vout: 0)])
            return stageSweeps(handler, [
                Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: nil, released: [(txid: fundingTxid, vout: 1)])
            ], walletId: walletId)
        }

        XCTAssertNil(transaction(container, txid: sweptTxid), "the loser is gone")
        let taken = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertEqual(taken.spendingTransaction?.txid, winnerTxid, "the winner keeps the link it recorded this round")
        XCTAssertTrue(taken.isSpent)
        let winner = try XCTUnwrap(transaction(container, txid: winnerTxid))
        XCTAssertEqual(winner.inputs.map(\.outpoint), [PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)])
        XCTAssertTrue(
            PlatformWalletPersistenceHandler.walletFundedTransaction(walletId: walletId, transaction: winner),
            "the winner reads as wallet-funded after the commit"
        )
        let freed = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertFalse(freed.isSpent)
    }

    /// Stage a transaction record inside whatever bracket the caller
    /// opened — the record half of `deliverRecord`, without the round.
    private func stageRecord(
        _ handler: PlatformWalletPersistenceHandler,
        txid: Data,
        context: UInt32,
        inputOutpoints: [(txid: Data, vout: UInt32)]
    ) {
        let name = strdup("Standard { index: 0 }")
        defer { free(name) }
        var inputs: [OutPointFFI] = inputOutpoints.map { outpoint in
            var input = OutPointFFI()
            Swift.withUnsafeMutableBytes(of: &input.txid) { dst in
                outpoint.txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            input.vout = outpoint.vout
            return input
        }
        var record = TransactionRecordFFI()
        Swift.withUnsafeMutableBytes(of: &record.txid) { dst in
            txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        record.context = context
        record.block_height = 0
        inputs.withUnsafeMutableBufferPointer { inputsPtr in
            record.input_outpoints = inputsPtr.baseAddress
            record.input_outpoints_count = UInt(inputsPtr.count)
            withUnsafeMutablePointer(to: &record) { recordPtr in
                var account = AccountChangeSetFFI()
                account.account_type_name = name
                account.transactions = recordPtr
                account.transactions_count = 1
                withUnsafeMutablePointer(to: &account) { accountPtr in
                    var cs = WalletChangeSetFFI()
                    cs.accounts = accountPtr
                    cs.accounts_count = 1
                    withUnsafePointer(to: &cs) { csPtr in
                        handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                    }
                }
            }
        }
    }

    /// The collector runs once per round, at the END — after the round's
    /// own `utxos_added`. Rust folds `BlockProcessed` and
    /// `SyncHeightAdvanced` into one `store()`, so the round that advances
    /// `syncedHeight` to the winner's height can be the very round that
    /// delivers the funding output the tombstone guards. Collecting first
    /// deleted the tombstone, and the funding output then landed unspent —
    /// a coin the chainlocked winner consumed, handed back as spendable.
    func testAFundingOutputDeliveredInTheRoundThatCompletesTheBoundaryStillDrainsItsTombstone() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        try seedSweptTombstone(handler, container, winnerMinedHeight: Self.winnerHeight)
        // The chainlock half is already past the stamp; the synced half is
        // one block short.
        heightsRound(handler, synced: Self.winnerHeight - 1, chainLockHeight: Self.winnerHeight + 100)
        XCTAssertEqual(try pendingRows(container).count, 1, "sanity: boundary not reached yet")

        // ONE round: the synced height reaches the stamp AND the funding
        // output arrives.
        round(handler) {
            let name = strdup("Standard { index: 0 }")
            let address = strdup("yFundAddr")
            defer {
                free(name)
                free(address)
            }
            var utxo = UtxoEntryFFI()
            Swift.withUnsafeMutableBytes(of: &utxo.outpoint.txid) { dst in
                fundingTxid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            utxo.outpoint.vout = 0
            utxo.amount = 100_000
            utxo.address = address
            utxo.height = Self.winnerHeight - 5
            utxo.is_confirmed = true
            var applied = false
            withUnsafeMutablePointer(to: &utxo) { utxoPtr in
                var account = AccountChangeSetFFI()
                account.account_type_name = name
                account.utxos_added = utxoPtr
                account.utxos_added_count = 1
                withUnsafeMutablePointer(to: &account) { accountPtr in
                    var cs = WalletChangeSetFFI()
                    cs.has_chain = true
                    cs.chain.has_synced_height = true
                    cs.chain.synced_height = Self.winnerHeight
                    cs.accounts = accountPtr
                    cs.accounts_count = 1
                    withUnsafePointer(to: &cs) { csPtr in
                        applied = handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                    }
                }
            }
            return applied
        }

        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent, "the funding output drained the tombstone before anything could collect it")
        XCTAssertEqual(coin.supersededByTxid, winnerTxid)
        XCTAssertTrue(try pendingRows(container).isEmpty, "the drain consumed the tombstone")
    }

    /// The release veto's stamp half: a coin held by a stamp naming a
    /// stored, network-final winner F whose bytes DO spend it stays spent
    /// when a later sweep of an unrelated loser names it released —
    /// upstream reporting its own amnesia about F.
    func testAReleaseNamingACoinAStoredFinalWinnerStampedIsRefused() throws {
        let (handler, container) = try makeHandler()
        let finalWinner = Data(repeating: 0x46, count: 32)
        let unrelatedLoser = Data(repeating: 0x48, count: 32)
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let funding = PersistentTransaction(
            txid: fundingTxid, transactionData: Data(repeating: 0x04, count: 10),
            context: 2, blockHeight: 100, netAmount: 100_000
        )
        context.insert(funding)
        context.insert(PersistentTransaction(
            txid: finalWinner,
            transactionData: serializedTransaction(inputs: [(txid: fundingTxid, vout: 0)]),
            context: 3, blockHeight: 120, netAmount: -100_000
        ))
        let coin = PersistentTxo(transaction: funding, vout: 0, amount: 100_000, address: "yFundAddr", height: 100)
        coin.walletId = walletId
        coin.isSpent = true
        coin.supersededByTxid = finalWinner
        context.insert(coin)
        context.insert(loserRow(txid: unrelatedLoser, spending: [(txid: fundingTxid, vout: 0)]))
        try context.save()

        sweep(handler, [Batch(
            losers: [unrelatedLoser], winner: winnerTxid, winnerMinedHeight: 400,
            released: [(txid: fundingTxid, vout: 0)]
        )])

        let held = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(held.isSpent, "a stored chainlocked spender of the coin refuses the release")
        XCTAssertEqual(held.supersededByTxid, finalWinner, "and keeps its attribution")
    }

    /// The veto's other half: a stamp naming a stored, network-final
    /// transaction whose bytes do NOT spend the coin does not refuse the
    /// release — the stamp was a global hold written by another loser's
    /// sweep, not a claim of that transaction's.
    func testAReleaseNamingACoinStampedWithAFinalTransactionThatDoesNotSpendItIsHonoured() throws {
        let (handler, container) = try makeHandler()
        let stampedWinner = Data(repeating: 0x46, count: 32)
        let unrelatedLoser = Data(repeating: 0x48, count: 32)
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let funding = PersistentTransaction(
            txid: fundingTxid, transactionData: Data(repeating: 0x04, count: 10),
            context: 2, blockHeight: 100, netAmount: 100_000
        )
        context.insert(funding)
        context.insert(PersistentTransaction(
            txid: stampedWinner,
            transactionData: serializedTransaction(inputs: [(txid: Data(repeating: 0x49, count: 32), vout: 0)]),
            context: 3, blockHeight: 120, netAmount: -100_000
        ))
        let coin = PersistentTxo(transaction: funding, vout: 0, amount: 100_000, address: "yFundAddr", height: 100)
        coin.walletId = walletId
        coin.isSpent = true
        coin.supersededByTxid = stampedWinner
        context.insert(coin)
        context.insert(loserRow(txid: unrelatedLoser, spending: [(txid: fundingTxid, vout: 0)]))
        try context.save()

        sweep(handler, [Batch(
            losers: [unrelatedLoser], winner: winnerTxid, winnerMinedHeight: 400,
            released: [(txid: fundingTxid, vout: 0)]
        )])

        let freed = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(freed.isSpent, "a stamp whose transaction does not spend the coin is no claim")
        XCTAssertNil(freed.supersededByTxid)
    }

    /// A released outpoint whose funding transaction is swept in the same
    /// round is deleted, never freed. Here the parent's record was lost
    /// (a `PersistentTxo` cannot outlive its transaction row, so the only
    /// claim left on the dead output is a pending row of this wallet's),
    /// and the release names that output: the claim is deleted with the
    /// batch rather than left behind as a claim on a coin that can never
    /// exist.
    func testAReleaseNamingAnOutputOfACoSweptParentDeletesItsClaimRatherThanFreeingIt() throws {
        let (handler, container) = try makeHandler()
        let parentTxid = Data(repeating: 0xD1, count: 32)
        let childTxid = Data(repeating: 0xD2, count: 32)
        let claimantTxid = Data(repeating: 0xD3, count: 32)
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        // The child's own claim on the parent's output, and a third
        // transaction's claim on the same dead output; the parent has no
        // row at all.
        let child = loserRow(txid: childTxid, spending: [(txid: parentTxid, vout: 0)], netAmount: -50_000)
        context.insert(child)
        let pOutpoint = PersistentTxo.makeOutpoint(txid: parentTxid, vout: 0)
        context.insert(PersistentPendingInput(
            outpoint: pOutpoint, inputIndex: 0, spendingTxid: childTxid, spendingTransaction: child, walletId: walletId
        ))
        context.insert(PersistentPendingInput(
            outpoint: pOutpoint, inputIndex: 0, spendingTxid: claimantTxid, spendingTransaction: nil, walletId: walletId
        ))
        try context.save()

        sweep(handler, [Batch(
            losers: [parentTxid, childTxid], winner: winnerTxid, winnerMinedHeight: 400,
            released: [(txid: parentTxid, vout: 0)]
        )])

        XCTAssertNil(transaction(container, txid: childTxid))
        XCTAssertTrue(
            try pendingRows(container, spentTxid: parentTxid).isEmpty,
            "no claim on a dead parent's output survives the batch, released or not"
        )
    }
}
