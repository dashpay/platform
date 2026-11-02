import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the spender half of the asset-lock record restore: the
/// settled spender of an unresolved lock's input rides
/// `unresolved_asset_lock_tx_records`, the same array that restores the
/// locks' own funding records, and Rust re-inserts it into live
/// transaction history where the conflict screen scans it.
///
/// At app launch that history is otherwise empty, so a lock whose input a
/// different, confirmed transaction already took has no other way to be
/// recognised as dead — it sits in the full proof wait instead. Restoring
/// the spender as an ordinary record (not a snapshot) keeps the evidence
/// live: chainlock promotion and reorg demotion both reach it.
@MainActor
final class AssetLockInputSpendRestoreTests: XCTestCase {

    private let walletId = Data(repeating: 0x01, count: 32)
    /// The coin the tracked asset lock spends, and that a different
    /// transaction is recorded as having taken.
    private let fundingTxid = Data(repeating: 0x41, count: 32)
    private let fundingVout: UInt32 = 0
    private let lockTxid = Data(repeating: 0x42, count: 32)
    private let spenderTxid = Data(repeating: 0x43, count: 32)

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// Serialize a transaction spending `input`, in the form
    /// `TransactionDecoder` parses: a plain (non-special) version-2
    /// transaction with one empty-script input and one empty-script output.
    private func serializedSpend(of input: (txid: Data, vout: UInt32)) -> Data {
        var bytes = Data()
        bytes.append(contentsOf: withUnsafeBytes(of: UInt32(2).littleEndian) { Data($0) })
        bytes.append(0x01) // one input
        bytes.append(input.txid)
        bytes.append(contentsOf: withUnsafeBytes(of: input.vout.littleEndian) { Data($0) })
        bytes.append(0x00) // empty scriptSig
        bytes.append(contentsOf: [0xff, 0xff, 0xff, 0xff]) // sequence
        bytes.append(0x01) // one output
        bytes.append(contentsOf: withUnsafeBytes(of: UInt64(1_000).littleEndian) { Data($0) })
        bytes.append(0x00) // empty scriptPubKey
        bytes.append(contentsOf: [0x00, 0x00, 0x00, 0x00]) // locktime
        return bytes
    }

    /// `<txid hex, display order>:<vout>`, the form
    /// `PersistentAssetLock.outPointHex` stores — produced through the SDK's
    /// own encoder so the fixture cannot drift from the format the load path
    /// actually reads.
    private func outPointHex(txid: Data, vout: UInt32) -> String {
        var raw = Data(txid)
        withUnsafeBytes(of: vout.littleEndian) { raw.append(contentsOf: $0) }
        return PersistentAssetLock.encodeOutPoint(rawBytes: raw)
    }

    /// Seed an unresolved asset lock spending the funding coin, plus a
    /// different confirmed transaction recorded as that coin's spender.
    ///
    /// `legacyTxoWalletId` is the whole point of the fixture: rows written
    /// before `PersistentTxo.walletId` existed carry an empty value, and the
    /// spend-reconciliation path sets `isSpent` and the spender link without
    /// backfilling it.
    private func seed(
        in container: ModelContainer,
        legacyTxoWalletId: Bool,
        spenderContext: UInt32 = 2
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

        // The transaction that created the coin, and the coin itself.
        let funding = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100,
            netAmount: 100_000
        )
        context.insert(funding)

        // A different transaction, confirmed, recorded as having taken it.
        let spender = PersistentTransaction(
            txid: spenderTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: spenderContext,
            blockHeight: spenderContext >= 2 ? 101 : 0,
            netAmount: -100_000
        )
        context.insert(spender)

        let coin = PersistentTxo(
            transaction: funding,
            vout: fundingVout,
            amount: 100_000,
            address: "yFundAddr",
            height: 100
        )
        coin.account = account
        coin.walletId = legacyTxoWalletId ? Data() : walletId
        coin.isSpent = true
        coin.spendingTransaction = spender
        context.insert(coin)

