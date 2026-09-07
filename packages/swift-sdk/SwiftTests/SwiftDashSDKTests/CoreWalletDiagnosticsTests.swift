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

    /// These tests assert over the complete `run.log`; a backlog buffered by
    /// an earlier suite must not be replayed into it.
    override func setUp() async throws {
        SDKLogger.resetForTesting()
    }

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

    func testCoinJoinSpendWithMissingBip44ChangeDetects4438AndLogIsPrivate() async throws {
        let fixture = try makeMissingOwnedOutputFixture()
        let session = try temporaryDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))

        let databaseSnapshot = await fixture.handler.emitCoreWalletDatabaseDiagnostics(walletId: walletId)
        XCTAssertNotNil(databaseSnapshot)

        let summaries = try logLines(in: session, event: "core_owned_output_audit_summary")
        let summary = try XCTUnwrap(summaries.last)
        XCTAssertTrue(summary.contains("candidate_transaction_count=1"), summary)
        XCTAssertTrue(summary.contains("coinjoin_to_bip44_missing_count=1"), summary)
        XCTAssertTrue(summary.contains("coinjoin_to_bip44_missing_value_duffs=151072"), summary)
        XCTAssertTrue(summary.contains("owned_bip44_output_count=1"), summary)
        XCTAssertTrue(summary.contains("persisted_valid_count=0"), summary)
        XCTAssertTrue(summary.contains("total_anomaly_count=1"), summary)
        // Output 1 of the fixture is `OP_RETURN`: no address to attribute.
        XCTAssertTrue(summary.contains("output_address_undecodable_count=1"), summary)
        XCTAssertTrue(summary.contains("unattributed_output_count=0"), summary)
        XCTAssertTrue(summary.contains("bip44_address_pool_size=1"), summary)

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

    func testPersistedBip44ChangeClears4438Alarm() async throws {
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
        let databaseSnapshot = await fixture.handler.emitCoreWalletDatabaseDiagnostics(walletId: walletId)
        XCTAssertNotNil(databaseSnapshot)

        let summaries = try logLines(in: session, event: "core_owned_output_audit_summary")
        let summary = try XCTUnwrap(summaries.last)
        XCTAssertTrue(summary.contains("coinjoin_to_bip44_missing_count=0"), summary)
        XCTAssertTrue(summary.contains("owned_bip44_output_count=1"), summary)
        XCTAssertTrue(summary.contains("persisted_valid_count=1"), summary)
        XCTAssertTrue(summary.contains("total_anomaly_count=0"), summary)
        XCTAssertTrue(try logLines(in: session, event: "core_owned_output_anomaly").isEmpty)
    }

    /// Above the row ceilings the export must refuse the exact audit outright
    /// and say so — never truncate the table and misclassify — while every
    /// lightweight snapshot still lands. The fixture has two transactions and
    /// one TXO, so a ceiling of one transaction is over the line.
    func testExportDeclinesExactAuditAboveRowLimitsButKeepsSnapshots() async throws {
        let fixture = try makeMissingOwnedOutputFixture()
        let session = try temporaryDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))

        let databaseSnapshot = await fixture.handler.emitCoreWalletDatabaseDiagnostics(
            walletId: walletId,
            limits: CoreDiagnosticRowLimits(
                crossWalletTxoRows: 100,
                exactAuditTransactionRows: 1
            )
        )
        XCTAssertNotNil(databaseSnapshot)

        let summaries = try logLines(in: session, event: "core_owned_output_audit_summary")
        let summary = try XCTUnwrap(summaries.last)
        XCTAssertTrue(summary.contains("audit_incomplete=true"), summary)
        XCTAssertTrue(summary.contains(#"reason="tables_too_large_for_exact_audit""#), summary)
        XCTAssertTrue(summary.contains("transaction_row_count=2"), summary)
        XCTAssertTrue(summary.contains("transaction_row_limit=1"), summary)
        XCTAssertTrue(summary.contains("txo_row_count=1"), summary)
        // Declining is not the same as finding nothing: no per-output verdict
        // may be emitted for an audit that never ran.
        XCTAssertFalse(summary.contains("coinjoin_to_bip44_missing_count"), summary)
        XCTAssertTrue(try logLines(in: session, event: "core_owned_output_anomaly").isEmpty)

        let walletSnapshot = try XCTUnwrap(
            try logLines(in: session, event: "core_db_wallet_snapshot").last
        )
        XCTAssertTrue(walletSnapshot.contains(#"txo_scan_scope="cross_wallet""#), walletSnapshot)
        XCTAssertTrue(walletSnapshot.contains("transaction_scan_available=false"), walletSnapshot)
        XCTAssertTrue(walletSnapshot.contains("txo_count=1"), walletSnapshot)
        XCTAssertFalse(try logLines(in: session, event: "core_db_account_snapshot").isEmpty)
        XCTAssertFalse(try logLines(in: session, event: "core_db_anomaly_summary").isEmpty)
        XCTAssertFalse(try logLines(in: session, event: "asset_lock_db_snapshot").isEmpty)
    }

    /// Over the TXO ceiling the scan narrows to this wallet's denormalized id
    /// and the snapshot records that scope, so an analyst knows relationship-
    /// only rows and cross-wallet duplicates were outside its view.
    func testExportNarrowsTxoScanToWalletAboveTxoRowLimit() async throws {
        let fixture = try makeMissingOwnedOutputFixture()
        let session = try temporaryDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))

        let databaseSnapshot = await fixture.handler.emitCoreWalletDatabaseDiagnostics(
            walletId: walletId,
            limits: CoreDiagnosticRowLimits(
                crossWalletTxoRows: 0,
                exactAuditTransactionRows: 100
            )
        )
        XCTAssertNotNil(databaseSnapshot)

        let walletSnapshot = try XCTUnwrap(
            try logLines(in: session, event: "core_db_wallet_snapshot").last
        )
        XCTAssertTrue(walletSnapshot.contains(#"txo_scan_scope="wallet_id_only""#), walletSnapshot)
        XCTAssertTrue(walletSnapshot.contains("txo_count=1"), walletSnapshot)
        // A narrowed TXO scan alone is enough to decline the audit, even
        // though the transaction table is under its own ceiling.
        let summary = try XCTUnwrap(
            try logLines(in: session, event: "core_owned_output_audit_summary").last
        )
        XCTAssertTrue(summary.contains(#"reason="tables_too_large_for_exact_audit""#), summary)
        XCTAssertTrue(summary.contains("txo_row_limit=0"), summary)
    }

    /// Without the change address's `PersistentCoreAddress` row the audit can
    /// attribute nothing, and must say so through the counter rather than
    /// report a clean wallet — this is the "false all-clear" from review.
    func testMissingAddressRowIsCountedAsUnattributedNotCleared() async throws {
        let fixture = try makeMissingOwnedOutputFixture()
        fixture.context.delete(fixture.bip44Address)
        try fixture.context.save()

        let session = try temporaryDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))
        let databaseSnapshot = await fixture.handler.emitCoreWalletDatabaseDiagnostics(walletId: walletId)
        XCTAssertNotNil(databaseSnapshot)

        let summary = try XCTUnwrap(
            try logLines(in: session, event: "core_owned_output_audit_summary").last
        )
        XCTAssertTrue(summary.contains("candidate_transaction_count=1"), summary)
        XCTAssertTrue(summary.contains("bip44_address_pool_size=0"), summary)
        XCTAssertTrue(summary.contains("unattributed_output_count=1"), summary)
        XCTAssertTrue(summary.contains("output_address_undecodable_count=1"), summary)
        XCTAssertTrue(summary.contains("owned_bip44_output_count=0"), summary)
        // Zero here means "of what could be attributed" — and the counters
        // above show that was nothing.
        XCTAssertTrue(summary.contains("coinjoin_to_bip44_missing_count=0"), summary)
        XCTAssertTrue(summary.contains("total_anomaly_count=0"), summary)
    }

    /// A change row that names this wallet by its denormalized id but has no
    /// account relationship is this wallet's row with a broken link, not
    /// another wallet's row — the same rule that admits it must judge it.
    func testOwnedRowWithBrokenRelationshipIsRelationshipMissingNotWrongWallet() async throws {
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
        change.walletId = walletId
        change.isConfirmed = true
        fixture.context.insert(change)
        try fixture.context.save()

        let session = try temporaryDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))
        let databaseSnapshot = await fixture.handler.emitCoreWalletDatabaseDiagnostics(walletId: walletId)
        XCTAssertNotNil(databaseSnapshot)

        let summary = try XCTUnwrap(
            try logLines(in: session, event: "core_owned_output_audit_summary").last
        )
        XCTAssertTrue(summary.contains("coinjoin_to_bip44_missing_count=0"), summary)
        XCTAssertTrue(summary.contains("total_anomaly_count=1"), summary)
        let anomaly = try XCTUnwrap(
            try logLines(in: session, event: "core_owned_output_anomaly").last
        )
        XCTAssertTrue(anomaly.contains(#"reason="relationship_missing""#), anomaly)
        XCTAssertFalse(anomaly.contains("wrong_wallet"), anomaly)
    }

    /// `outpoint` is `@Attribute(.unique)`, so a duplicated outpoint only ever
    /// exists in a corrupt store and cannot be saved through a context. The
    /// resolver is therefore exercised on transient rows.
    func testRepresentativeTxoPrefersThisWalletsRowRegardlessOfFetchOrder() throws {
        let ours = PersistentWallet(walletId: walletId, network: .testnet)
        let theirs = PersistentWallet(walletId: Data(repeating: 0xB2, count: 32), network: .testnet)
        let ourAccount = PersistentAccount(
            wallet: ours, accountType: 0, accountIndex: 0, accountTypeName: "Standard"
        )
        let theirAccount = PersistentAccount(
            wallet: theirs, accountType: 0, accountIndex: 0, accountTypeName: "Standard"
        )
        let transaction = PersistentTransaction(
            txid: Data(repeating: 0x44, count: 32),
            transactionData: Data(),
            context: 2,
            blockHeight: 1,
            netAmount: 0
        )
        func row(amount: UInt64, account: PersistentAccount?, walletId: Data) -> PersistentTxo {
            let txo = PersistentTxo(
                transaction: transaction,
                vout: 0,
                amount: amount,
                address: "duplicate-outpoint",
                scriptPubKey: Data([0x51]),
                height: 1
            )
            txo.account = account
            txo.walletId = walletId
            return txo
        }
        let ourRow = row(amount: 1, account: ourAccount, walletId: walletId)
        let theirRow = row(amount: 2, account: theirAccount, walletId: theirs.walletId)
        let theirOtherRow = row(amount: 3, account: theirAccount, walletId: theirs.walletId)

        XCTAssertTrue(
            PlatformWalletPersistenceHandler.representativeTxo(
                rows: [theirRow, ourRow], walletId: walletId
            ) === ourRow
        )
        XCTAssertTrue(
            PlatformWalletPersistenceHandler.representativeTxo(
                rows: [ourRow, theirRow], walletId: walletId
            ) === ourRow
        )
        // Ours by denormalized id alone still wins: the audit judges by the
        // same rule that admits, and reports the broken link separately.
        let ourBrokenRow = row(amount: 4, account: nil, walletId: walletId)
        XCTAssertTrue(
            PlatformWalletPersistenceHandler.representativeTxo(
                rows: [theirRow, ourBrokenRow], walletId: walletId
            ) === ourBrokenRow
        )
        // No owned row: still the same answer whichever order the fetch gave.
        let forward = PlatformWalletPersistenceHandler.representativeTxo(
            rows: [theirRow, theirOtherRow], walletId: walletId
        )
        let reversed = PlatformWalletPersistenceHandler.representativeTxo(
            rows: [theirOtherRow, theirRow], walletId: walletId
        )
        XCTAssertNotNil(forward)
        XCTAssertTrue(forward === reversed)
        XCTAssertNil(
            PlatformWalletPersistenceHandler.representativeTxo(rows: nil, walletId: walletId)
        )
        XCTAssertNil(
            PlatformWalletPersistenceHandler.representativeTxo(rows: [], walletId: walletId)
        )
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
        // The complete set of events the pre-export path emits, spelled exactly
        // as `PlatformWalletManagerCoreDiagnostics` writes them — a name that
        // is never emitted (`core_db_memory_diff`, say, whose real event is
        // `core_db_memory_diff_item`) would make its guard vacuous.
        let deepDiagnosticEvents = [
            "asset_lock_db_group",
            "asset_lock_db_memory_diff_item",
            "asset_lock_db_memory_diff_summary",
            "asset_lock_db_snapshot",
            "asset_lock_memory_group",
            "asset_lock_memory_snapshot",
            "core_db_account_snapshot",
            "core_db_anomaly_summary",
            "core_db_memory_diff_item",
            "core_db_memory_diff_summary",
            "core_db_txo_anomaly",
            "core_db_wallet_snapshot",
            "core_diagnostics_unavailable",
            "core_memory_account_snapshot",
            "core_memory_snapshot_unavailable",
            "core_owned_output_anomaly",
            "core_owned_output_audit_summary",
            "shielded_store_snapshot",
        ]
        for event in deepDiagnosticEvents {
            XCTAssertFalse(completeLog.contains("event=\(event) "), event)
        }
    }
}
