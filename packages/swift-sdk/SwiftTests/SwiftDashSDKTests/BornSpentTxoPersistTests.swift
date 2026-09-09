import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the engine's credit verdicts at the changeset seam — the
/// extension's `on_persist_wallet_changeset_utxo_verdicts_fn`, fired
/// BEFORE a round's changeset callback.
///
/// The shape they exist for (rust-dashcore#992, dashpay/platform#4575): a
/// coin is spent by a transaction with no wallet-owned output (a CoinJoin
/// collateral burn — sole `OP_RETURN` output) that the engine processed
/// while the coin was not yet in its UTXO set, so the spender matched
/// nothing and was discarded. When the funding record is (re)emitted it
/// still classifies the output `Received`, the persister derives a UTXO
/// row from that role, and — without the verdict — writes it UNSPENT for a
/// coin the engine never held. The restore path then hands that row back
/// to the engine on every launch, and the balance the engine had corrected
/// returns as a phantom. With the verdict the row is written spent at
/// creation, and the restore emits nothing for it.
@MainActor
final class BornSpentTxoPersistTests: XCTestCase {
    private let walletId = Data(repeating: 0x21, count: 32)
    private let fundingTxid = Data(repeating: 0x61, count: 32)
    private let otherTxid = Data(repeating: 0x62, count: 32)

