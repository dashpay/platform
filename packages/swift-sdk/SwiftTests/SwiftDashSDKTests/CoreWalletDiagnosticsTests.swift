import Foundation
import SwiftData
import XCTest
@testable import SwiftDashSDK

/// Regression coverage for the diagnostic that identifies #4438: a sent
/// transaction consumes a CoinJoin output and pays change back to an address
/// owned by the wallet's BIP44 account, but the owned output is absent from
/// SwiftData. The same test exercises the complete structured-log line so a
/// future field addition cannot accidentally expose wallet material.
@MainActor
final class CoreWalletDiagnosticsTests: XCTestCase {
    private static let fixtureHex =
        "01000000011111111111111111111111111111111111111111111111111111111111111111"
        + "030000006a4730303030303030303030303030303030303030303030303030303030303030"
        + "30303030303030303030303030303030303030303030303030303030303030303030303030"
        + "303030210324653eac434488002cc06bbfb7f10fe18991e35f9fe4302dbea6d2353dc0ab1c"
        + "ffffffff02204e0200000000001976a91414db4138d56a2ecfb10881a9be394d9f321985b2"
        + "88ac0000000000000000066a04aaaaaaaa00000000"

    private static let fixtureAddress = "yNDj28QBMm5sY6bLjFcNdWRNef24KLQNuQ"
    private static let fixtureTxidDisplay =
        "bf7479216e5ba76f60bf11654c881824c6f9cdbb64eebe332cf835a3391cb5d5"

    private let walletId = Data(repeating: 0xa1, count: 32)

    private var fixtureData: Data {
        var data = Data()
        var index = Self.fixtureHex.startIndex
        while index < Self.fixtureHex.endIndex {
            let next = Self.fixtureHex.index(index, offsetBy: 2)
            data.append(UInt8(Self.fixtureHex[index..<next], radix: 16)!)
            index = next
        }
        return data
    }

    private func temporaryDirectory() throws -> URL {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
            "CoreWalletDiagnosticsTests-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: directory) }
        return directory
    }

    private func logLines(in session: URL, event: String) throws -> [String] {
        SDKLogger.flush()
        let log = try String(
            contentsOf: session.appendingPathComponent("swift/run.log"),
            encoding: .utf8
        )
        return log.split(separator: "\n").map(String.init).filter {
            $0.contains("event=\(event) ")
        }
    }

    private struct Fixture {
        let handler: PlatformWalletPersistenceHandler
        let context: ModelContext
        let spendingTransaction: PersistentTransaction
        let bip44Account: PersistentAccount
        let bip44Address: PersistentCoreAddress
        let decoded: DecodedTransaction
    }

