import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// The engine reads the reconcile is built on, faked: a fixed inventory
/// served in pages behind a cursor, and a verdict per outpoint.
final class FakeCoreTxoEngine: CoreTxoEngineInventory, @unchecked Sendable {
    private let lock = NSLock()
    private var _inventory: [CoreEngineUtxo]
    private var _verdicts: [Data: CoreOutpointClass]
    private var _failPages = false
    private var _failClassify = false
    private(set) var pageCalls = 0
    private(set) var classifyCalls = 0
    private(set) var classified: [CoreOutpointOwnershipQuery] = []
    /// Runs before each `classify`, outside the engine's lock — a test's
    /// stand-in for the world moving while the reconcile is between its
    /// engine read and its store write (e.g. a persistence round committing).
    var onClassify: (@Sendable () -> Void)?

    init(inventory: [CoreEngineUtxo] = [], verdicts: [Data: CoreOutpointClass] = [:]) {
        _inventory = inventory
        _verdicts = verdicts
    }

    var failPages: Bool {
        get { lock.withLock { _failPages } }
        set { lock.withLock { _failPages = newValue } }
    }

    var failClassify: Bool {
        get { lock.withLock { _failClassify } }
        set { lock.withLock { _failClassify = newValue } }
    }

    struct Failure: Error {}

    func utxoPage(after: CoreEngineUtxo?, limit: Int) throws -> (rows: [CoreEngineUtxo], hasMore: Bool) {
        try lock.withLock {
            pageCalls += 1
            if _failPages { throw Failure() }
            var start = 0
            if let after, let index = _inventory.firstIndex(where: { $0.outpoint == after.outpoint }) {
                start = index + 1
            }
            let end = min(start + limit, _inventory.count)
            let rows = start < end ? Array(_inventory[start..<end]) : []
            return (rows, end < _inventory.count)
        }
    }

    func setVerdict(_ outpoint: Data, _ verdict: CoreOutpointClass) {
        lock.withLock { _verdicts[outpoint] = verdict }
    }

    func classify(_ queries: [CoreOutpointOwnershipQuery]) throws -> [CoreOutpointClass] {
        onClassify?()
        return try lock.withLock {
            classifyCalls += 1
            if _failClassify { throw Failure() }
            classified.append(contentsOf: queries)
            return queries.map { _verdicts[$0.outpoint] ?? .unknown }
        }
    }
}

/// Coverage for the post-scan store reconcile
/// (`PlatformWalletManager.runCoreTxoReconcile`) against a fake engine,
/// driven exactly the way the manager drives it — engine reads on the
/// calling thread, store steps on the persistence queue.
///
/// The safety properties under test: a row is marked spent only on the
/// engine's positive `knownUncredited` verdict; a coin the store lacks is
/// inserted only when validated, owned, and mature; absence from either
/// side never changes a row; nothing is deleted, nothing is un-marked; the
/// run is idempotent and wallet-scoped; and a repaired store restores
/// nothing for the repaired coin across relaunches.
@MainActor
final class CoreTxoReconcileTests: XCTestCase {
    private let walletId = Data(repeating: 0x31, count: 32)
    private let otherWalletId = Data(repeating: 0x32, count: 32)
    private let tipHeight: UInt32 = 2_535_898
    private let fixtureAddress = "yReconcileFixtureAddr"
    private let fixtureScript = Data([0x76, 0xa9, 0x14] + [UInt8](repeating: 0x5a, count: 20) + [0x88, 0xac])

    private var bip44: CoreAccountKey {
        CoreAccountKey(
            typeTag: 0, standardTag: 0, index: 0, registrationIndex: 0, keyClass: 0,
            userIdentityId: Data(count: 32), friendIdentityId: Data(count: 32)
        )
    }

    private var coinJoin: CoreAccountKey {
        CoreAccountKey(
            typeTag: 1, standardTag: 0, index: 0, registrationIndex: 0, keyClass: 0,
            userIdentityId: Data(count: 32), friendIdentityId: Data(count: 32)
        )
    }

    private func txid(_ byte: UInt8) -> Data { Data(repeating: byte, count: 32) }

