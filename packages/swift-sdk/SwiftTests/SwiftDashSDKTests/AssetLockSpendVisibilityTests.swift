import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

// MARK: - Asset-lock spend visibility
//
// An asset-lock transaction burns its value into the special-tx payload
// and often has no wallet-owned standard output, so SPV block matching
// can miss it: the spender's transaction row never leaves mempool
// context, `resolveInputOutpoint`'s in-block flip never runs, and the
// funding TXOs it consumed stay `isSpent == false` forever. Every
// relaunch then hands those consumed outputs back to Rust as spendable.
//
// Two repairs cover it — the callback-time reconcile in
// `persistAssetLocks`, and the load-time guard in `loadWalletList` for
// wallets whose lock reached the terminal `Consumed` status before that
// reconcile existed and so has no future callback to fire. Both depend
// on a SwiftData read, and both must fail CLOSED: the whole point is to
// withhold outputs that are gone, so an unreadable table may never be
// read as "nothing to withhold."
//
// Both directions are pinned here. The readable behaviour goes through a
// live store; the fail-closed behaviour goes through the handler's
// `ModelFetching` seam, which faults exactly one model type's read and
// serves every other read live. That isolation is the point: a load-path
// regression cannot pass because the wallet or unspent-TXO fetch failed
// first, and the reconcile regression cannot pass because the whole store
// was unreadable.

/// Serves every read live except the one model type it is told to fault,
/// and records the reads it saw so a test can prove which fetch failed.
private final class FetchFaultInjector: ModelFetching, @unchecked Sendable {
    struct ReadFault: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    private let live = LiveModelFetcher()
    private let faulted: ObjectIdentifier
    private let faultMessage: String
    private let lock = NSLock()
    private var reads: [String] = []

    init(
        faulting model: any PersistentModel.Type,
        faultMessage: String = "injected SwiftData read failure"
    ) {
        faulted = ObjectIdentifier(model)
        self.faultMessage = faultMessage
    }

    /// Model names in the order they were read, the faulted one included.
    var observedReads: [String] {
        lock.lock()
        defer { lock.unlock() }
        return reads
    }

    func fetch<T: PersistentModel>(
        _ descriptor: FetchDescriptor<T>,
        in context: ModelContext
    ) throws -> [T] {
        lock.lock()
        reads.append(String(describing: T.self))
        lock.unlock()
        guard ObjectIdentifier(T.self) != faulted else {
            throw ReadFault(message: faultMessage)
        }
        return try live.fetch(descriptor, in: context)
    }
}

final class AssetLockSpendVisibilityTests: XCTestCase {

    private var container: ModelContainer!
    private var handler: PlatformWalletPersistenceHandler!

    private let walletId = Data(repeating: 0xAA, count: 32)
    private let fundingTxid = Data(repeating: 0x51, count: 32)
    private let lockTxid = Data(repeating: 0x52, count: 32)

