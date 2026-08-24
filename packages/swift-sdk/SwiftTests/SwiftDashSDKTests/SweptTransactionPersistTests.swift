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
        let swept = PersistentTransaction(
            txid: sweptTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -140_000
        )
        context.insert(funding)
        context.insert(swept)

        let winner: PersistentTransaction?
        if winnerTakesA {
            let row = PersistentTransaction(
                txid: winnerTxid,
                transactionData: Data(repeating: 0x06, count: 10),
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
        handler.beginChangeset(walletId: walletId)
        let applied = handler.persistWalletChangesetSweeps(
            walletId: walletId,
            sweeps: UnsafePointer(sweeps),
            count: UInt(ffiBatches.count)
        )
        _ = handler.endChangeset(walletId: walletId, success: applied)
        return applied
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

    /// A re-delivery of the funding output — what a restore-rescan does,
    /// blind to the unconfirmed winner no block carries yet — must NOT
    /// outrank the sweep's verdict: the coin was provably consumed, and
    /// handing it back would resurrect it into the restore set on every
    /// restore-from-seed until the winner confirms. Only an explicit
    /// release frees a stamped hold — the same answer the SQLite store's
    /// upsert valve gives to the identical event stream.
    func testWalletReDeliveringAStampedHeldCoinKeepsItSpent() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)
        sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400)])
        XCTAssertTrue(txo(container, txid: fundingTxid, vout: 1)!.isSpent)

        redeliverCoinB(handler)

        let held = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(held.isSpent, "the stamped hold survives re-delivery")
        XCTAssertEqual(held.supersededByTxid, winnerTxid)
        XCTAssertNil(held.spendingTransaction)
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

        let l = PersistentTransaction(
            txid: sweptTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -100_000
        )
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

        let l = PersistentTransaction(
            txid: sweptTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -100_000
        )
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
        let reclaimer = PersistentTransaction(
            txid: secondLoser,
            transactionData: Data(repeating: 0x07, count: 10),
            context: 0,
            blockHeight: 0,
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

        let loser = PersistentTransaction(
            txid: loserTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
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
    /// fixture actually exercise the fix: without the `isGloballySwept`
    /// guard, `walletOwnsTransaction` finds `walletA` through
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

    /// The review finding, order 1: wallet B's callback — the one that
    /// releases nothing — runs first. Before the fix this alone deleted the
    /// shared loser row (nothing in the old code held it back), so wallet
    /// A's later release of P landed on the missing-row no-op and P stayed
    /// wrongly spent forever.
    func testSharedLoserAppliesBothWalletsReleaseSetsRegardlessOfOrder_BThenA() throws {
        let (handler, container) = try makeHandler()
        let loserTxid = Data(repeating: 0x81, count: 32)
        let winner = Data(repeating: 0x82, count: 32)
        let walletB = Data(repeating: 0x02, count: 32)
        try seedSharedLoserAcrossTwoWallets(
            in: container, walletA: walletId, walletB: walletB, loserTxid: loserTxid
        )

        // Wallet B first: its own released set names nothing, so its coin
        // (Q) is held rather than freed.
        sweep(handler, [Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400)], walletId: walletB)

        XCTAssertNotNil(
            transaction(container, txid: loserTxid),
            "wallet B alone must not delete a row wallet A still has a claim on"
        )
        let untouchedP = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(untouchedP.isSpent, "wallet B's callback must not touch wallet A's coin")
        XCTAssertNotNil(untouchedP.spendingTransaction, "P is still linked to the loser, untouched")

        // Wallet A second: its own released set names P.
        sweep(handler, [
            Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 0)])
        ], walletId: walletId)

        XCTAssertNil(
            transaction(container, txid: loserTxid),
            "the last wallet to run performs the delete"
        )

        let p = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(p.isSpent, "wallet A's own release must free its own coin")
        XCTAssertNil(p.spendingTransaction)

        let q = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(q.isSpent, "wallet B's earlier decision to hold Q must survive wallet A's callback")
        XCTAssertNil(q.spendingTransaction)
    }

    /// The review finding, order 2: wallet A — the one that releases P —
    /// runs first. The fix is meant to be order-independent, so this must
    /// land on the exact same end state as the B-then-A ordering above.
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

        XCTAssertNotNil(
            transaction(container, txid: loserTxid),
            "wallet A alone must not delete a row wallet B still has a claim on"
        )
        let untouchedQ = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertFalse(untouchedQ.isSpent, "wallet A's callback must not touch wallet B's coin")
        XCTAssertNotNil(untouchedQ.spendingTransaction, "Q is still linked to the loser, untouched")

        // Wallet B second: releases nothing.
        sweep(handler, [Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400)], walletId: walletB)

        XCTAssertNil(
            transaction(container, txid: loserTxid),
            "the last wallet to run performs the delete"
        )

        let p = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(p.isSpent, "wallet A's earlier release must survive wallet B's callback")
        XCTAssertNil(p.spendingTransaction)

        let q = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertTrue(q.isSpent, "wallet B's own decision to hold its coin must stick")
        XCTAssertNil(q.spendingTransaction)
    }

    /// The BLOCKING review finding: a shared loser's own output, and its
    /// reachability through `walletCoreTxids`, must not survive across a
    /// restart when only ONE wallet's callback ever commits and the other's
    /// never arrives at all — a crash, a rejection, or simply never coming.
    ///
    /// `commit_batch` calls `store()` once per wallet and each commits
    /// independently, so before the fix wallet B alone could not delete a
    /// row wallet A still had an outstanding claim on (see the
    /// `_BThenA`/`_AThenB` tests above) — and the OUTPUT went with the row,
    /// because deletion was the only thing that excluded either. If wallet
    /// A's own callback then never runs, that hold is permanent: the row,
    /// its phantom output, and its `involvedAccounts` link to wallet A all
    /// stay fully live forever, so `walletCoreTxids` hands the dead
    /// transaction back to wallet A as its own after every future restart.
    ///
    /// Only wallet B's callback ever runs here, and it releases nothing —
    /// the worst case, since it gives the row no reason to be physically
    /// deleted at all. The fix's global half must still make the output and
    /// the enumeration exclusion durable from that single callback alone.
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

            XCTAssertNotNil(
                transaction(container, txid: loserTxid),
                "wallet A's own claim on P is still outstanding, so the row itself survives"
            )
            XCTAssertNil(
                txo(container, txid: loserTxid, vout: 2),
                "the loser's own output must not survive even a single committed callback, "
                    + "regardless of which wallet's callback that was"
            )
            let row = try XCTUnwrap(transaction(container, txid: loserTxid))
            XCTAssertTrue(
                row.isGloballySwept,
                "any callback that reaches the sweep must flag the row, not just wallet A's own"
            )
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
        let (txidsA, erroredA) = handler.walletCoreTxids(walletId: walletId)
        XCTAssertFalse(erroredA)
        XCTAssertFalse(
            txidsA.contains { $0.txid == loserTxid },
            "wallet A must not be able to enumerate the swept loser as its own transaction "
                + "after a restart, even though it is still linked via involvedAccounts and "
                + "its own callback never ran"
        )
    }

    /// Cross-round reinstatement — the BLOCKING finding this round fixes.
    /// The sweep and its reinstating record land in two SEPARATE
    /// `persistWalletChangeset` rounds, with wallet B's still-outstanding
    /// claim keeping the shared row physically present in between, exactly
    /// as `testSharedLoserOutputAndEnumerationAreExcludedAfterOnlyOneWalletsCallbackCommits`
    /// establishes on its own. Before the fix, `upsertTransaction` bailed
    /// unconditionally on `isGloballySwept == true`, so round 2's record —
    /// upstream's newer word, per `CoreChangeSet::merge`'s documented
    /// IS-lock-precedence sequence (swept by an IS-locked conflict, then
    /// returns chainlocked and sweeps that conflict in turn) — would be
    /// silently discarded forever, and `upsertUtxo` would keep rejecting
    /// its output on the strength of a tombstone nothing could ever clear.
    /// Verified across a restart: the reinstatement has to be durable, not
    /// merely visible in the context that just applied it.
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
            // nothing. Wallet A's own claim on P (its funding coin) is still
            // outstanding, so the shared row survives physically even
            // though the global half of the sweep already tombstoned it and
            // deleted its phantom output.
            sweep(handler, [Batch(losers: [loserTxid], winner: winner, winnerMinedHeight: 400)], walletId: walletB)

            let tombstoned = try XCTUnwrap(transaction(container, txid: loserTxid))
            XCTAssertTrue(tombstoned.isGloballySwept, "sanity: the row is tombstoned after round 1")
            XCTAssertNil(
                txo(container, txid: loserTxid, vout: 2),
                "sanity: the loser's own output is gone after round 1"
            )

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
            XCTAssertFalse(
                reinstated.isGloballySwept,
                "a later record naming a tombstoned txid must clear the tombstone"
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

        let survived = try XCTUnwrap(transaction(container, txid: loserTxid))
        XCTAssertFalse(survived.isGloballySwept, "the reinstatement must survive a restart")
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

    /// Two wallets, each holding an unresolved *released* input on the same
    /// shared loser — the case where the row would otherwise never be
    /// reclaimed.
    ///
    /// Left attached, a released pending input reads as its wallet's claim
    /// in the ownership check, so A declines the delete because B's row is
    /// there and B declines because A's is: a stalemate no replay breaks.
    /// The dead transaction contributes no funds either way thanks to the
    /// global marker, so this is storage rather than balance — but the row
    /// and both pending entries would be kept forever.
    func testTwoWalletsReleasedPendingInputsDoNotDeadlockTheRowDelete() throws {
        let (handler, container) = try makeHandler()
        let walletB = Data(repeating: 0x02, count: 32)
        try seedSharedLoserAcrossTwoWallets(
            in: container, walletA: walletId, walletB: walletB, loserTxid: sweptTxid
        )

        // Each wallet has one pending input on the loser, and each will be
        // released by its own wallet's sweep.
        let context = ModelContext(container)
        let loserTxid = sweptTxid
        var descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == loserTxid }
        )
        descriptor.fetchLimit = 1
        let loser = try XCTUnwrap(try context.fetch(descriptor).first)
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

        sweep(handler, [
            Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 8)])
        ])
        sweep(
            handler,
            [Batch(losers: [sweptTxid], winner: winnerTxid, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 9)])],
            walletId: walletB
        )

        XCTAssertNil(
            transaction(container, txid: sweptTxid),
            "a released pending input is not a claim once its own wallet has resolved it"
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
            let swept = PersistentTransaction(
                txid: sweptTxid,
                transactionData: Data(repeating: 0x05, count: 10),
                context: 0,
                blockHeight: 0,
                netAmount: -100_000
            )
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
    /// original, older `createdAt`. The drain's newest-wins pick then
    /// selected the winner's ordinary row, took the gated branch (`isSpent`
    /// stays false until the winner confirms — never, for an IS-locked
    /// unconfirmed winner), skipped the `supersededByTxid` stamp, and
    /// deleted every pending row including the tombstone: the durable hold
    /// evaporated and the consumed coin re-entered the restore set.
    func testAWinnersOwnPendingRowDoesNotEvaporateTheSweepTombstone() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let outpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 0)

        // The doomed spend arrived before its funding output — parked as a
        // pending row, exactly what `resolveInputOutpoint` writes. Backdated
        // so the winner's row below is strictly newer, as it always is in
        // reality (the loser's record preceded the winner's by definition).
        let loser = PersistentTransaction(
            txid: sweptTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -100_000
        )
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
            transactionData: Data(repeating: 0x06, count: 10),
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

        let l = PersistentTransaction(
            txid: firstLoser,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -100_000
        )
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
        let w = PersistentTransaction(
            txid: secondLoser,
            transactionData: Data(repeating: 0x06, count: 10),
            context: 0,
            blockHeight: 0,
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

        let l = PersistentTransaction(
            txid: firstLoser,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -100_000
        )
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
        let w = PersistentTransaction(
            txid: secondLoser,
            transactionData: Data(repeating: 0x06, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -100_000
        )
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

        let c = PersistentTransaction(
            txid: childTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -50_000
        )
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
    /// nothing but in-memory mutation — must be visible to the second
    /// batch's scalar reconciliation. Pins the per-batch tombstone scan
    /// reading the mutable columns off live objects; a store-side predicate
    /// would test the stale saved values and miss the row entirely.
    func testChainedSweepAcrossTwoBatchesInOneRoundReleasesTheFreshTombstone() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let firstLoser = Data(repeating: 0xA1, count: 32) // L
        let secondLoser = Data(repeating: 0xA2, count: 32) // W — batch 1's winner
        let finalWinner = Data(repeating: 0xA3, count: 32) // X

        let l = PersistentTransaction(
            txid: firstLoser,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -50_000
        )
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
    /// hand the drain links `spendingTransaction` too, so the release DOES
    /// reach the row through `row.inputs` — but nothing cleared the marker,
    /// and a released coin keeping its dead winner's marker turns the next
    /// hold on this outpoint permanent (`upsertUtxo`'s recovery clear reads
    /// a present marker as a durable claim).
    func testAReleasedCoinDropsItsDeadWinnersMarker() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let firstLoser = Data(repeating: 0x91, count: 32) // L
        let secondLoser = Data(repeating: 0x92, count: 32) // W
        let finalWinner = Data(repeating: 0x93, count: 32) // X

        let l = PersistentTransaction(
            txid: firstLoser,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -50_000
        )
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

        // W's own record lands before the funding TXO does, so the drain
        // below links `spendingTransaction` as well as stamping the marker.
        let w = PersistentTransaction(
            txid: secondLoser,
            transactionData: Data(repeating: 0x06, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -50_000
        )
        context.insert(w)
        try context.save()

        deliverFundingUtxo(handler, vout: 0, amount: 50_000)

        let stamped = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(stamped.isSpent, "sanity: the drained claim holds the coin")
        XCTAssertEqual(stamped.supersededByTxid, secondLoser)

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

        let l = PersistentTransaction(
            txid: firstLoser,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -50_000
        )
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

    /// The multi-wallet continuation of the chained scenarios above — the
    /// review finding on the missing-row early return. A shared loser L
    /// spends one still-unfunded coin of wallet A's and two of wallet B's,
    /// so the first sweep leaves each wallet's claims as detached tombstones
    /// pointing at winner W. When W's own record then arrives,
    /// `resolveInputOutpoint`'s duplicate guard sees each `(outpoint, W)`
    /// tombstone and attaches nothing to W's row — so when W is swept in
    /// turn, wallet A's callback finds no other wallet's claim on the row
    /// and deletes it. Wallet B's independently committed callback then runs
    /// against a row that no longer exists, and before the fix returned
    /// without ever applying B's release decision: B's released coin would
    /// later come back spent by the obsolete W, and B's held coin stayed
    /// attributed to W, unable to follow any further sweep.
    func testSharedWinnerDeletedByAnotherWalletsCallbackStillReconcilesThisWalletsTombstones() throws {
        let (handler, container) = try makeHandler()
        let context = ModelContext(container)
        let walletB = Data(repeating: 0x02, count: 32)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentWallet(walletId: walletB, network: .testnet))

        let sharedLoser = Data(repeating: 0xC1, count: 32) // L
        let sharedWinner = Data(repeating: 0xC2, count: 32) // W
        let finalWinner = Data(repeating: 0xC3, count: 32) // X

        let l = PersistentTransaction(
            txid: sharedLoser,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
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

        // W's own record arrives, claiming all three outpoints. The
        // `(outpoint, W)` tombstones occupy the duplicate-guard key, so no
        // new pending relationship attaches to W's row — the premise that
        // lets wallet A's callback below delete it.
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

        // Second sweep: X beats W. Wallet A's callback runs first, releases
        // its own coin, and — finding no attached claim of any other
        // wallet's — deletes the shared row.
        sweep(handler, [
            Batch(losers: [sharedWinner], winner: finalWinner, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 0)])
        ], walletId: walletId)
        XCTAssertNil(
            transaction(container, txid: sharedWinner),
            "sanity: wallet A's callback deleted the shared winner row — the premise "
                + "wallet B's callback below has to survive"
        )

        // Wallet B's callback arrives after the row is gone, releasing one
        // of its two coins and holding the other.
        sweep(handler, [
            Batch(losers: [sharedWinner], winner: finalWinner, winnerMinedHeight: 400, released: [(txid: fundingTxid, vout: 2)])
        ], walletId: walletB)

        let heldOutpoint = PersistentTxo.makeOutpoint(txid: fundingTxid, vout: 1)
        let heldDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == heldOutpoint }
        )
        let heldTombstone = try XCTUnwrap(
            try context.fetch(heldDescriptor).first,
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
        let swept = PersistentTransaction(
            txid: loser,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            blockHeight: 0,
            netAmount: -100_000
        )
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
        spentTxid: Data? = nil
    ) throws -> [PersistentPendingInput] {
        let outpoint = PersistentTxo.makeOutpoint(txid: spentTxid ?? fundingTxid, vout: 0)
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
        // observed — the shape that would have become a tombstone.
        let unfundedTxid = Data(repeating: 0x77, count: 32)
        let context = ModelContext(container)
        let loserRow = try XCTUnwrap(transaction(container, txid: sweptTxid))
        context.insert(PersistentPendingInput(
            outpoint: PersistentTxo.makeOutpoint(txid: unfundedTxid, vout: 0),
            inputIndex: 2,
            spendingTxid: sweptTxid,
            spendingTransaction: loserRow,
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
}