    private func makeHandler(
        modelFetcher: ModelFetching = LiveModelFetcher()
    ) throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet,
            modelFetcher: modelFetcher
        )
        return (handler, container)
    }

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

    /// A wallet row with a BIP44 account (and, when asked, a CoinJoin one).
    @discardableResult
    private func seedWallet(
        in container: ModelContainer,
        walletId: Data? = nil,
        withCoinJoinAccount: Bool = false
    ) throws -> PersistentWallet {
        let walletId = walletId ?? self.walletId
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "BIP44 Account")
        // Xpub bytes make the wallet restorable: the load path emits a
        // wallet only through accounts it can rebuild keys for.
        account.accountExtendedPubKeyBytes = Data(repeating: walletId[0], count: 78)
        account.userIdentityId = Data(count: 32)
        account.friendIdentityId = Data(count: 32)
        context.insert(account)
        if withCoinJoinAccount {
            let cj = PersistentAccount(wallet: wallet, accountType: 1, accountIndex: 0, accountTypeName: "CoinJoin")
            cj.accountExtendedPubKeyBytes = Data(repeating: walletId[0] &+ 1, count: 78)
            cj.userIdentityId = Data(count: 32)
            cj.friendIdentityId = Data(count: 32)
            context.insert(cj)
        }
        try context.save()
        return wallet
    }

    /// An unspent, confirmed TXO row of `walletId` under its BIP44 account.
    private func seedUnspentTxo(
        in container: ModelContainer,
        walletId: Data? = nil,
        txid: Data,
        vout: UInt32 = 0,
        amount: UInt64 = 19_549,
        isSpent: Bool = false,
        pendingSpender: Data? = nil
    ) throws {
        let walletId = walletId ?? self.walletId
        let context = ModelContext(container)
        let accounts = try context.fetch(FetchDescriptor<PersistentAccount>(
            predicate: #Predicate { $0.wallet.walletId == walletId && $0.accountType == 0 }
        ))
        let account = try XCTUnwrap(accounts.first)
        let tx = PersistentTransaction(
            txid: txid,
            transactionData: Data(repeating: 0x04, count: 10),
            context: 3,
            blockHeight: 2_391_743,
            netAmount: Int64(amount)
        )
        context.insert(tx)
        let txo = PersistentTxo(
            transaction: tx,
            vout: vout,
            amount: amount,
            address: fixtureAddress,
            scriptPubKey: fixtureScript,
            height: 2_391_743
        )
        txo.account = account
        txo.walletId = walletId
        txo.isConfirmed = true
        txo.isSpent = isSpent
        context.insert(txo)
        if let pendingSpender {
            context.insert(PersistentPendingInput(
                outpoint: txo.outpoint,
                inputIndex: 0,
                spendingTxid: pendingSpender,
                spendingTransaction: nil,
                walletId: walletId
            ))
        }
        try context.save()
    }

    private func engineUtxo(
        account: CoreAccountKey? = nil,
        txid: Data,
        vout: UInt32 = 0,
        amount: UInt64 = 19_549,
        height: UInt32 = 2_391_743,
        address: String? = nil,
        script: Data? = nil,
        isConfirmed: Bool = true
    ) -> CoreEngineUtxo {
        CoreEngineUtxo(
            account: account ?? bip44,
            txid: txid,
            vout: vout,
            amount: amount,
            address: address ?? fixtureAddress,
            scriptPubKey: script ?? fixtureScript,
            height: height,
            isConfirmed: isConfirmed,
            isInstantLocked: false,
            isCoinbase: false,
            isLocked: false
        )
    }

    private func txo(_ container: ModelContainer, txid: Data, vout: UInt32 = 0) throws -> PersistentTxo? {
        let outpoint = PersistentTxo.makeOutpoint(txid: txid, vout: vout)
        return try ModelContext(container).fetch(
            FetchDescriptor<PersistentTxo>(predicate: #Predicate { $0.outpoint == outpoint })
        ).first
    }

    private func txoCount(_ container: ModelContainer) throws -> Int {
        try ModelContext(container).fetchCount(FetchDescriptor<PersistentTxo>())
    }

    private func pendingCount(_ container: ModelContainer) throws -> Int {
        try ModelContext(container).fetchCount(FetchDescriptor<PersistentPendingInput>())
    }

    private func run(
        _ handler: PlatformWalletPersistenceHandler,
        engine: FakeCoreTxoEngine,
        walletId: Data? = nil,
        pageSize: Int = 2,
        isCancelled: @Sendable @escaping () -> Bool = { false }
    ) -> CoreTxoReconcileReport {
        PlatformWalletManager.runCoreTxoReconcile(
            walletId: walletId ?? self.walletId,
            tipHeight: tipHeight,
            pageSize: pageSize,
            engine: engine,
            handler: handler,
            isCancelled: isCancelled
        )
    }

    /// Drives the FFI load path and returns the restored UTXO count of the
    /// single wallet entry — what the engine would be handed at launch.
    private func restoredUtxoCount(_ handler: PlatformWalletPersistenceHandler) throws -> Int {
        let (entries, count, errored) = handler.loadWalletList()
        XCTAssertFalse(errored)
        XCTAssertEqual(count, 1)
        let entriesPtr = try XCTUnwrap(entries)
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entriesPtr)) }
        return Int(entriesPtr[0].utxos_count)
    }

    // MARK: 1. Positive engine evidence marks a local unspent row spent

    func testPositiveEngineVerdictMarksALocalUnspentRowSpent() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try seedUnspentTxo(in: container, txid: txid(0x71), pendingSpender: txid(0x7f))
        let outpoint = PersistentTxo.makeOutpoint(txid: txid(0x71), vout: 0)
        let engine = FakeCoreTxoEngine(verdicts: [outpoint: .knownUncredited])

        let report = run(handler, engine: engine)

        XCTAssertTrue(report.completed)
        XCTAssertEqual(report.storeRows, 1)
        XCTAssertEqual(report.flipped, 1)
        XCTAssertEqual(report.flippedDuffs, 19_549)
        XCTAssertEqual(report.inserted, 0)
        let coin = try XCTUnwrap(txo(container, txid: txid(0x71)))
        XCTAssertTrue(coin.isSpent)
        XCTAssertNil(coin.spendingTransaction, "no spender is invented")
        XCTAssertEqual(try pendingCount(container), 0, "claims on a settled coin are dropped")
        XCTAssertEqual(try restoredUtxoCount(handler), 0)
        // The engine was asked about exactly this coin, with the store's
        // own account and script — the ownership it checks.
        XCTAssertEqual(engine.classified.count, 1)
        XCTAssertEqual(engine.classified.first?.account, bip44)
        XCTAssertEqual(engine.classified.first?.scriptPubKey, fixtureScript)
    }

    /// The engine is asked off the persistence queue and the verdict is
    /// applied on it; a persistence round that opens AND commits in that gap
    /// can re-credit the very coin (a reorg of its spender hands it back in
    /// `utxos_added`). The apply must refuse a verdict read before that
    /// round, and the page is classified again against the store as it is
    /// now — here the engine holds the coin again, so nothing is flipped.
    func testAVerdictReadBeforeAnInterveningRoundIsNotAppliedAndThePageIsReclassified() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try seedUnspentTxo(in: container, txid: txid(0x72))
        let outpoint = PersistentTxo.makeOutpoint(txid: txid(0x72), vout: 0)
        let engine = FakeCoreTxoEngine(verdicts: [outpoint: .knownUncredited])
        let walletId = self.walletId
        let classifies = Counter()
        engine.onClassify = {
            // Only the first classify sees the world move: a round commits
            // between this read and the apply, and after it the engine
            // holds the coin again.
            guard classifies.next() == 1 else { return }
            handler.beginChangeset(walletId: walletId)
            _ = handler.endChangeset(walletId: walletId, success: true)
            engine.setVerdict(outpoint, .unspent)
        }

        let report = run(handler, engine: engine)

        XCTAssertTrue(report.completed)
        XCTAssertEqual(report.staleRetries, 1, "the first verdict was read before the round and refused")
        XCTAssertEqual(engine.classifyCalls, 2, "the page is classified again after the refusal")
        XCTAssertEqual(report.flipped, 0)
        XCTAssertEqual(report.unspent, 1)
        XCTAssertEqual(report.storeRows, 1)
        let coin = try XCTUnwrap(txo(container, txid: txid(0x72)))
        XCTAssertFalse(coin.isSpent, "a coin the engine re-credited in the gap stays unspent")
    }

    private final class Counter: @unchecked Sendable {
        private let lock = NSLock()
        private var value = 0
        func next() -> Int { lock.withLock { value += 1; return value } }
    }

    // MARK: 2. Absence from both inventories changes nothing

    func testARowAbsentFromBothInventoriesIsLeftUnchanged() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try seedUnspentTxo(in: container, txid: txid(0x72))
        try seedUnspentTxo(in: container, txid: txid(0x73))
        let notOwned = PersistentTxo.makeOutpoint(txid: txid(0x73), vout: 0)
        let engine = FakeCoreTxoEngine(verdicts: [notOwned: .notOwned])

        let report = run(handler, engine: engine)

        XCTAssertTrue(report.completed)
        XCTAssertEqual(report.storeRows, 2)
        XCTAssertEqual(report.unknown, 1)
        XCTAssertEqual(report.notOwned, 1)
        XCTAssertEqual(report.mutations, 0)
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: txid(0x72))).isSpent)
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: txid(0x73))).isSpent)
        XCTAssertEqual(try txoCount(container), 2, "nothing is ever deleted")
        XCTAssertEqual(try restoredUtxoCount(handler), 2)
    }

    // MARK: 3. Engine coin missing from the store is inserted, validated

    func testAnEngineCoinMissingFromTheStoreIsInsertedWhenValid() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container, withCoinJoinAccount: true)
        let valid = engineUtxo(txid: txid(0x74))
        let coinJoinValid = engineUtxo(account: coinJoin, txid: txid(0x75), amount: 100_001)
        let engine = FakeCoreTxoEngine(inventory: [valid, coinJoinValid])

        let report = run(handler, engine: engine)

        XCTAssertTrue(report.completed)
        XCTAssertEqual(report.engineRows, 2)
        XCTAssertEqual(report.inserted, 2)
        XCTAssertEqual(report.insertedDuffs, 19_549 + 100_001)
        let coin = try XCTUnwrap(txo(container, txid: txid(0x74)))
        XCTAssertFalse(coin.isSpent)
        XCTAssertTrue(coin.isConfirmed)
        XCTAssertEqual(coin.amount, 19_549)
        XCTAssertEqual(coin.address, fixtureAddress)
        XCTAssertEqual(coin.scriptPubKey, fixtureScript)
        XCTAssertEqual(coin.height, 2_391_743)
        XCTAssertEqual(coin.walletId, walletId)
        XCTAssertEqual(coin.account?.accountType, 0)
        XCTAssertEqual(coin.transaction?.txid, txid(0x74), "a stub parent row holds the relationship")
        XCTAssertEqual(coin.transaction?.transactionData, Data())
        let mixed = try XCTUnwrap(txo(container, txid: txid(0x75)))
        XCTAssertEqual(mixed.account?.accountType, 1, "filed under the account the engine named")
        XCTAssertEqual(try restoredUtxoCount(handler), 2)
    }

    func testTheHealPassRefusesImmatureUnconfirmedUnresolvedAndMalformedCoins() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container) // BIP44 only: no CoinJoin account row
        let immature = engineUtxo(txid: txid(0x76), height: tipHeight - 50)
        let atGate = engineUtxo(txid: txid(0x77), height: tipHeight - 99) // exactly 100 confirmations
        // Deep enough, but the engine itself does not call it confirmed.
        let engineUnconfirmed = engineUtxo(txid: txid(0x78), isConfirmed: false)
        let unresolved = engineUtxo(account: coinJoin, txid: txid(0x79))
        let noScript = engineUtxo(txid: txid(0x7a), script: Data())
        let noAddress = engineUtxo(txid: txid(0x7b), address: "")
        let unconfirmed = engineUtxo(txid: txid(0x7c), height: 0)
        let engine = FakeCoreTxoEngine(
            inventory: [immature, atGate, engineUnconfirmed, unresolved, noScript, noAddress, unconfirmed]
        )

        let report = run(handler, engine: engine)

        XCTAssertTrue(report.completed)
        XCTAssertEqual(report.engineRows, 7)
        XCTAssertEqual(report.inserted, 1)
        XCTAssertEqual(report.skippedImmature, 3)
        XCTAssertEqual(report.skippedUnresolvedAccount, 1)
        XCTAssertEqual(report.skippedInvalid, 2)
        XCTAssertNotNil(try txo(container, txid: txid(0x77)))
        for byte: UInt8 in [0x76, 0x78, 0x79, 0x7a, 0x7b, 0x7c] {
            XCTAssertNil(try txo(container, txid: txid(byte)), "coin \(byte) must not be healed")
        }
    }

    /// A coin the engine holds and the store lacks, whose outpoint a
    /// pending-input claim already names: the heal inserts the row and the
    /// drain writes it spent on the spot. Nothing was repaired — the engine
    /// still holds the coin — but a row was written, so the run reports it
    /// as a mutation rather than as a clean, all-zero pass.
    func testAHealTheDrainWritesSpentIsReportedAsAMutation() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        // The spender is already a confirmed row of the store; only the coin
        // it consumed is missing (the funding record never reached the
        // store), so its input claim is still pending.
        let outpoint = PersistentTxo.makeOutpoint(txid: txid(0x7d), vout: 0)
        let context = ModelContext(container)
        let spender = PersistentTransaction(
            txid: txid(0x7e),
            transactionData: Data(repeating: 0x05, count: 10),
            context: 3,
            blockHeight: 2_391_800,
            netAmount: -19_549
        )
        context.insert(spender)
        context.insert(PersistentPendingInput(
            outpoint: outpoint,
            inputIndex: 0,
            spendingTxid: txid(0x7e),
            spendingTransaction: spender,
            walletId: walletId
        ))
        try context.save()
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(txid: txid(0x7d))])

        let report = run(handler, engine: engine)

        XCTAssertTrue(report.completed)
        XCTAssertEqual(report.engineRows, 1)
        XCTAssertEqual(report.healedSpent, 1)
        XCTAssertEqual(report.inserted, 0)
        XCTAssertEqual(report.flipped, 0)
        XCTAssertEqual(report.mutations, 1, "a row was written, even though nothing was repaired")
        let coin = try XCTUnwrap(txo(container, txid: txid(0x7d)))
        XCTAssertTrue(coin.isSpent)
        XCTAssertEqual(try pendingCount(container), 0, "the claim was consumed by the drain")
        XCTAssertEqual(try restoredUtxoCount(handler), 0)
    }

    /// A store read that fails inside the heal pass is a failure, not a
    /// skip: the account lookup cannot tell "no such account" from "could
    /// not read", so it must not answer the former when the latter
    /// happened. The run stops, counts the store failure, inserts nothing
    /// — and reports it as incomplete rather than as an unresolved account.
    func testTheRunStopsWhenTheAccountLookupCannotRead() throws {
        let injector = FetchFaultInjector(faulting: PersistentAccount.self)
        let (handler, container) = try makeHandler(modelFetcher: injector)
        try seedWallet(in: container)
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(txid: txid(0x83))])

        let report = run(handler, engine: engine)

        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.storeFailures, 1)
        XCTAssertEqual(report.skippedUnresolvedAccount, 0, "a failed read is not a missing account")
        XCTAssertEqual(report.inserted, 0)
        XCTAssertNil(try txo(container, txid: txid(0x83)))
        XCTAssertTrue(injector.observedReads.contains("PersistentAccount"))
    }

    /// The heal pass asks the store whether it already holds each engine
    /// coin. A read that fails there must not read as "absent": that would
    /// insert a row the store may well hold. The step fails and the run
    /// stops before any insert.
    func testTheRunStopsWhenTheHealCannotReadTheStore() throws {
        let injector = FetchFaultInjector(faulting: PersistentTxo.self)
        let (handler, container) = try makeHandler(modelFetcher: injector)
        try seedWallet(in: container)
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(txid: txid(0x84))])

        let report = run(handler, engine: engine)

        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.storeFailures, 1)
        XCTAssertEqual(report.alreadyPresent, 0, "a failed read is not a hit either")
        XCTAssertEqual(report.inserted, 0)
        XCTAssertEqual(try txoCount(container), 0)
    }

    /// The classify pass re-reads each `knownUncredited` row before it
    /// flips it. A read that fails there must not read as "stale" and let
    /// the step report `.done`: the step fails, nothing staged is kept, and
    /// the row stays as it was. The page read itself is served; only the
    /// per-row lookup behind it faults.
    func testTheRunStopsWhenTheFlipLookupCannotRead() throws {
        let injector = FetchFaultInjector(faulting: PersistentTxo.self, afterServing: 1)
        let (handler, container) = try makeHandler(modelFetcher: injector)
        try seedWallet(in: container)
        try seedUnspentTxo(in: container, txid: txid(0x85))
        let outpoint = PersistentTxo.makeOutpoint(txid: txid(0x85), vout: 0)
        let engine = FakeCoreTxoEngine(verdicts: [outpoint: .knownUncredited])

        let report = run(handler, engine: engine)

        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.storeFailures, 1)
        XCTAssertEqual(report.storeRows, 1, "the page itself was read")
        XCTAssertEqual(report.flipped, 0)
        XCTAssertEqual(report.staleRetries, 0, "a failed read is not a stale page either")
        let coin = try XCTUnwrap(txo(container, txid: txid(0x85)))
        XCTAssertFalse(coin.isSpent)
    }

    // MARK: 4. Nothing runs before the scan is complete

    func testTheSteadyStateGateRefusesAnUnfinishedScan() {
        func progress(_ state: PlatformSpvSyncState, filters: (UInt32, UInt32)?) -> PlatformSpvSyncProgress {
            PlatformSpvSyncProgress(
                overallState: state,
                overallPercentage: 0,
                headers: PlatformSpvSubProgress(state: state, currentHeight: 2_535_898, targetHeight: 2_535_898, percentage: 1),
                filterHeaders: nil,
                filters: filters.map {
                    PlatformSpvSubProgress(state: state, currentHeight: $0.0, targetHeight: $0.1, percentage: 0)
                },
                masternodes: nil
            )
        }
        XCTAssertFalse(PlatformWalletManager.isSteadySyncState(progress(.syncing, filters: (199_999, 2_535_898))))
        XCTAssertFalse(PlatformWalletManager.isSteadySyncState(progress(.waitingForConnections, filters: nil)))
        XCTAssertFalse(PlatformWalletManager.isSteadySyncState(progress(.error, filters: (2_535_898, 2_535_898))))
        XCTAssertFalse(
            PlatformWalletManager.isSteadySyncState(progress(.waitForEvents, filters: (2_535_800, 2_535_898))),
            "waiting for events with the filter phase behind its target is a scan still running"
        )
        XCTAssertFalse(
            PlatformWalletManager.isSteadySyncState(progress(.waitForEvents, filters: nil)),
            "no filter phase at all proves nothing"
        )
        XCTAssertTrue(PlatformWalletManager.isSteadySyncState(progress(.synced, filters: (2_535_898, 2_535_898))))
        XCTAssertTrue(
            PlatformWalletManager.isSteadySyncState(progress(.waitForEvents, filters: (2_535_898, 2_535_898))),
            "dash-spv's fully synced steady state"
        )
        XCTAssertEqual(PlatformWalletManager.scanTipHeight(progress(.synced, filters: (2_535_898, 2_535_898))), 2_535_898)
        XCTAssertEqual(
            PlatformWalletManager.scanTipHeight(progress(.synced, filters: nil)),
            2_535_898,
            "falls back to the header tip"
        )
    }

    func testTheManagerRefusesToReconcileAnUnknownOrUnconfiguredWallet() async throws {
        let unconfigured = PlatformWalletManager()
        let before = try await unconfigured.reconcileCoreTxoStore(for: walletId)
        XCTAssertEqual(before, .skipped(.notConfigured))

        let manager = PlatformWalletManager.makeForTesting(
            handle: 42,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: { _ in PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil) },
                platformAddressSyncStop: { _ in PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil) },
                shieldedSyncStop: { _ in PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil) },
                dashPaySyncStop: { _ in PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil) },
                dpnsSyncStop: { _ in PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil) },
                destroy: { _ in PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil) }
            )
        )
        // Configured, but no persistence handler and no loaded wallet: the
        // gate refuses before any native read.
        let outcome = try await manager.reconcileCoreTxoStore(for: walletId)
        XCTAssertEqual(outcome, .skipped(.notConfigured))
        _ = await manager.shutdown()
        PlatformWalletManager.destroyQueue.sync {}
    }

    // MARK: 5. Idempotent

    func testASecondRunChangesNothing() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try seedUnspentTxo(in: container, txid: txid(0x81))
        let flipped = PersistentTxo.makeOutpoint(txid: txid(0x81), vout: 0)
        let healed = engineUtxo(txid: txid(0x82))
        let engine = FakeCoreTxoEngine(inventory: [healed], verdicts: [flipped: .knownUncredited])

        let first = run(handler, engine: engine)
        XCTAssertEqual(first.mutations, 2)

        let second = run(handler, engine: engine)
        XCTAssertTrue(second.completed)
        XCTAssertEqual(second.mutations, 0)
        XCTAssertEqual(second.alreadyPresent, 1)
        XCTAssertEqual(second.storeRows, 1, "the healed coin is the only unspent row left")
        XCTAssertEqual(second.unknown, 1, "and the fake has no verdict for it")
        XCTAssertEqual(try txoCount(container), 2)
    }

    // MARK: 6. Wallet- and account-scoped

    func testTheReconcileTouchesOnlyTheWalletItWasAskedAbout() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try seedWallet(in: container, walletId: otherWalletId)
        try seedUnspentTxo(in: container, txid: txid(0x91))
        try seedUnspentTxo(in: container, walletId: otherWalletId, txid: txid(0x92))
        let mine = PersistentTxo.makeOutpoint(txid: txid(0x91), vout: 0)
        let theirs = PersistentTxo.makeOutpoint(txid: txid(0x92), vout: 0)
        // The fake would flip both if asked; only one may be asked.
        let engine = FakeCoreTxoEngine(verdicts: [mine: .knownUncredited, theirs: .knownUncredited])

        let report = run(handler, engine: engine)

        XCTAssertEqual(report.storeRows, 1)
        XCTAssertEqual(report.flipped, 1)
        XCTAssertTrue(try XCTUnwrap(txo(container, txid: txid(0x91))).isSpent)
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: txid(0x92))).isSpent, "the other wallet's coin is untouched")
        XCTAssertEqual(engine.classified.map(\.outpoint), [mine])
    }

    // MARK: 8. Repeated restart without rescan does not resurrect repaired funds

    func testRepeatedRelaunchesRestoreNothingForARepairedCoin() throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("txo-reconcile-\(UUID().uuidString).sqlite")
        defer {
            for suffix in ["", "-wal", "-shm"] {
                try? FileManager.default.removeItem(at: URL(fileURLWithPath: url.path + suffix))
            }
        }
        let outpoint = PersistentTxo.makeOutpoint(txid: txid(0xa1), vout: 0)
        do {
            let (handler, container) = try makeHandler(url: url)
            try seedWallet(in: container)
            try seedUnspentTxo(in: container, txid: txid(0xa1))
            XCTAssertEqual(try restoredUtxoCount(handler), 1, "the phantom the engine would be handed")
            let report = run(handler, engine: FakeCoreTxoEngine(verdicts: [outpoint: .knownUncredited]))
            XCTAssertEqual(report.flipped, 1)
            XCTAssertEqual(try restoredUtxoCount(handler), 0)
        }
        for _ in 0..<2 {
            let (handler, container) = try makeHandler(url: url)
            XCTAssertTrue(try XCTUnwrap(txo(container, txid: txid(0xa1))).isSpent)
            XCTAssertEqual(try restoredUtxoCount(handler), 0, "a relaunch without a rescan restores nothing")
        }
    }

    // MARK: 9. A correct wallet stays untouched

    func testAConsistentStoreYieldsZeroMutations() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try seedUnspentTxo(in: container, txid: txid(0xb1))
        try seedUnspentTxo(in: container, txid: txid(0xb2), isSpent: true)
        let unspent = PersistentTxo.makeOutpoint(txid: txid(0xb1), vout: 0)
        // The engine holds exactly the store's unspent coin, and says so.
        let engine = FakeCoreTxoEngine(
            inventory: [engineUtxo(txid: txid(0xb1))],
            verdicts: [unspent: .unspent]
        )

        let report = run(handler, engine: engine)

        XCTAssertTrue(report.completed)
        XCTAssertEqual(report.mutations, 0)
        XCTAssertEqual(report.alreadyPresent, 1)
        XCTAssertEqual(report.unspent, 1)
        XCTAssertEqual(report.storeRows, 1, "spent rows are never even asked about")
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: txid(0xb1))).isSpent)
        XCTAssertTrue(try XCTUnwrap(txo(container, txid: txid(0xb2))).isSpent, "never un-marked")
        XCTAssertEqual(try txoCount(container), 2)
    }

    // MARK: Never un-mark, never delete

    func testASpentRowTheEngineStillHoldsIsNeverUnmarked() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try seedUnspentTxo(in: container, txid: txid(0xc1), isSpent: true)
        // The engine claims to hold the coin the store says is spent: a live
        // spend racing the engine is indistinguishable from lost residue,
        // and un-marking mid-payment would let the wallet double-spend.
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(txid: txid(0xc1))])

        let report = run(handler, engine: engine)

        XCTAssertEqual(report.alreadyPresent, 1)
        XCTAssertEqual(report.mutations, 0)
        XCTAssertTrue(try XCTUnwrap(txo(container, txid: txid(0xc1))).isSpent)
        XCTAssertEqual(try restoredUtxoCount(handler), 0)
    }

    // MARK: Run shape

    func testTheRunStopsAtTheFirstFailedEngineReadAndKeepsWhatLanded() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try seedUnspentTxo(in: container, txid: txid(0xd1))
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(txid: txid(0xd2))])
        engine.failClassify = true

        let report = run(handler, engine: engine)

        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.transportFailures, 1)
        XCTAssertEqual(report.inserted, 1, "the heal pass landed before the classify pass failed")
        XCTAssertNotNil(try txo(container, txid: txid(0xd2)))
        XCTAssertFalse(try XCTUnwrap(txo(container, txid: txid(0xd1))).isSpent)
    }

    func testTheHealPassWalksEveryPageOfTheInventory() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        let inventory = (0..<5).map { engineUtxo(txid: txid(0xe0 + UInt8($0))) }
        let engine = FakeCoreTxoEngine(inventory: inventory)

        let report = run(handler, engine: engine, pageSize: 2)

        XCTAssertEqual(engine.pageCalls, 3)
        XCTAssertEqual(report.engineRows, 5)
        XCTAssertEqual(report.inserted, 5)
        XCTAssertEqual(try txoCount(container), 5)
    }

    func testTheClassifyPassWalksEveryUnspentRowAcrossFlips() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        var verdicts: [Data: CoreOutpointClass] = [:]
        for i in 0..<5 {
            try seedUnspentTxo(in: container, txid: txid(0xf0 + UInt8(i)))
            verdicts[PersistentTxo.makeOutpoint(txid: txid(0xf0 + UInt8(i)), vout: 0)] = .knownUncredited
        }
        let engine = FakeCoreTxoEngine(verdicts: verdicts)

        let report = run(handler, engine: engine, pageSize: 2)

        XCTAssertTrue(report.completed)
        XCTAssertEqual(report.storeRows, 5)
        XCTAssertEqual(report.flipped, 5, "flipping rows out of the page does not skip the next ones")
        XCTAssertEqual(try restoredUtxoCount(handler), 0)
    }
}
