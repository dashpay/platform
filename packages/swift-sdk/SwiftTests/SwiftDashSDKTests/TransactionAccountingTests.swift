import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

@MainActor
final class TransactionAccountingTests: XCTestCase {
    private func input(_ value: UInt64, wallet: UInt8 = 1) -> PersistentTxo {
        let parent = PersistentTransaction(txid: Data(repeating: wallet, count: 32), transactionData: Data())
        let txo = PersistentTxo(transaction: parent, vout: 0, amount: value, address: "", height: 1)
        txo.walletId = Data(repeating: wallet, count: 32)
        return txo
    }

    func testShouldRepairSpentInputsWithoutChangingBalanceOrConsumption() {
        let tx = PersistentTransaction(txid: Data(repeating: 3, count: 32), transactionData: Data(), netAmount: 40)
        let spent = input(100)
        spent.isSpent = true
        let result = PersistentTransaction.reconciledAccounting(
            inputs: [spent], ownedOutputAmounts: [40], allOutputsOwned: false, previousDirection: 0, isAssetLock: false
        )
        XCTAssertEqual(result?.netAmount, -60)
        XCTAssertEqual(result?.direction, 1)
        XCTAssertTrue(spent.isSpent)
        XCTAssertEqual(tx.netAmount, 40) // Computing accounting has no storage side effects.
    }

    func testShouldKeepInternalAndCoinJoinSemanticsWithNegativeNet() {
        let spent = input(100)
        XCTAssertEqual(PersistentTransaction.reconciledAccounting(
            inputs: [spent], ownedOutputAmounts: [99], allOutputsOwned: true, previousDirection: 0, isAssetLock: false
        )?.direction, 2)
        let lock = PersistentTransaction.reconciledAccounting(
            inputs: [spent], ownedOutputAmounts: [], allOutputsOwned: false, previousDirection: 2, isAssetLock: true
        )
        XCTAssertEqual(lock?.netAmount, -100)
        XCTAssertEqual(lock?.direction, 2)
        XCTAssertEqual(PersistentTransaction.reconciledAccounting(
            inputs: [spent], ownedOutputAmounts: [99], allOutputsOwned: false, previousDirection: 3, isAssetLock: false
        )?.direction, 3)
    }

    func testShouldRejectOverflowInsteadOfWrappingHistory() {
        XCTAssertNil(PersistentTransaction.reconciledAccounting(
            inputs: [input(UInt64.max)], ownedOutputAmounts: [], allOutputsOwned: false, previousDirection: 0, isAssetLock: false
        ))
    }

    func testShouldScopeNetToWalletAndDeduplicateOutpoints() {
        let tx = PersistentTransaction(txid: Data(repeating: 3, count: 32), transactionData: Data())
        let first = input(100)
        let other = input(200, wallet: 2)
        tx.inputs = [first, first, other]
        XCTAssertEqual(tx.netAmount(for: first.walletId), -100)
        XCTAssertEqual(tx.netAmount(for: other.walletId), -200)
    }
    private func serializedSpend(inputs: [Data], outputValue: UInt64 = 40) -> Data {
        var bytes = Data([2, 0, 0, 0, UInt8(inputs.count)])
        for txid in inputs {
            bytes.append(txid)
            bytes.append(contentsOf: [0, 0, 0, 0, 0, 255, 255, 255, 255])
        }
        bytes.append(1)
        withUnsafeBytes(of: outputValue.littleEndian) { bytes.append(contentsOf: $0) }
        bytes.append(contentsOf: [0, 0, 0, 0, 0])
        return bytes
    }

