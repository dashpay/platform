import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the one subtractive part of the changeset path:
/// `WalletChangeSetFFI.swept_txids`.
///
/// A swept transaction was a recorded spend that a later, final
/// transaction provably beat to one of its inputs, so it can never
/// confirm and Rust has already dropped it. Every other field on that
/// struct is additive, so a mirror that ignores this one keeps the dead
/// row, hands it back at the next load, and re-creates a balance the
/// wallet has already corrected — the bug the upstream sweep exists to
/// fix, one layer up.
@MainActor
final class SweptTransactionPersistTests: XCTestCase {

    private let walletId = Data(repeating: 0x01, count: 32)
    private let fundingTxid = Data(repeating: 0x41, count: 32)
    private let sweptTxid = Data(repeating: 0x42, count: 32)

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// Seed the shape a confirmed spend leaves behind: a funding
    /// transaction with one output, a spending transaction that claimed
    /// it (linked and flagged spent), and the change that spend created.
    private func seedSpend(in container: ModelContainer) throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))

        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 100_000
        )
        let swept = PersistentTransaction(
            txid: sweptTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 2,
            blockHeight: 101,
            netAmount: -40_000
        )
        context.insert(funding)
        context.insert(swept)

        let fundedOutput = PersistentTxo(
            transaction: funding,
            vout: 0,
            amount: 100_000,
            address: "yFundAddr",
            height: 100
        )
        fundedOutput.walletId = walletId
        fundedOutput.isSpent = true
        fundedOutput.spendingTransaction = swept
        context.insert(fundedOutput)

        let change = PersistentTxo(
            transaction: swept,
            vout: 0,
            amount: 60_000,
            address: "yChangeAddr",
            height: 101
        )
        change.walletId = walletId
        context.insert(change)

        try context.save()
    }

    /// Drive one changeset round that sweeps `sweptTxid`, through the same
    /// entry point the Rust persister calls.
    private func sweep(_ handler: PlatformWalletPersistenceHandler, txids: [Data]) {
        var raw: [(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                   UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                   UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                   UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)] = []
        for txid in txids {
            var tuple = (UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0),
                         UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0),
                         UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0),
                         UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0), UInt8(0))
            withUnsafeMutableBytes(of: &tuple) { dst in
                txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            raw.append(tuple)
        }

        handler.beginChangeset(walletId: walletId)
        raw.withUnsafeMutableBufferPointer { buf in
            var cs = WalletChangeSetFFI()
            cs.swept_txids = buf.baseAddress
            cs.swept_txids_count = UInt(buf.count)
            withUnsafePointer(to: &cs) { csPtr in
                handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
            }
        }
        _ = handler.endChangeset(walletId: walletId, success: true)
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

    /// The row and everything it created go; the funding transaction and
    /// its coin stay.
    func testSweptTransactionAndItsOutputsAreDeleted() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container)

        sweep(handler, txids: [sweptTxid])

        XCTAssertNil(transaction(container, txid: sweptTxid), "the swept row is gone")
        XCTAssertNil(txo(container, txid: sweptTxid, vout: 0), "the change it created is gone with it")
        XCTAssertNotNil(transaction(container, txid: fundingTxid), "the funding transaction is untouched")
    }

    /// The coin the swept transaction claimed becomes spendable again.
    /// Left as-is it would be marked spent by a transaction that no longer
    /// exists — invisible to the wallet and to the restore set, which is
    /// the same lost-funds shape as the phantom balance, inverted.
    func testSweepReleasesTheSpendClaimOnItsInputs() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container)

        sweep(handler, txids: [sweptTxid])

        let funded = txo(container, txid: fundingTxid, vout: 0)
        XCTAssertNotNil(funded)
        XCTAssertFalse(funded!.isSpent, "the claim died with the transaction that made it")
        XCTAssertNil(funded!.spendingTransaction)
    }

    /// A txid the store has never seen is not an error: sweeps are
    /// idempotent, and a round can name a transaction this mirror never
    /// recorded in the first place.
    func testSweepingAnUnknownTransactionIsANoOp() throws {
        let (handler, container) = try makeHandler()
        try seedSpend(in: container)

        sweep(handler, txids: [Data(repeating: 0x99, count: 32)])

        XCTAssertNotNil(transaction(container, txid: sweptTxid))
        XCTAssertNotNil(transaction(container, txid: fundingTxid))
    }
}