    override func setUpWithError() throws {
        try super.setUpWithError()
        container = try DashModelContainer.createInMemory()
        handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet
        )
    }

    override func tearDown() {
        handler = nil
        container = nil
        super.tearDown()
    }

    private func temporaryLogDirectory() throws -> URL {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(
            "AssetLockSpendVisibilityTests-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: directory) }
        return directory
    }

    // MARK: Fixtures

    /// Seeds the state an older build left behind: a restorable wallet
    /// holding one funding TXO whose spender is an asset-lock
    /// transaction stuck at mempool context, so the row is still
    /// `isSpent == false` with `spendingTransaction` linked.
    @discardableResult
    private func seedFundingTxoSpentByAMempoolAssetLock(
        into container: ModelContainer
    ) throws -> Data {
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet,
            accountType: 0,
            accountIndex: 0,
            accountTypeName: "standard"
        )
        account.accountExtendedPubKeyBytes = Data(repeating: 0xEE, count: 78)
        context.insert(account)

        let fundingTx = PersistentTransaction(
            txid: fundingTxid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 2,
            blockHeight: 100
        )
        context.insert(fundingTx)
        let txo = PersistentTxo(
            transaction: fundingTx,
            vout: 0,
            amount: 999_545,
            address: "yLockFunder",
            scriptPubKey: Data(repeating: 0x06, count: 25),
            height: 100
        )
        txo.walletId = walletId
        txo.account = account
        txo.isConfirmed = true
        context.insert(txo)

        // Mempool context (0): the lock's spend is linked but the
        // in-block flip never ran.
        let lockTx = PersistentTransaction(
            txid: lockTxid,
            transactionData: Data(repeating: 0x05, count: 10),
            context: 0,
            transactionType: "AssetLock",
            netAmount: -999_545
        )
        context.insert(lockTx)
        txo.spendingTransaction = lockTx
        try context.save()
        return txo.outpoint
    }

    /// A directly-written terminal `Consumed` lock row, keyed at
    /// [vout] of the lock's own funding transaction.
    private func insertConsumedAssetLock(into container: ModelContainer, vout: UInt32) throws {
        let context = ModelContext(container)
        let outPoint = PersistentTxo.makeOutpoint(txid: lockTxid, vout: vout)
        context.insert(PersistentAssetLock(
            outPointHex: PersistentAssetLock.encodeOutPoint(rawBytes: outPoint),
            walletId: walletId,
            transactionBytes: Data(repeating: 0x05, count: 10),
            fundingTypeRaw: 0,
            identityIndexRaw: 0,
            amountDuffs: 999_545,
            statusRaw: 4
        ))
        try context.save()
    }

    /// Drives the FFI load path and returns the single wallet entry's
    /// UTXO count, releasing the buffers before returning.
    private func restoredUtxoCount() throws -> Int {
        let (entries, count, errored) = handler.loadWalletList()
        XCTAssertFalse(errored, "the restore must not report a failure here")
        XCTAssertEqual(count, 1)
        let entriesPtr = try XCTUnwrap(entries)
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entriesPtr)) }
        return Int(entriesPtr[0].utxos_count)
    }

    /// Committed `PersistentAssetLock` rows, read on a context of its own
    /// so a round staged but never saved does not count.
    private func persistedAssetLockCount() throws -> Int {
        try ModelContext(container).fetchCount(FetchDescriptor<PersistentAssetLock>())
    }

    private func txoIsSpent(outpoint: Data) throws -> Bool {
        let context = ModelContext(container)
        let descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        return try XCTUnwrap(try context.fetch(descriptor).first).isSpent
    }

    // MARK: Load-time guard

    /// `Consumed` is terminal — the lock never upserts again — so the
    /// callback-time reconcile has no future event to repair this with,
    /// and every relaunch would re-inflate the balance with an output
    /// that is provably gone. The load guard must both exclude it from
    /// the restore and heal the row in place.
    func testLoadExcludesAndHealsATxoConsumedByAFinalizedAssetLock() throws {
        let outpoint = try seedFundingTxoSpentByAMempoolAssetLock(into: container)

        // Without a finalized lock row this IS the phantom UTXO.
        XCTAssertEqual(try restoredUtxoCount(), 1)
        XCTAssertFalse(try txoIsSpent(outpoint: outpoint))

        try insertConsumedAssetLock(into: container, vout: 0)

        XCTAssertEqual(
            try restoredUtxoCount(), 0,
            "the finalized lock's funding output must not rehydrate as spendable"
        )
        XCTAssertTrue(
            try txoIsSpent(outpoint: outpoint),
            "and the stale flag must be healed in place"
        )
    }

    /// DIP-0027 lets one funding transaction carry several credit
    /// outputs, and Rust persists each tracked lock under its own
    /// credit-output index, so a valid lock row can be keyed `<txid>:1`
    /// with no `<txid>:0` row anywhere. Finality belongs to the
    /// transaction, so the guard keys on the funding txid alone.
    func testLoadExcludesATxoConsumedByALockPersistedAtANonZeroVout() throws {
        let outpoint = try seedFundingTxoSpentByAMempoolAssetLock(into: container)
        XCTAssertEqual(try restoredUtxoCount(), 1)

        try insertConsumedAssetLock(into: container, vout: 1)

        XCTAssertEqual(
            try restoredUtxoCount(), 0,
            "finality belongs to the funding transaction, not to credit output 0"
        )
        XCTAssertTrue(try txoIsSpent(outpoint: outpoint))
    }

    // MARK: Callback-time reconcile

    /// The reconcile that runs while the lock is still upserting:
    /// from `InstantSendLocked` the network has locked the inputs, so
    /// every TXO already linked to the lock's funding tx is spent.
    func testPersistAssetLocksFlipsLinkedTxosAndReportsSuccess() throws {
        let outpoint = try seedFundingTxoSpentByAMempoolAssetLock(into: container)
        let outPointRaw = PersistentTxo.makeOutpoint(txid: lockTxid, vout: 0)

        handler.beginChangeset(walletId: walletId)
        let staged = handler.persistAssetLocks(
            walletId: walletId,
            upserts: [.init(
                outPointHex: PersistentAssetLock.encodeOutPoint(rawBytes: outPointRaw),
                transactionBytes: Data(repeating: 0x05, count: 10),
                fundingTypeRaw: 0,
                identityIndexRaw: 0,
                accountIndexRaw: 0,
                amountDuffs: 999_545,
                statusRaw: 2,
                proofBytes: nil
            )],
            removed: []
        )
        XCTAssertTrue(staged, "a readable round must report success")
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        XCTAssertTrue(
            try txoIsSpent(outpoint: outpoint),
            "the linked funding TXO must be flipped by the reconcile"
        )
    }

    /// The reconcile's fetch is the only thing that can find the TXOs a
    /// now-final lock consumed, so an unreadable read must not be spent as
    /// "nothing to heal". `Consumed` is terminal — it never upserts again —
    /// so a status that commits over an unread TXO table leaves a phantom
    /// UTXO with no future callback to repair it. Fail the round instead,
    /// which rolls the status back and lets Rust re-emit it.
    func testPersistAssetLocksFailsTheRoundWhenTheStaleTxoFetchThrows() throws {
        let outpoint = try seedFundingTxoSpentByAMempoolAssetLock(into: container)
        let privacyTxid = Data((0..<32).map { UInt8($0) })
        let outPointRaw = PersistentTxo.makeOutpoint(txid: privacyTxid, vout: 0)
        let outPointHex = PersistentAssetLock.encodeOutPoint(rawBytes: outPointRaw)
        let injector = FetchFaultInjector(
            faulting: PersistentTxo.self,
            faultMessage: "failed for \(outPointHex), wire \(privacyTxid.hexString), at /private/user/wallet.sqlite"
        )
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet,
            modelFetcher: injector
        )
        let session = try temporaryLogDirectory()
        XCTAssertTrue(SDKLogger.installFileSink(at: session, includeDebug: false))

        handler.beginChangeset(walletId: walletId)
        let staged = handler.persistAssetLocks(
            walletId: walletId,
            upserts: [.init(
                outPointHex: outPointHex,
                transactionBytes: Data(repeating: 0x05, count: 10),
                fundingTypeRaw: 0,
                identityIndexRaw: 0,
                accountIndexRaw: 0,
                amountDuffs: 999_545,
                statusRaw: 4,
                proofBytes: nil
            )],
            removed: []
        )

        XCTAssertEqual(
            injector.observedReads, ["PersistentTxo"],
            "the faulted read must be the reconcile's stale-TXO fetch"
        )
        XCTAssertFalse(
            staged,
            "an unreadable stale-TXO fetch must fail the round, not report success"
        )
        // What Rust does with a non-zero callback: close the round as failed.
        XCTAssertFalse(handler.endChangeset(walletId: walletId, success: staged))

        XCTAssertEqual(
            try persistedAssetLockCount(), 0,
            "the terminal status must not commit ahead of the spend flags it could not read"
        )
        XCTAssertFalse(
            try txoIsSpent(outpoint: outpoint),
            "and nothing may be left half-applied by the rolled-back round"
        )

        SDKLogger.flush()
        let log = try String(
            contentsOf: session.appendingPathComponent("swift/run.log"),
            encoding: .utf8
        )
        let event = try XCTUnwrap(log.split(separator: "\n").first {
            $0.contains("event=persistence_asset_lock_stale_txo_fetch_failed ")
        })
        XCTAssertTrue(event.contains(
            "outpoint_reference=\(SDKLogFormatter.reference(outPointHex))"
        ))
        XCTAssertTrue(event.contains("<redacted>"))
        XCTAssertFalse(event.contains(outPointHex))
        XCTAssertFalse(event.contains(privacyTxid.hexString))
        XCTAssertFalse(event.contains(Data(privacyTxid.reversed()).hexString))
        XCTAssertFalse(event.contains(outPointRaw.hexString))
        XCTAssertFalse(event.contains(walletId.hexString))
        XCTAssertFalse(event.contains("wallet.sqlite"))
    }

    /// An unreadable lock table must not be read as the positive claim
    /// "no lock has finalized": the caller acts on that by restoring every
    /// `isSpent == false` row, which is exactly the consumed output this
    /// guard exists to withhold. The load must reject the snapshot.
    func testLoadReportsFailureWhenTheFinalizedLockFetchThrows() throws {
        try seedFundingTxoSpentByAMempoolAssetLock(into: container)
        try insertConsumedAssetLock(into: container, vout: 0)

        let injector = FetchFaultInjector(faulting: PersistentAssetLock.self)
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet,
            modelFetcher: injector
        )

        let (entries, count, errored) = handler.loadWalletList()
        if let entries {
            handler.loadWalletListFree(entries: UnsafeRawPointer(entries))
        }

        XCTAssertEqual(
            injector.observedReads,
            ["PersistentWallet", "PersistentTxo", "PersistentAssetLock"],
            "the wallet and unspent-TXO reads must have been served — only the lock read failed"
        )
        XCTAssertTrue(
            errored,
            "an unreadable lock table may not restore as 'no lock has finalized'"
        )
        XCTAssertNil(entries)
        XCTAssertEqual(count, 0)
    }
}
