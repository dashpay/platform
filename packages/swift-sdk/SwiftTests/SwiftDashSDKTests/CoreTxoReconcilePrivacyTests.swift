import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Every event the credit-verdict seam and the store reconcile emit goes
/// through the SDK's file sink here, with fixture values shaped like the
/// real thing, and the rendered log is checked for them: no address, no
/// txid or outpoint in either byte orientation, no script, and — as a
/// backstop against any future field — no long hex or Base58 run at all.
/// References are 12 hex characters; counts and duffs are numbers.
@MainActor
final class CoreTxoReconcilePrivacyTests: XCTestCase {
    private let walletId = Data(repeating: 0x71, count: 32)
    /// A realistic testnet-length Base58 address.
    private let fixtureAddress = "yTestPrivacyFixtureAddress12345678"
    private let fixtureScript = Data([0x76, 0xa9, 0x14] + [UInt8](repeating: 0x5c, count: 20) + [0x88, 0xac])
    private let healedTxid = Data((0..<32).map { UInt8(0xa0 + $0 % 16) })
    private let flippedTxid = Data((0..<32).map { UInt8(0x30 + $0 % 16) })
    private let bornSpentTxid = Data((0..<32).map { UInt8(0xc0 + $0 % 16) })

    private var bip44: CoreAccountKey {
        CoreAccountKey(
            typeTag: 0, standardTag: 0, index: 0, registrationIndex: 0, keyClass: 0,
            userIdentityId: Data(count: 32), friendIdentityId: Data(count: 32)
        )
    }

    private func hex(_ data: Data) -> String {
        data.map { String(format: "%02x", $0) }.joined()
    }

    func testTheEmittedEventsCarryNoWalletHistory() throws {
        let sessionDirectory = FileManager.default.temporaryDirectory
            .appendingPathComponent("txo-reconcile-privacy-\(UUID().uuidString)", isDirectory: true)
        defer { try? FileManager.default.removeItem(at: sessionDirectory) }
        XCTAssertTrue(SDKLogger.installFileSink(at: sessionDirectory, includeDebug: true))

        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        do {
            let context = ModelContext(container)
            let wallet = PersistentWallet(walletId: walletId, network: .testnet)
            context.insert(wallet)
            let account = PersistentAccount(wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "BIP44 Account")
            account.userIdentityId = Data(count: 32)
            account.friendIdentityId = Data(count: 32)
            context.insert(account)
            let tx = PersistentTransaction(
                txid: flippedTxid, transactionData: Data(repeating: 0x04, count: 10),
                context: 3, blockHeight: 2_391_743, netAmount: 19_549
            )
            context.insert(tx)
            let txo = PersistentTxo(
                transaction: tx, vout: 1, amount: 19_549, address: fixtureAddress,
                scriptPubKey: fixtureScript, height: 2_391_743
            )
            txo.account = account
            txo.walletId = walletId
            txo.isConfirmed = true
            context.insert(txo)
            try context.save()
        }

        // The seam: a round with an observed-spent verdict for a delivered coin.
        handler.beginChangeset(walletId: walletId)
        var verdict = UtxoCreditVerdictFFI()
        Swift.withUnsafeMutableBytes(of: &verdict.outpoint.txid) { dst in
            bornSpentTxid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        verdict.verdict = 1
        verdict.spent_at_height = 2_402_896
        let staged = withUnsafePointer(to: &verdict) { ptr in
            handler.persistWalletChangesetUtxoVerdicts(walletId: walletId, verdicts: ptr, count: 1)
        }
        XCTAssertTrue(staged)
        let name = strdup("Standard { index: 0 }")
        let address = strdup(fixtureAddress)
        defer {
            free(name)
            free(address)
        }
        var utxo = UtxoEntryFFI()
        Swift.withUnsafeMutableBytes(of: &utxo.outpoint.txid) { dst in
            bornSpentTxid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        utxo.amount = 19_549
        utxo.address = address
        utxo.height = 2_391_786
        utxo.is_confirmed = true
        var applied = false
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
                    applied = handler.persistWalletChangeset(walletId: walletId, changeset: csPtr)
                }
            }
        }
        XCTAssertTrue(applied)
        _ = handler.endChangeset(walletId: walletId, success: true)

        // The reconcile: one heal, one flip, one row the engine cannot classify.
        let healed = CoreEngineUtxo(
            account: bip44, txid: healedTxid, vout: 0, amount: 100_001, address: fixtureAddress,
            scriptPubKey: fixtureScript, height: 2_402_986, isConfirmed: true,
            isInstantLocked: false, isCoinbase: false, isLocked: false
        )
        let flipped = PersistentTxo.makeOutpoint(txid: flippedTxid, vout: 1)
        let engine = FakeCoreTxoEngine(inventory: [healed], verdicts: [flipped: .knownUncredited])
        let report = PlatformWalletManager.runCoreTxoReconcile(
            walletId: walletId, tipHeight: 2_535_898, engine: engine, handler: handler,
            isCancelled: { false }
        )
        XCTAssertEqual(report.inserted, 1)
        XCTAssertEqual(report.flipped, 1)
        SDKLogger.flush()

        let logURL = sessionDirectory.appendingPathComponent("swift").appendingPathComponent("run.log")
        let log = try String(contentsOf: logURL, encoding: .utf8)
        XCTAssertTrue(log.contains("event=persistence_txo_credit_verdicts"))
        XCTAssertTrue(log.contains("event=persistence_txo_reconcile_item"))
        XCTAssertTrue(log.contains("observed_spent_count=1"))
        XCTAssertTrue(log.contains("action=\"healed\""))
        XCTAssertTrue(log.contains("action=\"flipped_spent\""))

        let forbidden: [(String, String)] = [
            ("address", fixtureAddress),
            ("script", hex(fixtureScript)),
            ("healed txid", hex(healedTxid)),
            ("healed txid reversed", hex(Data(healedTxid.reversed()))),
            ("flipped txid", hex(flippedTxid)),
            ("flipped txid reversed", hex(Data(flippedTxid.reversed()))),
            ("born-spent txid", hex(bornSpentTxid)),
            ("born-spent txid reversed", hex(Data(bornSpentTxid.reversed()))),
            ("flipped outpoint", hex(flipped)),
            ("wallet id", hex(walletId)),
        ]
        for (label, value) in forbidden {
            XCTAssertFalse(
                log.range(of: value, options: .caseInsensitive) != nil,
                "the log must not carry the \(label)"
            )
        }
        let longHex = try NSRegularExpression(pattern: "[0-9a-fA-F]{32,}")
        XCTAssertNil(
            longHex.firstMatch(in: log, range: NSRange(log.startIndex..., in: log)),
            "no 32+ character hex run may appear in any event"
        )
        let base58Run = try NSRegularExpression(pattern: "[1-9A-HJ-NP-Za-km-z]{26,}")
        let lines = log.split(separator: "\n").filter {
            $0.contains("persistence_txo_") || $0.contains("txo_reconcile")
        }
        XCTAssertFalse(lines.isEmpty)
        for line in lines {
            let text = String(line)
            XCTAssertNil(
                base58Run.firstMatch(in: text, range: NSRange(text.startIndex..., in: text)),
                "no address-length Base58 run may appear: \(text)"
            )
        }
    }
}