    private static let observedSpent: UInt8 = 1
    private static let doomed: UInt8 = 2
    private static let uncredited: UInt8 = 3

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try seedWallet(in: container)
        return (handler, container)
    }

    /// File-backed variant, so a restart (a fresh handler over the same
    /// on-disk store) can be simulated.
    private func makeHandler(url: URL) throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let configuration = ModelConfiguration(schema: DashModelContainer.schema, url: url)
        let container = try ModelContainer(
            for: DashModelContainer.schema,
            migrationPlan: DashMigrationPlan.self,
            configurations: [configuration]
        )
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        return (handler, container)
    }

    private func seedWallet(in container: ModelContainer) throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
    }

    private func round(_ handler: PlatformWalletPersistenceHandler, _ body: () -> Bool) {
        handler.beginChangeset(walletId: walletId)
        let success = body()
        _ = handler.endChangeset(walletId: walletId, success: success)
    }

    /// The verdict slot alone, inside whatever bracket the caller opened.
    private func stageVerdicts(
        _ handler: PlatformWalletPersistenceHandler,
        _ verdicts: [(txid: Data, vout: UInt32, verdict: UInt8, height: UInt32)]
    ) -> Bool {
        var entries: [UtxoCreditVerdictFFI] = verdicts.map { verdict in
            var entry = UtxoCreditVerdictFFI()
            Swift.withUnsafeMutableBytes(of: &entry.outpoint.txid) { dst in
                verdict.txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
            }
            entry.outpoint.vout = verdict.vout
            entry.verdict = verdict.verdict
            entry.spent_at_height = verdict.height
            return entry
        }
        return entries.withUnsafeMutableBufferPointer { ptr in
            handler.persistWalletChangesetUtxoVerdicts(
                walletId: walletId,
                verdicts: UnsafePointer(ptr.baseAddress),
                count: UInt(ptr.count)
            )
        }
    }

    /// The changeset callback alone: one `utxos_added` entry for
    /// `txid:vout`, inside whatever bracket the caller opened.
    private func stageUtxoAdded(
        _ handler: PlatformWalletPersistenceHandler,
        txid: Data,
        vout: UInt32,
        amount: UInt64 = 19_549,
        height: UInt32 = 2_391_743
    ) -> Bool {
        let name = strdup("Standard { index: 0 }")
        let address = strdup("yBornSpentFixtureAddr")
        defer {
            free(name)
            free(address)
        }
        var utxo = UtxoEntryFFI()
        Swift.withUnsafeMutableBytes(of: &utxo.outpoint.txid) { dst in
            txid.withUnsafeBytes { src in dst.copyMemory(from: src) }
        }
        utxo.outpoint.vout = vout
        utxo.amount = amount
        utxo.address = address
        utxo.height = height
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
        return applied
    }

    private func txo(_ container: ModelContainer, txid: Data, vout: UInt32) throws -> PersistentTxo? {
        let outpoint = PersistentTxo.makeOutpoint(txid: txid, vout: vout)
        let context = ModelContext(container)
        return try context.fetch(
            FetchDescriptor<PersistentTxo>(predicate: #Predicate { $0.outpoint == outpoint })
        ).first
    }

    /// Drives the FFI load path and returns the wallet entry's restored
    /// UTXO count — what the engine would be handed at launch.
    private func restoredUtxoCount(_ handler: PlatformWalletPersistenceHandler) throws -> Int {
        let (entries, count, errored) = handler.loadWalletList()
        XCTAssertFalse(errored)
        XCTAssertEqual(count, 1)
        let entriesPtr = try XCTUnwrap(entries)
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entriesPtr)) }
        return Int(entriesPtr[0].utxos_count)
    }

    // MARK: - The field case

    /// The funding record's output arrives with an observed-spent verdict:
    /// the row is created, spent, unlinked — and the restore hands nothing
    /// back for it.
    func testObservedSpentVerdictWritesTheRowSpentAndKeepsItOutOfTheRestore() throws {
        let (handler, container) = try makeHandler()
        round(handler) {
            stageVerdicts(handler, [(fundingTxid, 0, Self.observedSpent, 2_402_896)])
                && stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
        }
        let coin = try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0))
        XCTAssertTrue(coin.isSpent)
        XCTAssertNil(coin.spendingTransaction, "the spender was never recorded")
        XCTAssertNil(coin.supersededByTxid)
        XCTAssertEqual(coin.amount, 19_549)
        XCTAssertEqual(try restoredUtxoCount(handler), 0)
    }

    func testDoomedVerdictWritesTheRowSpent() throws {
        let (handler, container) = try makeHandler()
        round(handler) {
            stageVerdicts(handler, [(fundingTxid, 0, Self.doomed, 0)])
                && stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
        }
        XCTAssertTrue(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)
        XCTAssertEqual(try restoredUtxoCount(handler), 0)
    }

    /// A context-free verdict never marks the row spent: the engine only
    /// said "not credited", and the spender's own record or the sweep
    /// callback settles it. What it does change is the redelivery clear.
    func testUncreditedVerdictLeavesANewRowUnspent() throws {
        let (handler, container) = try makeHandler()
        round(handler) {
            stageVerdicts(handler, [(fundingTxid, 0, Self.uncredited, 0)])
                && stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
        }
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)
        XCTAssertEqual(try restoredUtxoCount(handler), 1)
    }

    /// Without a verdict, a redelivery of a spent, unlinked row clears the
    /// flag (the wallet is handing the coin back as unspent — today's
    /// recovery rule). With ANY verdict the clear is vetoed: the record
    /// merely still names the output as ours, the engine does not hold it.
    func testAVerdictVetoesTheRedeliveryClearAndItsAbsenceDoesNot() throws {
        let (handler, container) = try makeHandler()
        round(handler) {
            stageVerdicts(handler, [(fundingTxid, 0, Self.observedSpent, 2_402_896)])
                && stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
        }
        XCTAssertTrue(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)

        // Redelivered with a context-free verdict: stays spent.
        round(handler) {
            stageVerdicts(handler, [(fundingTxid, 0, Self.uncredited, 0)])
                && stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
        }
        XCTAssertTrue(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)
        XCTAssertEqual(try restoredUtxoCount(handler), 0)

        // Redelivered with no verdict at all: the engine credited it again
        // (a reorg of the spender), and the row follows the wallet.
        round(handler) {
            stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
        }
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)
        XCTAssertEqual(try restoredUtxoCount(handler), 1)
    }

    /// Verdicts are round-scoped: one for an outpoint the round never
    /// delivers is dropped with the round, and a later round delivering
    /// that outpoint without a verdict writes it unspent as ever.
    func testVerdictsDoNotOutliveTheirRound() throws {
        let (handler, container) = try makeHandler()
        round(handler) {
            stageVerdicts(handler, [(otherTxid, 0, Self.observedSpent, 2_402_896)])
                && stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
        }
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)
        XCTAssertNil(try txo(container, txid: otherTxid, vout: 0))

        round(handler) {
            stageUtxoAdded(handler, txid: otherTxid, vout: 0)
        }
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: otherTxid, vout: 0)).isSpent)
    }

    /// A verdict with no matching wallet row is accepted and ignored — the
    /// same contract as every other per-kind callback for an unknown wallet.
    func testVerdictForAnUnknownWalletIsAcceptedAndIgnored() throws {
        let (handler, _) = try makeHandler()
        let stranger = Data(repeating: 0x7e, count: 32)
        var entry = UtxoCreditVerdictFFI()
        entry.verdict = Self.observedSpent
        let accepted = withUnsafePointer(to: &entry) { ptr in
            handler.persistWalletChangesetUtxoVerdicts(walletId: stranger, verdicts: ptr, count: 1)
        }
        XCTAssertTrue(accepted)
    }

    /// A rolled-back round leaves neither the row nor the verdict behind.
    func testARolledBackRoundLeavesNoRow() throws {
        let (handler, container) = try makeHandler()
        round(handler) {
            _ = stageVerdicts(handler, [(fundingTxid, 0, Self.observedSpent, 2_402_896)])
            _ = stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
            return false
        }
        XCTAssertNil(try txo(container, txid: fundingTxid, vout: 0))
        XCTAssertEqual(try restoredUtxoCount(handler), 0)
    }

    // MARK: - Restart

    /// The acceptance shape: after the round that wrote the row spent, a
    /// relaunch (a fresh handler over the same on-disk store) restores zero
    /// coins for the wallet — and so does the relaunch after that. The
    /// phantom never comes back.
    func testRestartRestoresNothingForABornSpentRow() throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("born-spent-\(UUID().uuidString).sqlite")
        defer {
            for suffix in ["", "-wal", "-shm"] {
                try? FileManager.default.removeItem(at: URL(fileURLWithPath: url.path + suffix))
            }
        }
        do {
            let (handler, container) = try makeHandler(url: url)
            try seedWallet(in: container)
            round(handler) {
                stageVerdicts(handler, [(fundingTxid, 0, Self.observedSpent, 2_402_896)])
                    && stageUtxoAdded(handler, txid: fundingTxid, vout: 0)
            }
            XCTAssertTrue(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)
            XCTAssertEqual(try restoredUtxoCount(handler), 0)
        }
        for _ in 0..<2 {
            let (handler, container) = try makeHandler(url: url)
            XCTAssertTrue(try XCTUnwrap(txo(container, txid: fundingTxid, vout: 0)).isSpent)
            XCTAssertEqual(try restoredUtxoCount(handler), 0, "a relaunch restores nothing for the burned coin")
        }
    }
}
