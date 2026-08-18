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
    /// attributed to, and the coins it freed.
    private struct Batch {
        var losers: [Data]
        var winner: Data
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
            Batch(losers: [sweptTxid], winner: winnerTxid, released: [(txid: fundingTxid, vout: 1)])
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
            Batch(losers: [sweptTxid], winner: winnerTxid, released: [(txid: fundingTxid, vout: 1)])
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
            Batch(losers: [sweptTxid], winner: winnerTxid, released: [(txid: fundingTxid, vout: 1)])
        ])

        XCTAssertNil(transaction(container, txid: sweptTxid), "the swept row still goes")

        let takenByWinner = txo(container, txid: fundingTxid, vout: 0)
        XCTAssertNotNil(takenByWinner)
        XCTAssertTrue(
            takenByWinner!.isSpent,
            "a coin the chain has already spent must not come back"
        )
        XCTAssertNil(takenByWinner!.spendingTransaction, "and no spender is invented for it")

        let losersOwn = txo(container, txid: fundingTxid, vout: 1)
        XCTAssertNotNil(losersOwn)
        XCTAssertFalse(
            losersOwn!.isSpent,
            "the loser's own input is free, winner record or not"
        )
    }

    /// A coin held spent with no spender is not a dead end: the wallet is
    /// the authority on what it holds, so re-delivering the coin as a UTXO —
    /// what a rescan does — lifts the mark. This is the backstop for a sweep
    /// that released nothing, and for any older row left in that state.
    func testWalletReDeliveringAHeldCoinFreesIt() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)
        sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid)])
        XCTAssertTrue(txo(container, txid: fundingTxid, vout: 1)!.isSpent)

        redeliverCoinB(handler)

        let freed = txo(container, txid: fundingTxid, vout: 1)
        XCTAssertNotNil(freed)
        XCTAssertFalse(freed!.isSpent, "the wallet holds it as a UTXO, so the hold is lifted")
        XCTAssertNil(freed!.spendingTransaction)
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
            Batch(losers: [sweptTxid], winner: winnerTxid, released: [(txid: fundingTxid, vout: 1)]),
            // Its winner consumed B, so this batch frees nothing.
            Batch(losers: [secondLoser], winner: Data(repeating: 0x56, count: 32)),
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
        sweep(handler, [Batch(losers: [loserTxid], winner: winner)], walletId: walletB)

        XCTAssertNotNil(
            transaction(container, txid: loserTxid),
            "wallet B alone must not delete a row wallet A still has a claim on"
        )
        let untouchedP = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertFalse(untouchedP.isSpent, "wallet B's callback must not touch wallet A's coin")
        XCTAssertNotNil(untouchedP.spendingTransaction, "P is still linked to the loser, untouched")

        // Wallet A second: its own released set names P.
        sweep(handler, [
            Batch(losers: [loserTxid], winner: winner, released: [(txid: fundingTxid, vout: 0)])
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
            Batch(losers: [loserTxid], winner: winner, released: [(txid: fundingTxid, vout: 0)])
        ], walletId: walletId)

        XCTAssertNotNil(
            transaction(container, txid: loserTxid),
            "wallet A alone must not delete a row wallet B still has a claim on"
        )
        let untouchedQ = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 1))
        XCTAssertFalse(untouchedQ.isSpent, "wallet A's callback must not touch wallet B's coin")
        XCTAssertNotNil(untouchedQ.spendingTransaction, "Q is still linked to the loser, untouched")

        // Wallet B second: releases nothing.
        sweep(handler, [Batch(losers: [loserTxid], winner: winner)], walletId: walletB)

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
            sweep(handler, [Batch(losers: [loserTxid], winner: winner)], walletId: walletB)

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
            sweep(handler, [Batch(losers: [loserTxid], winner: winner)], walletId: walletB)

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

        let applied = sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid)])

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

        let applied = sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid)])

        XCTAssertFalse(applied, "a genuinely failed wallet lookup must fail the round")
    }

    /// A txid the store has never seen is not an error: sweeps are
    /// idempotent, and a round can name a transaction this mirror never
    /// recorded in the first place.
    func testSweepingAnUnknownTransactionIsANoOp() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)

        let applied = sweep(handler, [
            Batch(losers: [Data(repeating: 0x99, count: 32)], winner: winnerTxid)
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

            sweep(handler, [Batch(losers: [sweptTxid], winner: winnerTxid)])

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
        sweep(handler, [Batch(losers: [firstLoser], winner: secondLoser)])

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
            Batch(losers: [secondLoser], winner: finalWinner, released: [(txid: fundingTxid, vout: 0)])
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
        sweep(handler, [Batch(losers: [firstLoser], winner: secondLoser)])

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
        sweep(handler, [Batch(losers: [secondLoser], winner: finalWinner)])

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
        sweep(handler, [Batch(losers: [sharedLoser], winner: sharedWinner)], walletId: walletId)
        sweep(handler, [Batch(losers: [sharedLoser], winner: sharedWinner)], walletId: walletB)
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
            Batch(losers: [sharedWinner], winner: finalWinner, released: [(txid: fundingTxid, vout: 0)])
        ], walletId: walletId)
        XCTAssertNil(
            transaction(container, txid: sharedWinner),
            "sanity: wallet A's callback deleted the shared winner row — the premise "
                + "wallet B's callback below has to survive"
        )

        // Wallet B's callback arrives after the row is gone, releasing one
        // of its two coins and holding the other.
        sweep(handler, [
            Batch(losers: [sharedWinner], winner: finalWinner, released: [(txid: fundingTxid, vout: 2)])
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
}
