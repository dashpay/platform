import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the selection rule behind `unconfirmed_outgoing_tx_records`.
///
/// The spend effect of an unconfirmed outgoing send is never persisted —
/// `spendIsInBlock` withholds `isSpent` from an input whose spender is only
/// in the mempool, because that sighting is reversible by eviction. The UTXO
/// restore therefore hands the input back as spendable, and unless the send
/// is replayed at load the balance re-counts the coin. For a send that never
/// reached the network there is nothing to re-observe, so it stays wrong.
///
/// What these tests pin down is *which* rows may be offered for that replay.
/// The rule is driven from the TXO side on purpose: a send is offered only
/// while one of our own outputs still names it as its spender and is itself
/// still unspent. That makes liveness fall out for free — a send that lost a
/// conflict has had its input flipped by the winner and drops out on its own,
/// which matters because the FFI restore never rebuilds `observed_spent` and
/// Rust cannot make that judgement for itself.
@MainActor
final class UnconfirmedOutgoingSendRestoreTests: XCTestCase {

    private let walletId = Data(repeating: 0x07, count: 32)
    private let fundingTxid = Data(repeating: 0x51, count: 32)
    private let sendTxid = Data(repeating: 0x52, count: 32)
    private let rivalTxid = Data(repeating: 0x53, count: 32)
    private let fundingVout: UInt32 = 0

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// A plain version-2 transaction spending `input` — the shape
    /// `TransactionDecoder` parses, so the fixture cannot drift from what the
    /// load path actually reads.
    private func serializedSpend(of input: (txid: Data, vout: UInt32)) -> Data {
        var bytes = Data()
        bytes.append(contentsOf: withUnsafeBytes(of: UInt32(2).littleEndian) { Data($0) })
        bytes.append(0x01)
        bytes.append(input.txid)
        bytes.append(contentsOf: withUnsafeBytes(of: input.vout.littleEndian) { Data($0) })
        bytes.append(0x00)
        bytes.append(contentsOf: [0xff, 0xff, 0xff, 0xff])
        bytes.append(0x01)
        bytes.append(contentsOf: withUnsafeBytes(of: UInt64(9_000).littleEndian) { Data($0) })
        bytes.append(0x00)
        bytes.append(contentsOf: [0x00, 0x00, 0x00, 0x00])
        return bytes
    }

    /// One funded coin, and one transaction recorded as spending it.
    ///
    /// - `sendContext`/`sendHeight`: the spender's settlement state.
    /// - `inputStillOurs`: whether the coin still points at that spender and
    ///   is still unspent — `false` models a send that lost a conflict, where
    ///   the winning spender flipped the row.
    /// - `legacyTxoWalletId`: a row migrated from the schema that never
    ///   backfilled `walletId`, whose ownership resolves through the account.
    private func seed(
        in container: ModelContainer,
        sendContext: UInt32 = 0,
        sendHeight: UInt32 = 0,
        inputStillOurs: Bool = true,
        legacyTxoWalletId: Bool = false
    ) throws {
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet,
            accountType: 0,
            accountIndex: 0,
            accountTypeName: "Standard"
        )
        // A wallet only reaches the restore path with at least one account
        // carrying an xpub — that is what Rust rebuilds the watch-only
        // wallet from.
        account.accountExtendedPubKeyBytes = Data(repeating: 0x30, count: 78)
        context.insert(account)

        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x01, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 10_000
        )
        context.insert(funding)

        let send = PersistentTransaction(
            txid: sendTxid,
            transactionData: serializedSpend(of: (txid: fundingTxid, vout: fundingVout)),
            context: sendContext,
            blockHeight: sendHeight,
            netAmount: -10_000
        )
        context.insert(send)

        let coin = PersistentTxo(
            transaction: funding,
            vout: fundingVout,
            amount: 10_000,
            address: "yFundAddr",
            height: 100
        )
        coin.account = account
        coin.walletId = legacyTxoWalletId ? Data() : walletId

        if inputStillOurs {
            coin.isSpent = false
            coin.spendingTransaction = send
        } else {
            // A different, settled transaction took the coin first. The
            // winner's flip is what retires our send.
            let rival = PersistentTransaction(
                txid: rivalTxid,
                transactionData: Data(repeating: 0x02, count: 10),
                context: 2,
                blockHeight: 101,
                netAmount: -10_000
            )
            context.insert(rival)
            coin.isSpent = true
            coin.spendingTransaction = rival
        }
        context.insert(coin)

        try context.save()
    }

    /// Drive the real load path and report how many sends were offered.
    private func offeredCount(_ handler: PlatformWalletPersistenceHandler) -> Int {
        let loaded = handler.loadWalletList()
        XCTAssertFalse(loaded.errored, "the load must not fail")
        guard let entries = loaded.entries, loaded.count > 0 else { return -1 }
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entries)) }
        return Int(entries[0].unconfirmed_outgoing_tx_records_count)
    }

    /// The case the fix exists for: an unconfirmed send whose input is still
    /// ours and still unspent is offered for replay.
    func testUnconfirmedSendIsOffered() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container)

        XCTAssertEqual(offeredCount(handler), 1)
    }

    /// A send that already lost a conflict must never be offered: replaying
    /// it would re-spend a coin this wallet no longer owns, and
    /// re-dispatching it would put a dead transaction back on the wire.
    /// Nothing else can catch this — the FFI restore does not rebuild
    /// `observed_spent`.
    func testSendThatLostAConflictIsNotOffered() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container, inputStillOurs: false)

        XCTAssertEqual(
            offeredCount(handler),
            0,
            "the winning spender flipped the input; our send is dead and must not be replayed"
        )
    }

    /// A settled send needs no replay: the chain already carries the spend,
    /// and the ordinary restore path reconstructs it.
    func testConfirmedSendIsNotOffered() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container, sendContext: 2, sendHeight: 101)

        XCTAssertEqual(offeredCount(handler), 0)
    }

    /// A row migrated from the older schema, where `walletId` was never
    /// backfilled. Comparing that column raw would discard exactly these
    /// rows and silently leave the balance wrong for the wallets most likely
    /// to be carrying history — ownership has to resolve through the account,
    /// which is why this pass consumes the caller's bucketed rows rather than
    /// running its own `walletId` query.
    func testLegacyTxoWithNoWalletIdIsStillOffered() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container, legacyTxoWalletId: true)

        XCTAssertEqual(
            offeredCount(handler),
            1,
            "a legacy TXO resolving to this wallet through its account must not be discarded"
        )
    }
}
