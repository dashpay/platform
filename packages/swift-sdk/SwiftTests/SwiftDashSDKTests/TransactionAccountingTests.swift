import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// The Swift store persists the per-wallet accounting `rs-platform-wallet`
/// emits and displays it; it never derives it. The rule itself is tested in
/// Rust (`platform_wallet::changeset::wallet_accounting`).
@MainActor
final class TransactionAccountingTests: XCTestCase {
    private let walletA = Data(repeating: 1, count: 32)
    private let walletB = Data(repeating: 2, count: 32)
    private let txid = Data(repeating: 3, count: 32)

    private func makeStore() throws -> (ModelContainer, PlatformWalletPersistenceHandler) {
        let container = try DashModelContainer.createInMemory()
        container.mainContext.insert(PersistentWallet(walletId: walletA, network: .testnet))
        container.mainContext.insert(PersistentWallet(walletId: walletB, network: .testnet))
        try container.mainContext.save()
        return (container, PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet))
    }

    private func row(_ txid: Data, net: Int64, direction: UInt32, fee: UInt64? = nil) -> TransactionAccountingFFI {
        var row = TransactionAccountingFFI()
        withUnsafeMutableBytes(of: &row.txid) { $0.copyBytes(from: txid) }
        row.net_amount = net
        row.direction = direction
        row.fee = fee ?? 0
        row.has_fee = fee != nil
        return row
    }

    /// Hand the handler `rows` the way Rust's extension slot does: inside a
    /// round, or on their own like the load-time backfill.
    @discardableResult
    private func account(
        _ handler: PlatformWalletPersistenceHandler, walletId: Data,
        _ rows: [TransactionAccountingFFI], inRound: Bool = true
    ) -> Bool {
        if inRound { handler.beginChangeset(walletId: walletId) }
        let stored = rows.withUnsafeBufferPointer {
            handler.persistTransactionAccounting(walletId: walletId, rows: $0.baseAddress, count: UInt($0.count))
        }
        if inRound { XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true)) }
        return stored
    }

    private func transaction(_ container: ModelContainer, _ txid: Data) throws -> PersistentTransaction {
        try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == txid })
    }

    private func persist(
        _ handler: PlatformWalletPersistenceHandler, walletId: Data, txid: Data,
        bytes: Data? = nil, net: Int64 = 0, kind: UInt8 = 0,
        inputTxids: [Data] = [], outputs: [(Data, UInt64)] = []
    ) {
        let name = strdup("Standard { index: 0 }")
        let address = strdup("")
        defer { free(name); free(address) }
        var record = TransactionRecordFFI()
        withUnsafeMutableBytes(of: &record.txid) { $0.copyBytes(from: txid) }
        record.context = 2
        record.net_amount = net
        record.transaction_type_kind = kind
        var inputs = inputTxids.map { txid -> OutPointFFI in
            var result = OutPointFFI()
            withUnsafeMutableBytes(of: &result.txid) { $0.copyBytes(from: txid) }
            return result
        }
        var txos = outputs.map { txid, amount -> UtxoEntryFFI in
            var result = UtxoEntryFFI()
            withUnsafeMutableBytes(of: &result.outpoint.txid) { $0.copyBytes(from: txid) }
            result.amount = amount
            result.address = address
            return result
        }
        var raw = Array(bytes ?? Data())
        handler.beginChangeset(walletId: walletId)
        raw.withUnsafeMutableBufferPointer { rawPtr in
            inputs.withUnsafeMutableBufferPointer { inputPtr in
                txos.withUnsafeMutableBufferPointer { txoPtr in
                    record.tx_data = rawPtr.baseAddress
                    record.tx_data_len = UInt(rawPtr.count)
                    record.input_outpoints = inputPtr.baseAddress
                    record.input_outpoints_count = UInt(inputPtr.count)
                    withUnsafeMutablePointer(to: &record) { recordPtr in
                        var account = AccountChangeSetFFI()
                        account.account_type_name = name
                        account.transactions = recordPtr
                        account.transactions_count = bytes == nil ? 0 : 1
                        account.utxos_added = txoPtr.baseAddress
                        account.utxos_added_count = UInt(txoPtr.count)
                        withUnsafeMutablePointer(to: &account) { accountPtr in
                            var changeset = WalletChangeSetFFI()
                            changeset.accounts = accountPtr
                            changeset.accounts_count = 1
                            withUnsafePointer(to: &changeset) { ptr in
                                XCTAssertTrue(handler.persistWalletChangeset(walletId: walletId, changeset: ptr))
                            }
                        }
                    }
                }
            }
        }
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    func testShouldStoreEachWalletsEmittedAccountingSeparately() throws {
        let (container, handler) = try makeStore()
        persist(handler, walletId: walletA, txid: txid, bytes: Data([1]), net: -100)
        persist(handler, walletId: walletB, txid: txid, bytes: Data([1]), net: 90)
        account(handler, walletId: walletA, [row(txid, net: -100, direction: CoreDirectionCode.outgoing, fee: 10)])
        account(handler, walletId: walletB, [row(txid, net: 90, direction: CoreDirectionCode.incoming)])

        let stored = try transaction(container, txid)
        XCTAssertEqual(stored.walletAccounting.count, 2)
        XCTAssertEqual(stored.netAmount(for: walletA), -100)
        XCTAssertEqual(stored.direction(for: walletA), CoreDirectionCode.outgoing)
        XCTAssertEqual(stored.fee(for: walletA), 10)
        XCTAssertEqual(stored.netAmount(for: walletB), 90)
        XCTAssertEqual(stored.direction(for: walletB), CoreDirectionCode.incoming)
        XCTAssertNil(stored.fee(for: walletB), "a wallet that funded nothing knows no fee")
        XCTAssertEqual(stored.displayFormattedAmount(for: walletB), "+0.00000090 DASH")
    }

    func testShouldUpdateTheWalletsRowInsteadOfAddingAnother() throws {
        let (container, handler) = try makeStore()
        persist(handler, walletId: walletA, txid: txid, bytes: Data([1]), net: 40)
        account(handler, walletId: walletA, [row(txid, net: 40, direction: CoreDirectionCode.incoming)])
        account(handler, walletId: walletA, [row(txid, net: -60, direction: CoreDirectionCode.outgoing)])

        let stored = try transaction(container, txid)
        XCTAssertEqual(stored.walletAccounting.count, 1)
        XCTAssertEqual(stored.netAmount(for: walletA), -60)
        XCTAssertEqual(stored.direction(for: walletA), CoreDirectionCode.outgoing)
    }

    /// The load-time backfill arrives outside any round and commits itself.
    func testShouldCommitAccountingDeliveredOutsideARound() throws {
        let (container, handler) = try makeStore()
        persist(handler, walletId: walletA, txid: txid, bytes: Data([1]), net: 40)

        XCTAssertTrue(account(handler, walletId: walletA,
                              [row(txid, net: -60, direction: CoreDirectionCode.outgoing)], inRound: false))

        XCTAssertEqual(try transaction(container, txid).netAmount(for: walletA), -60)
    }

    func testShouldIgnoreAccountingForATransactionTheStoreDoesNotHold() throws {
        let (container, handler) = try makeStore()

        XCTAssertTrue(account(handler, walletId: walletA, [row(txid, net: 5, direction: CoreDirectionCode.incoming)]))

        XCTAssertTrue(try ModelContext(container).fetch(FetchDescriptor<PersistentTransactionAccounting>()).isEmpty)
    }

    /// A record Rust re-emits without accounting (an asset lock restored from
    /// its bytes, promoted by a chain lock) overwrites only the shared
    /// scalar; the wallet's row, which Rust did not resend, still answers.
    func testShouldKeepTheWalletsRowWhenARecordArrivesWithoutAccounting() throws {
        let (container, handler) = try makeStore()
        let assetLock = TransactionTypeKind.assetLock.rawValue
        persist(handler, walletId: walletA, txid: txid, bytes: Data([1]), net: -100, kind: assetLock)
        account(handler, walletId: walletA, [row(txid, net: -100, direction: CoreDirectionCode.internalTransfer)])
        persist(handler, walletId: walletA, txid: txid, bytes: Data([1]), net: 0, kind: assetLock)

        let stored = try transaction(container, txid)
        XCTAssertEqual(stored.netAmount, 0, "the shared scalar is written as delivered")
        XCTAssertEqual(stored.netAmount(for: walletA), -100)
        XCTAssertEqual(stored.direction(for: walletA), CoreDirectionCode.internalTransfer)
    }

    /// Without a row for the wallet, display falls back to the shared scalar.
    func testShouldFallBackToTheSharedValuesWithoutAWalletRow() {
        let tx = PersistentTransaction(
            txid: txid, transactionData: Data(), direction: CoreDirectionCode.incoming, netAmount: 25
        )
        XCTAssertEqual(tx.netAmount(for: walletA), 25)
        XCTAssertEqual(tx.direction(for: walletA), CoreDirectionCode.incoming)
        XCTAssertEqual(tx.displayNetAmount(for: nil), 25)
    }

    func testShouldOfferNoHistoryWhenEveryTransactionHasTheWalletsRow() throws {
        let (_, handler) = try makeStore()
        persist(handler, walletId: walletA, txid: txid, bytes: Data([1, 2]), net: 40)
        account(handler, walletId: walletA, [row(txid, net: 40, direction: CoreDirectionCode.incoming)])

        let history = handler.loadCoreHistoryForAccounting(walletId: walletA)

        XCTAssertFalse(history.errored)
        XCTAssertNil(history.rows)
        XCTAssertEqual(history.count, 0)
    }

    /// Rust needs the whole history to resolve a flagged spend's inputs, so
    /// every transaction crosses, flagged by whether it lacks a row.
    func testShouldOfferTheWholeHistoryFlaggingTransactionsWithoutARow() throws {
        let (_, handler) = try makeStore()
        let other = Data(repeating: 4, count: 32)
        persist(handler, walletId: walletA, txid: txid, bytes: Data([1, 2]), net: 40)
        persist(handler, walletId: walletA, txid: other, bytes: Data([7, 8, 9]), net: -40)
        persist(handler, walletId: walletB, txid: Data(repeating: 5, count: 32), bytes: Data([5]), net: 1)
        account(handler, walletId: walletA, [row(txid, net: 40, direction: CoreDirectionCode.incoming)])

        let history = handler.loadCoreHistoryForAccounting(walletId: walletA)

        XCTAssertFalse(history.errored)
        let rows = try XCTUnwrap(history.rows)
        let offered = (0..<history.count).map { index -> (Data, Bool) in
            let entry = rows[index]
            return (Data(bytes: entry.tx_bytes!, count: Int(entry.tx_bytes_len)), entry.needs_accounting)
        }
        XCTAssertEqual(offered.count, 2, "only wallet A's transactions")
        XCTAssertEqual(offered.first { $0.0 == Data([1, 2]) }?.1, false)
        XCTAssertEqual(offered.first { $0.0 == Data([7, 8, 9]) }?.1, true)
        handler.loadCoreHistoryForAccountingFree(rows: UnsafeRawPointer(rows))
    }

    func testShouldDeleteOnlyTheDepartingWalletsRows() throws {
        let (container, handler) = try makeStore()
        persist(handler, walletId: walletA, txid: txid, bytes: Data([1]), net: -100)
        persist(handler, walletId: walletB, txid: txid, bytes: Data([1]), net: 90)
        account(handler, walletId: walletA, [row(txid, net: -100, direction: CoreDirectionCode.outgoing)])
        account(handler, walletId: walletB, [row(txid, net: 90, direction: CoreDirectionCode.incoming)])

        try handler.deleteWalletData(walletId: walletA)

        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentTransactionAccounting>())
        XCTAssertEqual(rows.map(\.walletId), [walletB])
        XCTAssertEqual(rows.first?.netAmount, 90)
    }

    func testShouldFormatSignedDuffsWithoutTrappingOnInt64Min() {
        XCTAssertEqual(PersistentTransaction.format(duffs: 150_000_000), "+1.50000000 DASH")
        XCTAssertEqual(PersistentTransaction.format(duffs: -100), "-0.00000100 DASH")
        XCTAssertTrue(PersistentTransaction.format(duffs: .min).hasPrefix("-92233720368."))
    }
}