    func testShouldBackfillExistingHistoryOnLoadAndRemainIdempotent() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let walletId = Data(repeating: 1, count: 32)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let funding = PersistentTransaction(txid: walletId, transactionData: Data())
        let spender = PersistentTransaction(
            txid: Data(repeating: 3, count: 32), transactionData: serializedSpend(inputs: [walletId]),
            direction: 0, netAmount: 40
        )
        let coin = PersistentTxo(transaction: funding, vout: 0, amount: 100, address: "", height: 1)
        coin.walletId = walletId
        coin.isSpent = true
        coin.spendingTransaction = spender
        context.insert(funding)
        context.insert(spender)
        context.insert(coin)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertFalse(handler.loadWalletList().errored)
        XCTAssertFalse(handler.loadWalletList().errored)
        let fresh = ModelContext(container)
        let repaired = try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == spender.txid })
        XCTAssertEqual(repaired.netAmount, -100)
        XCTAssertEqual(repaired.direction, 1)
        XCTAssertTrue(try XCTUnwrap(fresh.fetch(FetchDescriptor<PersistentTxo>()).first).isSpent)
    }

    func testShouldPreserveAccountingWhenSomePrevoutsAreMissing() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let walletId = Data(repeating: 1, count: 32)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let coin = input(100)
        let spender = PersistentTransaction(
            txid: Data(repeating: 3, count: 32),
            transactionData: serializedSpend(inputs: [walletId, Data(repeating: 2, count: 32)]),
            direction: 1, netAmount: -200
        )
        coin.spendingTransaction = spender
        context.insert(coin)
        context.insert(spender)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertFalse(handler.loadWalletList().errored)
        let rows = try ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>())
        XCTAssertEqual(rows.first { $0.txid == spender.txid }?.netAmount, -200)
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

    func testShouldRepairLateInputAndLateOutputInSeparateAtomicRounds() throws {
        let container = try DashModelContainer.createInMemory()
        let walletId = Data(repeating: 1, count: 32)
        container.mainContext.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try container.mainContext.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let spenderId = Data(repeating: 3, count: 32)
        persist(handler, walletId: walletId, txid: spenderId,
                bytes: serializedSpend(inputs: [walletId]), net: 40, inputTxids: [walletId])
        persist(handler, walletId: walletId, txid: walletId, outputs: [(walletId, 100)])
        persist(handler, walletId: walletId, txid: spenderId, outputs: [(spenderId, 40)])
        let context = ModelContext(container)
        let row = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == spenderId })
        XCTAssertEqual(row.netAmount, -60)
        XCTAssertEqual(row.direction, 2)
        XCTAssertEqual(row.inputs.count, 1)
        XCTAssertEqual(row.outputs.count, 1)
        XCTAssertTrue(try XCTUnwrap(row.inputs.first).isSpent)
    }

    func testShouldPreserveFundedAssetLockAccountingDuringSyntheticReplayWithMissingPrevout() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let walletId = Data(repeating: 1, count: 32)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        let spenderId = Data(repeating: 3, count: 32)
        let bytes = serializedSpend(inputs: [walletId, Data(repeating: 2, count: 32)], outputValue: 0)
        let spender = PersistentTransaction(txid: spenderId, transactionData: bytes, direction: 2, netAmount: -200)
        spender.transactionTypeKind = 6
        let coin = input(100)
        coin.spendingTransaction = spender
        context.insert(coin)
        context.insert(spender)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        persist(handler, walletId: walletId, txid: spenderId, bytes: bytes, kind: 6, inputTxids: [walletId])
        let row = try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == spenderId })
        XCTAssertEqual(row.netAmount, -200)
        XCTAssertEqual(row.direction, 2)
        XCTAssertEqual(row.context, 2)
    }

    func testShouldRepairNoChangeAssetLockToFullCoreDebit() throws {
        let container = try DashModelContainer.createInMemory()
        let walletId = Data(repeating: 1, count: 32)
        container.mainContext.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try container.mainContext.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let spenderId = Data(repeating: 3, count: 32)
        persist(handler, walletId: walletId, txid: walletId, outputs: [(walletId, 100)])
        persist(handler, walletId: walletId, txid: spenderId,
                bytes: serializedSpend(inputs: [walletId], outputValue: 0), kind: 6, inputTxids: [walletId])
        let row = try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == spenderId })
        XCTAssertEqual(row.netAmount, -100)
        XCTAssertEqual(row.direction, 2)
        XCTAssertTrue(row.isAssetLock)
    }

}
