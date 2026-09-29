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
            inputs: [spent], ownedOutputAmounts: [], allOutputsOwned: true, previousDirection: 2, isAssetLock: true
        )
        XCTAssertEqual(lock?.netAmount, -100)
        XCTAssertEqual(lock?.direction, 2)
        XCTAssertEqual(PersistentTransaction.reconciledAccounting(
            inputs: [spent], ownedOutputAmounts: [99], allOutputsOwned: false, previousDirection: 3, isAssetLock: false
        )?.direction, 3)
    }

    /// Same case table as the Rust repair's
    /// `should_classify_repaired_direction_like_the_swift_sdk`.
    func testShouldClassifyDirectionLikeTheRustRepair() {
        let spent = input(100)
        // (spends ours, owned output amounts, all outputs owned, asset lock, previous direction, expected)
        let cases: [(Bool, [UInt64], Bool, Bool, UInt32, UInt32)] = [
            (true, [99], true, false, 0, 2),
            (true, [40], false, false, 0, 1),
            (true, [], true, false, 0, 1),
            (true, [], true, true, 0, 2),
            (true, [40], true, true, 0, 2),
            (true, [], false, true, 0, 1),
            (false, [40], true, false, 0, 0),
            (true, [99], true, false, 3, 3),
        ]
        for (index, (spendsOurs, owned, allOwned, isLock, previous, expected)) in cases.enumerated() {
            let result = PersistentTransaction.reconciledAccounting(
                inputs: spendsOurs ? [spent] : [], ownedOutputAmounts: owned,
                allOutputsOwned: allOwned, previousDirection: previous, isAssetLock: isLock
            )
            XCTAssertEqual(result?.direction, expected, "case \(index)")
        }
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
    /// `burn` makes the single output an OP_RETURN, as an asset lock's is.
    private func serializedSpend(inputs: [Data], outputValue: UInt64 = 40, burn: Bool = false) -> Data {
        var bytes = Data([2, 0, 0, 0, UInt8(inputs.count)])
        for txid in inputs {
            bytes.append(txid)
            bytes.append(contentsOf: [0, 0, 0, 0, 0, 255, 255, 255, 255])
        }
        bytes.append(1)
        withUnsafeBytes(of: outputValue.littleEndian) { bytes.append(contentsOf: $0) }
        bytes.append(contentsOf: burn ? [1, 0x6a] : [0])
        bytes.append(contentsOf: [0, 0, 0, 0])
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

    func testShouldNotCommitOrDropOpenRoundWhenLoadRunsMidChangeset() throws {
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
        let identityId = Data(repeating: 9, count: 32)
        let fetchIdentity = {
            try ModelContext(container).fetch(FetchDescriptor<PersistentIdentity>()).first { $0.identityId == identityId }
        }

        handler.beginChangeset(walletId: walletId)
        handler.persistIdentities(
            walletId: walletId,
            upserts: [.init(
                identityId: identityId, balance: 100, revision: 1, identityIndex: 0, label: nil,
                status: 0, walletId: walletId, dpnsNames: [], dashpayProfile: nil, contactProfiles: []
            )],
            removed: []
        )
        XCTAssertFalse(handler.loadWalletList().errored)
        XCTAssertNil(try fetchIdentity(), "load must not commit half of an open round")
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        XCTAssertNotNil(try fetchIdentity(), "load must not roll back an open round")

        // The skipped full pass runs on the next load outside any round.
        XCTAssertFalse(handler.loadWalletList().errored)
        let repaired = try ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>())
            .first { $0.txid == spender.txid }
        XCTAssertEqual(repaired?.netAmount, -100)
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
        let bytes = serializedSpend(inputs: [walletId, Data(repeating: 2, count: 32)], outputValue: 0, burn: true)
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
                bytes: serializedSpend(inputs: [walletId], outputValue: 0, burn: true), kind: 6, inputTxids: [walletId])
        let row = try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == spenderId })
        XCTAssertEqual(row.netAmount, -100)
        XCTAssertEqual(row.direction, 2)
        XCTAssertTrue(row.isAssetLock)
    }

    func testShouldNotReportStoredAmountWhileInputsArePending() {
        let walletId = Data(repeating: 1, count: 32)
        let tx = PersistentTransaction(txid: Data(repeating: 3, count: 32), transactionData: Data(), netAmount: 40)
        let change = PersistentTxo(transaction: tx, vout: 0, amount: 40, address: "", height: 1)
        change.walletId = walletId
        tx.outputs = [change]
        XCTAssertEqual(tx.netAmount(for: walletId), 40)
        tx.pendingInputs = [PersistentPendingInput(
            outpoint: Data(repeating: 9, count: 36), inputIndex: 0,
            spendingTxid: tx.txid, spendingTransaction: tx, walletId: walletId
        )]
        XCTAssertNil(tx.netAmount(for: walletId), "a missing input makes the stored amount provisional")
    }

    func testShouldNotCountAnotherLocalWalletsUnlinkedOutputForTheSender() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let senderId = Data(repeating: 1, count: 32)
        let receiver = PersistentWallet(walletId: Data(repeating: 2, count: 32), network: .testnet)
        let receiverAccount = PersistentAccount(
            wallet: receiver, accountType: 0, accountIndex: 0, accountTypeName: "Standard BIP44 Account"
        )
        // P2PKH to pubkey hash 0x05 x 20 on testnet: B's address with no TXO row yet.
        let receiverAddress = PersistentCoreAddress(
            address: "yLmzEvw3frCPS4cyRmFFeKbt64fUPzMwFh", poolTypeTag: 0, addressIndex: 0, derivationPath: ""
        )
        receiverAddress.account = receiverAccount
        context.insert(PersistentWallet(walletId: senderId, network: .testnet))
        context.insert(receiver)
        context.insert(receiverAccount)
        context.insert(receiverAddress)
        var bytes = Data([2, 0, 0, 0, 1])
        bytes.append(senderId)
        bytes.append(contentsOf: [0, 0, 0, 0, 0, 255, 255, 255, 255, 1])
        withUnsafeBytes(of: UInt64(40).littleEndian) { bytes.append(contentsOf: $0) }
        bytes.append(contentsOf: [25, 0x76, 0xa9, 0x14] + [UInt8](repeating: 5, count: 20) + [0x88, 0xac])
        bytes.append(contentsOf: [0, 0, 0, 0])
        let spender = PersistentTransaction(
            txid: Data(repeating: 3, count: 32), transactionData: bytes, direction: 1, netAmount: -100
        )
        let coin = input(100)
        coin.spendingTransaction = spender
        context.insert(coin)
        context.insert(spender)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertFalse(handler.loadWalletList().errored)
        let row = try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == spender.txid })
        XCTAssertEqual(row.netAmount, -100, "the receiving wallet's credit is not the sender's")
        XCTAssertEqual(row.netAmount(for: senderId), -100)
    }

    func testShouldExcludePersistedContactOutputsFromOwnedAccounting() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let walletId = Data(repeating: 1, count: 32)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        let contactAccount = PersistentAccount(
            wallet: wallet, accountType: PlatformWalletPersistenceHandler.dashpayExternalAccountTypeTag,
            accountIndex: 0, accountTypeName: "DashPay External Account"
        )
        context.insert(wallet)
        context.insert(contactAccount)
        let coin = input(100)
        let spender = PersistentTransaction(
            txid: Data(repeating: 3, count: 32), transactionData: serializedSpend(inputs: [walletId]),
            direction: 2, netAmount: -60
        )
        coin.spendingTransaction = spender
        let contactOutput = PersistentTxo(transaction: spender, vout: 0, amount: 40, address: "", height: 1)
        contactOutput.walletId = walletId
        contactOutput.account = contactAccount
        context.insert(coin)
        context.insert(spender)
        context.insert(contactOutput)
        try context.save()
        XCTAssertEqual(spender.netAmount(for: walletId), -100)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertFalse(handler.loadWalletList().errored)
        let row = try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == spender.txid })
        XCTAssertEqual(row.netAmount, -100)
        XCTAssertEqual(row.direction, 1)
        XCTAssertEqual(row.netAmount(for: walletId), -100)
    }

    func testShouldNotTreatPersistedContactInputAsOurFunding() throws {
        let container = try DashModelContainer.createInMemory()
        let context = container.mainContext
        let walletId = Data(repeating: 1, count: 32)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        let contactAccount = PersistentAccount(
            wallet: wallet, accountType: PlatformWalletPersistenceHandler.dashpayExternalAccountTypeTag,
            accountIndex: 0, accountTypeName: "DashPay External Account"
        )
        context.insert(wallet)
        context.insert(contactAccount)
        let coin = input(100)
        coin.account = contactAccount
        let spender = PersistentTransaction(
            txid: Data(repeating: 3, count: 32), transactionData: serializedSpend(inputs: [walletId]),
            direction: 0, netAmount: 40
        )
        coin.spendingTransaction = spender
        let received = PersistentTxo(transaction: spender, vout: 0, amount: 40, address: "", height: 1)
        received.walletId = walletId
        context.insert(coin)
        context.insert(spender)
        context.insert(received)
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertFalse(handler.loadWalletList().errored)
        let row = try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTransaction>()).first { $0.txid == spender.txid })
        XCTAssertEqual(row.netAmount, 40)
        XCTAssertEqual(row.direction, 0)
        XCTAssertEqual(row.netAmount(for: walletId), 40)
    }

}