    private func makeMissingOwnedOutputFixture() throws -> Fixture {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.autosaveEnabled = false
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet
        )

        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)

        let bip44 = PersistentAccount(
            wallet: wallet,
            accountType: 0,
            accountIndex: 0,
            accountTypeName: "Standard"
        )
        bip44.standardTag = 0
        context.insert(bip44)

        let coinJoin = PersistentAccount(
            wallet: wallet,
            accountType: 1,
            accountIndex: 0,
            accountTypeName: "CoinJoin"
        )
        context.insert(coinJoin)

        let address = PersistentCoreAddress(
            address: Self.fixtureAddress,
            poolTypeTag: 1,
            addressIndex: 4,
            derivationPath: "privacy-fixture-path"
        )
        address.account = bip44
        context.insert(address)

        // This is the output that the decoded fixture spends (11…11:3).
        // Empty consensus bytes keep it out of the transaction decoder while
        // preserving the real ownership relation used by the audit.
        let funding = PersistentTransaction(
            txid: Data(repeating: 0x11, count: 32),
            transactionData: Data(),
            context: 2,
            blockHeight: 100,
            netAmount: 151_072
        )
        context.insert(funding)
        let coinJoinTxo = PersistentTxo(
            transaction: funding,
            vout: 3,
            amount: 151_072,
            address: "coinjoin-input-address",
            scriptPubKey: Data([0x51]),
            height: 100
        )
        coinJoinTxo.account = coinJoin
        coinJoinTxo.walletId = walletId
        coinJoinTxo.isConfirmed = true
        context.insert(coinJoinTxo)

        let decoded = try TransactionDecoder.decode(fixtureData, network: .testnet)
        let spending = PersistentTransaction(
            txid: decoded.txid,
            transactionData: fixtureData,
            context: 2,
            blockHeight: 101,
            direction: 1,
            netAmount: -151_072
        )
        spending.involvedAccounts.append(coinJoin)
        coinJoinTxo.spendingTransaction = spending
        coinJoinTxo.isSpent = true
        context.insert(spending)

        try context.save()
        return Fixture(
            handler: handler,
            context: context,
            spendingTransaction: spending,
            bip44Account: bip44,
            bip44Address: address,
            decoded: decoded
        )
    }

    func testCoinJoinSpendWithMissingBip44ChangeDetects4438AndLogIsPrivate() throws {
        let fixture = try makeMissingOwnedOutputFixture()
        let session = try temporaryDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))

        XCTAssertNotNil(fixture.handler.emitCoreWalletDatabaseDiagnostics(
            walletId: walletId,
            checkpoint: .preExport
        ))

        let summaries = try logLines(in: session, event: "core_owned_output_audit_summary")
        let summary = try XCTUnwrap(summaries.last)
        XCTAssertTrue(summary.contains("candidate_transaction_count=1"), summary)
        XCTAssertTrue(summary.contains("coinjoin_to_bip44_missing_count=1"), summary)
        XCTAssertTrue(summary.contains("coinjoin_to_bip44_missing_value_duffs=151072"), summary)
        XCTAssertTrue(summary.contains("owned_bip44_output_count=1"), summary)
        XCTAssertTrue(summary.contains("persisted_valid_count=0"), summary)
        XCTAssertTrue(summary.contains("total_anomaly_count=1"), summary)

        let anomalies = try logLines(in: session, event: "core_owned_output_anomaly")
        let anomaly = try XCTUnwrap(anomalies.last)
        XCTAssertTrue(anomaly.contains(#"reason="missing_txo""#), anomaly)

        // Assert privacy over every line generated by the complete snapshot,
        // not just over one hand-constructed formatter input.
        SDKLogger.flush()
        let completeLog = try String(
            contentsOf: session.appendingPathComponent("swift/run.log"),
            encoding: .utf8
        )
        XCTAssertFalse(completeLog.contains(Self.fixtureAddress))
        XCTAssertFalse(completeLog.contains(Self.fixtureTxidDisplay))
        XCTAssertFalse(completeLog.contains(Self.fixtureHex))
        XCTAssertFalse(completeLog.contains("privacy-fixture-path"))
        let rawTxidHex = fixture.decoded.txid.map { String(format: "%02x", $0) }.joined()
        let reversedTxidHex = fixture.decoded.txid.reversed().map {
            String(format: "%02x", $0)
        }.joined()
        let scriptHex = fixture.decoded.outputs[0].scriptPubkey.map {
            String(format: "%02x", $0)
        }.joined()
        let rawOutpointHex = PersistentTxo.makeOutpoint(
            txid: fixture.decoded.txid,
            vout: 0
        ).map { String(format: "%02x", $0) }.joined()
        XCTAssertFalse(completeLog.contains(rawTxidHex))
        XCTAssertFalse(completeLog.contains(reversedTxidHex))
        XCTAssertFalse(completeLog.contains(scriptHex))
        XCTAssertFalse(completeLog.contains(rawOutpointHex))
        XCTAssertFalse(completeLog.contains(walletId.map { String(format: "%02x", $0) }.joined()))
        XCTAssertFalse(completeLog.contains(Data(repeating: 0x11, count: 32).map {
            String(format: "%02x", $0)
        }.joined()))
    }

    func testPersistedBip44ChangeClears4438Alarm() throws {
        let fixture = try makeMissingOwnedOutputFixture()
        let output = fixture.decoded.outputs[0]
        let change = PersistentTxo(
            transaction: fixture.spendingTransaction,
            vout: 0,
            amount: output.valueDuffs,
            address: try XCTUnwrap(output.address),
            scriptPubKey: output.scriptPubkey,
            height: fixture.spendingTransaction.blockHeight
        )
        change.account = fixture.bip44Account
        change.coreAddress = fixture.bip44Address
        change.walletId = walletId
        change.isConfirmed = true
        fixture.context.insert(change)
        try fixture.context.save()

        let session = try temporaryDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))
        XCTAssertNotNil(fixture.handler.emitCoreWalletDatabaseDiagnostics(
            walletId: walletId,
            checkpoint: .preExport
        ))

        let summaries = try logLines(in: session, event: "core_owned_output_audit_summary")
        let summary = try XCTUnwrap(summaries.last)
        XCTAssertTrue(summary.contains("coinjoin_to_bip44_missing_count=0"), summary)
        XCTAssertTrue(summary.contains("owned_bip44_output_count=1"), summary)
        XCTAssertTrue(summary.contains("persisted_valid_count=1"), summary)
        XCTAssertTrue(summary.contains("total_anomaly_count=0"), summary)
        XCTAssertTrue(try logLines(in: session, event: "core_owned_output_anomaly").isEmpty)
    }

    func testRestoreOnlyLogsLightweightBufferSnapshotAndNoDeepStartupEvents() throws {
        let fixture = try makeMissingOwnedOutputFixture()
        fixture.bip44Account.accountExtendedPubKeyBytes = Data(repeating: 0x02, count: 78)

        let validTransaction = PersistentTransaction(
            txid: Data(repeating: 0x22, count: 32),
            transactionData: Data(),
            context: 2,
            blockHeight: 102,
            netAmount: 100
        )
        fixture.context.insert(validTransaction)
        let validTxo = PersistentTxo(
            transaction: validTransaction,
            vout: 0,
            amount: 100,
            address: "valid-restore-address",
            scriptPubKey: Data([0x51]),
            height: 102
        )
        validTxo.account = fixture.bip44Account
        validTxo.walletId = walletId
        validTxo.isConfirmed = true
        fixture.context.insert(validTxo)

        let missingAccountTransaction = PersistentTransaction(
            txid: Data(repeating: 0x33, count: 32),
            transactionData: Data(),
            context: 2,
            blockHeight: 103,
            netAmount: 200
        )
        fixture.context.insert(missingAccountTransaction)
        let missingAccountTxo = PersistentTxo(
            transaction: missingAccountTransaction,
            vout: 0,
            amount: 200,
            address: "missing-account-restore-address",
            scriptPubKey: Data([0x52]),
            height: 103
        )
        missingAccountTxo.walletId = walletId
        missingAccountTxo.isConfirmed = true
        fixture.context.insert(missingAccountTxo)
        try fixture.context.save()

        let session = try temporaryDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))

        let result = fixture.handler.loadWalletList()
        XCTAssertFalse(result.errored)
        XCTAssertEqual(result.count, 1)
        let entries = try XCTUnwrap(result.entries)
        defer { fixture.handler.loadWalletListFree(entries: UnsafeRawPointer(entries)) }

        let snapshots = try logLines(in: session, event: "core_restore_buffer_snapshot")
        let snapshot = try XCTUnwrap(snapshots.last)
        XCTAssertTrue(snapshot.contains("candidate_count=2"), snapshot)
        XCTAssertTrue(snapshot.contains("candidate_value_duffs=300"), snapshot)
        XCTAssertTrue(snapshot.contains("emitted_count=1"), snapshot)
        XCTAssertTrue(snapshot.contains("emitted_value_duffs=100"), snapshot)
        XCTAssertTrue(snapshot.contains("skipped_missing_account_count=1"), snapshot)
        XCTAssertTrue(snapshot.contains(#"checkpoint="restore_buffer""#), snapshot)
        XCTAssertFalse(snapshot.contains("fingerprint"), snapshot)

        SDKLogger.flush()
        let completeLog = try String(
            contentsOf: session.appendingPathComponent("swift/run.log"),
            encoding: .utf8
        )
        let deepStartupEvents = [
            "core_db_wallet_snapshot",
            "core_db_account_snapshot",
            "core_db_anomaly_summary",
            "core_db_txo_anomaly",
            "core_owned_output_audit_summary",
            "core_owned_output_anomaly",
            "asset_lock_db_snapshot",
            "shielded_store_snapshot",
            "core_memory_account_snapshot",
            "core_db_memory_diff_summary",
            "core_db_memory_diff",
            "asset_lock_memory_snapshot",
            "asset_lock_db_memory_diff_summary",
        ]
        for event in deepStartupEvents {
            XCTAssertFalse(completeLog.contains("event=\(event) "), event)
        }
    }
}
