import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the spend-linkage half of the asset-lock restore:
/// `asset_lock_input_spends`, the evidence the conflict screen runs on at
/// app-launch catch-up.
///
/// At that moment the wallet's in-memory transaction history is empty, so a
/// lock whose input a different, confirmed transaction already took has no
/// other way to be recognised as dead — it sits in the full proof wait
/// instead. The rows restored here are the only source that works.
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
    /// `PersistentAssetLock.outPointHex` stores.
    private func outPointHex(txid: Data, vout: UInt32) -> String {
        let display = txid.reversed().map { String(format: "%02x", $0) }.joined()
        return "\(display):\(vout)"
    }

    /// Seed an unresolved asset lock spending the funding coin, plus a
    /// different confirmed transaction recorded as that coin's spender.
    ///
    /// `legacyTxoWalletId` is the whole point of the fixture: rows written
    /// before `PersistentTxo.walletId` existed carry an empty value, and the
    /// spend-reconciliation path sets `isSpent` and the spender link without
    /// backfilling it.
    private func seed(in container: ModelContainer, legacyTxoWalletId: Bool) throws {
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
            context: 2,
            blockHeight: 101,
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

    /// Drive the real load path and report how many spend-linkage rows the
    /// wallet's restore entry carries.
    private func restoredInputSpendCount(_ handler: PlatformWalletPersistenceHandler) -> Int {
        let loaded = handler.loadWalletList()
        XCTAssertFalse(loaded.errored, "the load must not fail")
        XCTAssertGreaterThan(loaded.count, 0, "the wallet must produce a restore entry")
        guard let entries = loaded.entries, loaded.count > 0 else { return -1 }
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entries)) }
        return Int(entries[0].asset_lock_input_spends_count)
    }

    /// The ordinary case: the TXO carries its wallet id, and the confirmed
    /// spender is reported so the conflict screen can act at startup.
    func testConfirmedSpenderOfALockInputIsRestored() throws {
        let (handler, container) = try makeHandler()
        try seed(in: container, legacyTxoWalletId: false)

        XCTAssertEqual(restoredInputSpendCount(handler), 1)
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
            restoredInputSpendCount(handler),
            1,
            "a legacy TXO resolving to this wallet through its account must not be discarded"
        )
    }
}