        // The tracked lock: Built (statusRaw 0), spending the funding coin.
        let lock = PersistentAssetLock(
            outPointHex: outPointHex(txid: lockTxid, vout: 0),
            walletId: walletId,
            transactionBytes: serializedSpend(of: (txid: fundingTxid, vout: fundingVout)),
            fundingTypeRaw: 0,
            identityIndexRaw: 0,
            amountDuffs: 100_000,
            statusRaw: 0
        )
        context.insert(lock)

        try context.save()
    }

    /// Drive the real load path and report how many unresolved-lock tx
    /// records the wallet's restore entry carries. In these fixtures the
    /// lock's own txid has no `PersistentTransaction` row, so every entry
    /// counted here is a restored spender record.
    private func restoredRecordCount(_ handler: PlatformWalletPersistenceHandler) -> Int {
        let loaded = handler.loadWalletList()
        XCTAssertFalse(loaded.errored, "the load must not fail")
        XCTAssertGreaterThan(loaded.count, 0, "the wallet must produce a restore entry")
        guard let entries = loaded.entries, loaded.count > 0 else { return -1 }
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entries)) }
        return Int(entries[0].unresolved_asset_lock_tx_records_count)
    }

    /// The ordinary case: the TXO carries its wallet id, and the confirmed
    /// spender's record is restored so the conflict screen's history scan
    /// can act at startup.
    func testConfirmedSpenderOfALockInputIsRestored() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container, legacyTxoWalletId: false)

        XCTAssertEqual(restoredRecordCount(handler), 1)
    }

    /// The same coin on a row migrated from the older schema, where
    /// `walletId` was never backfilled. Comparing that column raw discards
    /// exactly these rows, which leaves the restored conflict map empty and
    /// sends startup back into the full proof wait this path exists to
    /// prevent — so ownership has to resolve through the account instead.
    func testConfirmedSpenderIsRestoredForALegacyTxoWithNoWalletId() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container, legacyTxoWalletId: true)

        XCTAssertEqual(
            restoredRecordCount(handler),
            1,
            "a legacy TXO resolving to this wallet through its account must not be discarded"
        )
    }

    /// The record payload is the cross-language contract, and a count
    /// assertion alone would let a wrong-source copy — bytes from the wrong
    /// transaction, a context read off the funding tx — ship green. Read
    /// the emitted entry back and pin its fields to the spender's values.
    func testRestoredSpenderRecordCarriesTheExactPayload() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container, legacyTxoWalletId: false)

        let loaded = handler.loadWalletList()
        XCTAssertFalse(loaded.errored, "the load must not fail")
        guard let entries = loaded.entries, loaded.count > 0 else {
            return XCTFail("the wallet must produce a restore entry")
        }
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entries)) }

        let entry = entries[0]
        XCTAssertEqual(Int(entry.unresolved_asset_lock_tx_records_count), 1)
        guard let rows = entry.unresolved_asset_lock_tx_records else {
            return XCTFail("a count of 1 must come with a row pointer")
        }
        let row = rows[0]
        XCTAssertEqual(
            Int(row.tx_bytes_len), 10,
            "the spender's consensus bytes, not the funding tx's (which the fixture sizes differently)"
        )
        XCTAssertEqual(row.context_raw, 2, "the spender's persisted context, verbatim")
        XCTAssertEqual(row.block_height, 101, "the spender's persisted block height")
    }

    /// A mempool-context spender is deliberately NOT restored: it can still
    /// be replaced, the screen ignores it, and shipping it would widen the
    /// restore surface for nothing — the same minimum-surface rule as the
    /// `statusRaw < 2` lock filter.
    func testAMempoolSpenderIsNotRestored() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container, legacyTxoWalletId: false, spenderContext: 0)

        XCTAssertEqual(restoredRecordCount(handler), 0)
    }
}
