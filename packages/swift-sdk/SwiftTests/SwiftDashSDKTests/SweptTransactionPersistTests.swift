import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the one subtractive part of the changeset path:
/// `WalletChangeSetFFI.swept`.
///
/// A swept transaction was a recorded spend that a later, final transaction
/// provably beat to one of its inputs, so it can never confirm and Rust has
/// already dropped it. Every other field on that struct is additive, so a
/// mirror that ignores this one keeps the dead row, hands it back at the
/// next load, and re-creates a balance the wallet has already corrected —
/// the bug the upstream sweep exists to fix, one layer up.
///
/// The fixtures model the shape that makes the coins tricky: an unconfirmed
/// loser — upstream sweeps nothing else — spends A and B, and the winner
/// takes only A. Because the loser never reached a block, this store never
/// flipped `isSpent` on either coin, so both are one deleted row away from
/// re-entering the restore set.
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
    @discardableResult
    private func sweep(
        _ handler: PlatformWalletPersistenceHandler,
        _ pairs: [(loser: Data, winner: Data)]
    ) -> Bool {
        var entries: [SweptTransactionFFI] = []
        for pair in pairs {
            var entry = SweptTransactionFFI()
            Swift.withUnsafeMutableBytes(of: &entry.txid) { dst in
                pair.loser.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            Swift.withUnsafeMutableBytes(of: &entry.superseded_by) { dst in
                pair.winner.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            entries.append(entry)
        }

        handler.beginChangeset(walletId: walletId)
        let applied = entries.withUnsafeMutableBufferPointer { buf -> Bool in
            var cs = WalletChangeSetFFI()
            cs.swept = buf.baseAddress
            cs.swept_count = UInt(buf.count)
            return withUnsafePointer(to: &cs) { csPtr in
                handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
            }
        }
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

        sweep(handler, [(loser: sweptTxid, winner: winnerTxid)])

        XCTAssertNil(transaction(container, txid: sweptTxid), "the swept row is gone")
        XCTAssertNil(txo(container, txid: sweptTxid, vout: 0), "the change it created is gone with it")
        XCTAssertNotNil(transaction(container, txid: fundingTxid), "the funding transaction is untouched")
    }

    /// With the winner in the store, releasing what still points at the
    /// loser frees exactly the loser's own input: the winner re-pointed the
    /// shared one at itself earlier in the round.
    func testSweepFreesOnlyTheInputsTheWinnerDidNotTake() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)

        sweep(handler, [(loser: sweptTxid, winner: winnerTxid)])

        let takenByWinner = txo(container, txid: fundingTxid, vout: 0)
        XCTAssertNotNil(takenByWinner)
        XCTAssertTrue(takenByWinner!.isSpent, "the coin the winner took stays spent")
        XCTAssertEqual(takenByWinner!.spendingTransaction?.txid, winnerTxid)

        let losersOwn = txo(container, txid: fundingTxid, vout: 1)
        XCTAssertNotNil(losersOwn)
        XCTAssertFalse(losersOwn!.isSpent, "the loser's own input is free again")
        XCTAssertNil(losersOwn!.spendingTransaction)
    }

    /// A winner that pays only to outside addresses sweeps the loser without
    /// ever being recorded here. The loser was unconfirmed, so both coins it
    /// named are linked to it and unspent — deleting it and stopping there
    /// would return the one the chain already spent to the restore set. With
    /// nothing to tell the two apart, both are held instead.
    func testSweepByAnIrrelevantWinnerHoldsTheInputsOutOfTheRestoreSet() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)

        sweep(handler, [(loser: sweptTxid, winner: winnerTxid)])

        XCTAssertNil(transaction(container, txid: sweptTxid), "the swept row still goes")
        for vout: UInt32 in [0, 1] {
            let coin = txo(container, txid: fundingTxid, vout: vout)
            XCTAssertNotNil(coin)
            XCTAssertTrue(
                coin!.isSpent,
                "a coin the unrecorded winner may have consumed must not come back"
            )
            XCTAssertNil(coin!.spendingTransaction, "and no spender is invented for it")
        }
    }

    /// The wallet decides which of the held coins are actually free: it
    /// re-delivers them as UTXOs after a rescan, and that lifts the hold.
    /// Without this the conservative branch above would be permanent.
    func testWalletReDeliveringAHeldCoinFreesIt() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: false)
        sweep(handler, [(loser: sweptTxid, winner: winnerTxid)])
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

    /// A txid the store has never seen is not an error: sweeps are
    /// idempotent, and a round can name a transaction this mirror never
    /// recorded in the first place.
    func testSweepingAnUnknownTransactionIsANoOp() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container, winnerTakesA: true)

        let applied = sweep(
            handler,
            [(loser: Data(repeating: 0x99, count: 32), winner: winnerTxid)]
        )

        XCTAssertTrue(applied, "an absent row is a successful no-op, not a failed round")
        XCTAssertNotNil(transaction(container, txid: sweptTxid))
        XCTAssertNotNil(transaction(container, txid: fundingTxid))
    }
}
