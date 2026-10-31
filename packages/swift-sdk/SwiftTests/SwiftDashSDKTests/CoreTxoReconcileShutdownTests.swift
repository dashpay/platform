import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// The native side of a reconcile run's bracket, faked: the start gate's
/// answer, and a record of every begin, every finish (with whether an
/// engine guard was still held when it was called) and the tip each engine
/// was made at.
final class FakeReconcileNative: @unchecked Sendable {
    private let lock = NSLock()
    let engine: any CoreTxoReconcileEngine
    private let holding: @Sendable () -> Bool
    private var _gate: CoreTxoReconcileGate
    private var _begins: [Data] = []
    private var _finishes: [(walletId: Data, ran: Bool, guardHeld: Bool)] = []
    private var _enginesMade = 0
    /// Runs inside each begin, before it answers.
    var onBegin: (@Sendable () -> Void)?

    init(
        engine: any CoreTxoReconcileEngine,
        holding: @escaping @Sendable () -> Bool,
        gate: CoreTxoReconcileGate = .run(tipHeight: 2_535_898)
    ) {
        self.engine = engine
        self.holding = holding
        _gate = gate
    }

    convenience init(engine: FakeCoreTxoEngine, gate: CoreTxoReconcileGate = .run(tipHeight: 2_535_898)) {
        self.init(engine: engine, holding: { engine.isHoldingVerdicts }, gate: gate)
    }

    var begins: [Data] { lock.withLock { _begins } }
    var finishes: [(walletId: Data, ran: Bool, guardHeld: Bool)] { lock.withLock { _finishes } }
    var enginesMade: Int { lock.withLock { _enginesMade } }

    func calls() -> PlatformWalletNativeTxoReconcileCalls {
        PlatformWalletNativeTxoReconcileCalls(
            begin: { [self] _, walletId in
                onBegin?()
                return lock.withLock {
                    _begins.append(walletId)
                    return _gate
                }
            },
            finish: { [self] _, walletId, ran in
                let held = holding()
                lock.withLock { _finishes.append((walletId, ran, held)) }
            },
            makeEngine: { [self] _, _ in
                lock.withLock { _enginesMade += 1 }
                return engine
            }
        )
    }
}

/// The run's bracket and its stops: a granted begin is always finished,
/// after the last engine guard is released, whatever ends the run — a
/// failure, the epoch, a cancelled caller; a skip claims nothing and is
/// never finished; the run stops between pages when cancelled, defers a
/// guard-free store step while a Rust persistence round is open, and the
/// manager refuses it once shutdown has begun.
@MainActor
final class CoreTxoReconcileShutdownTests: XCTestCase {
    private let walletId = Data(repeating: 0x51, count: 32)
    private let fixtureScript = Data([0x76, 0xa9, 0x14] + [UInt8](repeating: 0x5b, count: 20) + [0x88, 0xac])

    private var bip44: CoreAccountKey {
        CoreAccountKey(
            typeTag: 0, standardTag: 0, index: 0, registrationIndex: 0, keyClass: 0,
            userIdentityId: Data(count: 32), friendIdentityId: Data(count: 32)
        )
    }

    private func txid(_ byte: UInt8) -> Data { Data(repeating: byte, count: 32) }

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "BIP44 Account")
        account.userIdentityId = Data(count: 32)
        account.friendIdentityId = Data(count: 32)
        context.insert(account)
        try context.save()
        return (handler, container)
    }

    /// Unspent, confirmed store rows of the wallet under its BIP44 account.
    private func seedUnspentTxos(in container: ModelContainer, _ bytes: [UInt8]) throws {
        let context = ModelContext(container)
        let account = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentAccount>()).first)
        for byte in bytes {
            let tx = PersistentTransaction(
                txid: txid(byte), transactionData: Data(repeating: 0x04, count: 10),
                context: 3, blockHeight: 2_391_743, netAmount: 19_549
            )
            context.insert(tx)
            let row = PersistentTxo(
                transaction: tx, vout: 0, amount: 19_549, address: "yShutdownFixtureAddr",
                scriptPubKey: fixtureScript, height: 2_391_743
            )
            row.account = account
            row.walletId = walletId
            row.isConfirmed = true
            context.insert(row)
        }
        try context.save()
    }

    private func engineUtxo(_ byte: UInt8) -> CoreEngineUtxo {
        CoreEngineUtxo(
            account: bip44, txid: txid(byte), vout: 0, amount: 19_549,
            address: "yShutdownFixtureAddr", scriptPubKey: fixtureScript, height: 2_391_743,
            isConfirmed: true, isInstantLocked: false, isCoinbase: false, isLocked: false
        )
    }

    private func txoCount(_ container: ModelContainer) throws -> Int {
        try ModelContext(container).fetchCount(FetchDescriptor<PersistentTxo>())
    }

    private nonisolated static func okTeardownCalls() -> PlatformWalletNativeTeardownCalls {
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
            dashPaySyncStop: ok, dpnsSyncStop: ok, destroy: ok
        )
    }

    /// A manager with a fake handle, `handler`, the given reconcile calls,
    /// and the fixture wallet loaded through a create table whose native
    /// handle is `NULL_HANDLE` (never a live registry entry).
    private func makeManager(
        handle: Handle,
        handler: PlatformWalletPersistenceHandler,
        native: FakeReconcileNative
    ) async throws -> PlatformWalletManager {
        let manager = PlatformWalletManager.makeForTesting(
            handle: handle, calls: Self.okTeardownCalls(), persistenceHandler: handler)
        let walletId = self.walletId
        manager.nativeCreateCalls = PlatformWalletNativeCreateCalls(createFromMnemonic: { _, _ in
            (
                PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil),
                NULL_HANDLE,
                walletId
            )
        })
        manager.nativeTxoReconcileCalls = native.calls()
        let wallet = try await manager.createWallet(mnemonic: "m", network: .testnet)
        XCTAssertEqual(wallet.walletId, walletId)
        return manager
    }

    // MARK: The walk's own stops

    func testACancelledRunDoesNothingAndSaysSo() throws {
        let (handler, container) = try makeHandler()
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(0x11)])

        let report = PlatformWalletManager.runCoreTxoReconcile(
            walletId: walletId, engine: engine, handler: handler,
            isCancelled: { true }
        )

        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.mutations, 0)
        XCTAssertEqual(engine.pageCalls, 0, "cancellation is checked before the first engine read")
        XCTAssertEqual(try txoCount(container), 0)
    }

    func testARunCancelledMidWayStopsAtThePageBoundaryAndKeepsWhatLanded() throws {
        let (handler, container) = try makeHandler()
        let engine = FakeCoreTxoEngine(inventory: (0..<6).map { engineUtxo(0x20 + UInt8($0)) })
        let checks = Counter()

        let report = PlatformWalletManager.runCoreTxoReconcile(
            walletId: walletId, pageSize: 2, engine: engine, handler: handler,
            // The run checks once before each engine read, and twice per
            // store step — before queueing it and again on the persistence
            // queue: the first three admit page one and its heal, the
            // fourth (before page two) reports the epoch bumped.
            isCancelled: { checks.next() >= 4 }
        )

        XCTAssertFalse(report.completed)
        XCTAssertEqual(engine.pageCalls, 1)
        XCTAssertEqual(report.inserted, 2, "the page that landed stays")
        XCTAssertEqual(try txoCount(container), 2)
    }

    /// A cancellation that lands while a page's verdicts are held stops the
    /// run before the apply, and still releases the guard.
    func testACancellationUnderAHeldGuardReleasesItAndWritesNothing() throws {
        let (handler, container) = try makeHandler()
        try seedUnspentTxos(in: container, [0x61])
        let outpoint = PersistentTxo.makeOutpoint(txid: txid(0x61), vout: 0)
        let engine = FakeCoreTxoEngine(verdicts: [outpoint: .knownUncredited])
        let cancelled = SyncGenerationCounter()
        engine.onClassify = { cancelled.bump() }

        let report = PlatformWalletManager.runCoreTxoReconcile(
            walletId: walletId, engine: engine, handler: handler,
            isCancelled: { cancelled.current() != 0 }
        )

        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.flipped, 0)
        XCTAssertEqual(engine.releases, 2, "the heal page's guard, and the classified page's")
        XCTAssertFalse(engine.isHoldingVerdicts)
        let row = try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTxo>()).first)
        XCTAssertFalse(row.isSpent)
    }

    /// A Rust round open on the persistence queue defers every guard-free
    /// store step: the run retries until the round closes, then completes
    /// normally.
    func testAStepDeferredBehindAnOpenRoundCompletesOnceTheRoundCloses() throws {
        let (handler, container) = try makeHandler()
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(0x31)])

        handler.beginChangeset(walletId: walletId)
        let finished = expectation(description: "reconcile finished")
        let box = ReportBox()
        let walletId = self.walletId
        DispatchQueue.global(qos: .utility).async {
            let report = PlatformWalletManager.runCoreTxoReconcile(
                walletId: walletId, engine: engine, handler: handler,
                isCancelled: { false }
            )
            box.set(report)
            finished.fulfill()
        }
        // Let the run hit the open round a few times, then close it.
        Thread.sleep(forTimeInterval: 0.3)
        XCTAssertNil(box.get(), "the run must not have completed while the round was open")
        _ = handler.endChangeset(walletId: walletId, success: true)
        wait(for: [finished], timeout: 15)

        let report = try XCTUnwrap(box.get())
        XCTAssertTrue(report.completed)
        XCTAssertGreaterThan(report.retries, 0)
        XCTAssertEqual(report.inserted, 1)
        XCTAssertEqual(try txoCount(container), 1)
    }

    /// Teardown can release a page's engine guard while that page's apply
    /// is still waiting for the persistence queue: shutdown moves the epoch,
    /// waits a bounded time for the run, then destroys the manager, which
    /// drops every guard it still holds. The apply checks the stop again
    /// once it is on the queue, so that page writes no spend verdict — and
    /// the run still frees its token and finishes its claim.
    func testARunStoppedBeforeItsApplyReachesThePersistenceQueueWritesNoVerdict() throws {
        let (handler, container) = try makeHandler()
        try seedUnspentTxos(in: container, [0x64])
        let outpoint = PersistentTxo.makeOutpoint(txid: txid(0x64), vout: 0)
        let engine = FakeCoreTxoEngine(verdicts: [outpoint: .knownUncredited])
        let native = FakeReconcileNative(engine: engine)
        let stopsSeenOnQueue = Counter()
        // The stop lands after the run's own check that follows the
        // classification, and before its apply runs: the run reads as
        // stopped only on the persistence queue, once a page was classified.
        let isCancelled: @Sendable () -> Bool = {
            guard engine.classifyCalls > 0, Self.onPersistenceQueue() else { return false }
            _ = stopsSeenOnQueue.next()
            return true
        }

        let outcome = try PlatformWalletManager.performCoreTxoReconcileRun(
            handle: 42, walletId: walletId, calls: native.calls(), handler: handler,
            isCancelled: isCancelled)

        guard case .reconciled(let report) = outcome else {
            return XCTFail("expected a run, got \(String(describing: outcome))")
        }
        XCTAssertGreaterThan(stopsSeenOnQueue.next(), 1, "the apply asked on the persistence queue")
        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.flipped, 0)
        XCTAssertEqual(report.storeFailures, 0, "a stopped page is not a failed one")
        XCTAssertEqual(report.staleRetries, 0)
        XCTAssertEqual(engine.classifyCalls, 1)
        XCTAssertEqual(engine.releases, 2, "the classified page's token is still freed, as is the heal page's")
        XCTAssertFalse(engine.isHoldingVerdicts)
        XCTAssertEqual(native.finishes.count, 1, "the claim is still finished")
        XCTAssertEqual(native.finishes.first?.ran, true)
        XCTAssertEqual(native.finishes.first?.guardHeld, false)
        let row = try XCTUnwrap(ModelContext(container).fetch(FetchDescriptor<PersistentTxo>()).first)
        XCTAssertFalse(row.isSpent, "no spend verdict is written once the run is stopped")
    }

    // MARK: The begin / finish bracket

    func testASkippedBeginClaimsNothingAndIsNeverFinished() throws {
        let (handler, _) = try makeHandler()
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(0x12)])
        let native = FakeReconcileNative(
            engine: engine, gate: .skip(code: UInt8(TXO_RECONCILE_SKIP_ALREADY_RUNNING)))

        let outcome = try PlatformWalletManager.performCoreTxoReconcileRun(
            handle: 42, walletId: walletId, calls: native.calls(), handler: handler,
            isCancelled: { false })

        XCTAssertEqual(outcome, .skipped(.alreadyRunning))
        XCTAssertEqual(native.begins, [walletId])
        XCTAssertTrue(native.finishes.isEmpty, "a skip holds no claim to release")
        XCTAssertEqual(native.enginesMade, 0)
        XCTAssertEqual(engine.pageCalls, 0)
    }

    func testAnUnknownGateCodeThrowsWithoutAClaimToRelease() throws {
        let (handler, _) = try makeHandler()
        let engine = FakeCoreTxoEngine()
        let native = FakeReconcileNative(engine: engine, gate: .skip(code: 0xee))

        XCTAssertThrowsError(try PlatformWalletManager.performCoreTxoReconcileRun(
            handle: 42, walletId: walletId, calls: native.calls(), handler: handler,
            isCancelled: { false }))
        XCTAssertTrue(native.finishes.isEmpty)
        XCTAssertEqual(engine.pageCalls, 0)
    }

    /// A granted run makes one engine and is finished once, as a run that
    /// went ahead — even when it stopped on a failed classification — and
    /// only after its last guard is released.
    func testAGrantedRunIsFinishedAfterItsLastGuardIsReleased() throws {
        let (handler, container) = try makeHandler()
        try seedUnspentTxos(in: container, [0x62, 0x63])
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(0x13)])
        let native = FakeReconcileNative(engine: engine, gate: .run(tipHeight: 2_400_001))
        let classifies = Counter()
        // The second page's classification fails, after the first page's
        // guard was released.
        engine.onClassify = { if classifies.next() == 2 { engine.failClassify = true } }

        let outcome = try PlatformWalletManager.performCoreTxoReconcileRun(
            handle: 42, walletId: walletId, calls: native.calls(), handler: handler, pageSize: 1,
            isCancelled: { false })

        guard case .reconciled(let report) = outcome else {
            return XCTFail("expected a run, got \(String(describing: outcome))")
        }
        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.transportFailures, 1)
        XCTAssertEqual(report.inserted, 1)
        XCTAssertEqual(native.enginesMade, 1)
        XCTAssertEqual(native.finishes.count, 1)
        XCTAssertEqual(native.finishes.first?.ran, true)
        XCTAssertEqual(native.finishes.first?.guardHeld, false, "the guard goes before the finish resolves the manager")
        XCTAssertEqual(engine.events, ["heal", "release", "classify", "release", "classify"])
    }

    /// A run staled between a granted begin and its walk walks nothing and
    /// still releases the claim, as a run that did not go ahead.
    func testARunStaledAfterItsBeginIsFinishedAsNotRun() throws {
        let (handler, container) = try makeHandler()
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(0x14)])
        let native = FakeReconcileNative(engine: engine)
        let epoch = SyncGenerationCounter()
        let generation = epoch.current()
        native.onBegin = { epoch.bump() }

        let outcome = try PlatformWalletManager.performCoreTxoReconcileRun(
            handle: 42, walletId: walletId, calls: native.calls(), handler: handler,
            isCancelled: { epoch.current() != generation })

        XCTAssertNil(outcome)
        XCTAssertEqual(native.finishes.count, 1)
        XCTAssertEqual(native.finishes.first?.ran, false)
        XCTAssertEqual(engine.pageCalls, 0)
        XCTAssertEqual(try txoCount(container), 0)
    }

    /// A run staled before it began claims nothing at all.
    func testARunStaledBeforeItsBeginNeverClaims() throws {
        let (handler, _) = try makeHandler()
        let native = FakeReconcileNative(engine: FakeCoreTxoEngine())

        let outcome = try PlatformWalletManager.performCoreTxoReconcileRun(
            handle: 42, walletId: walletId, calls: native.calls(), handler: handler,
            isCancelled: { true })

        XCTAssertNil(outcome)
        XCTAssertTrue(native.begins.isEmpty)
        XCTAssertTrue(native.finishes.isEmpty)
    }

    /// A finish that fails cannot strand a guard: the release runs first
    /// and on its own.
    func testAFailedFinishStillFollowsTheGuardRelease() throws {
        let engine = FakeCoreTxoEngine()
        _ = try engine.classifyHeld([CoreOutpointOwnershipQuery(
            account: bip44, txid: txid(0x15), vout: 0, scriptPubKey: fixtureScript)])
        XCTAssertTrue(engine.isHoldingVerdicts)
        var heldAtFinish: Bool?

        PlatformWalletManager.endCoreTxoReconcileRun(walletId: walletId, engine: engine) {
            heldAtFinish = engine.isHoldingVerdicts
            throw FakeCoreTxoEngine.Failure()
        }

        XCTAssertEqual(heldAtFinish, false)
        XCTAssertFalse(engine.isHoldingVerdicts)
        XCTAssertEqual(engine.releases, 1)
    }

    // MARK: Through the manager

    /// Cancelling the caller of a run already in flight stops the walk at
    /// its next boundary; the run's claim is still released, and the
    /// caller still gets the run's report.
    func testACancelledCallerStillFinishesTheClaim() async throws {
        let (handler, container) = try makeHandler()
        let entered = expectation(description: "first engine page entered")
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let base = FakeCoreTxoEngine(inventory: [engineUtxo(0x16)])
        let engine = PausedPageEngine(base: base, entered: entered, gate: gate)
        let native = FakeReconcileNative(engine: engine, holding: { base.isHoldingVerdicts })
        let manager = try await makeManager(handle: 44, handler: handler, native: native)
        let walletId = self.walletId

        let run = Task { try await manager.reconcileCoreTxoStore(for: walletId) }
        await fulfillment(of: [entered], timeout: 5)
        run.cancel()
        gate.signal()
        let outcome = try await run.value

        guard case .reconciled(let report) = outcome else {
            return XCTFail("expected a run, got \(outcome)")
        }
        XCTAssertFalse(report.completed)
        XCTAssertEqual(report.mutations, 0, "the page read before the cancellation was not written")
        XCTAssertEqual(try txoCount(container), 0)
        XCTAssertEqual(native.begins, [walletId])
        XCTAssertEqual(native.finishes.count, 1, "a granted begin is finished despite the cancellation")
        XCTAssertEqual(native.finishes.first?.ran, true)
        _ = await manager.shutdown()
        PlatformWalletManager.destroyQueue.sync {}
    }

    /// The poll's due list launches one run per loaded wallet, never a
    /// second one behind a launch that has not returned, and nothing for a
    /// wallet this manager has not loaded.
    func testThePollLaunchesOneRunPerDueLoadedWallet() async throws {
        let (handler, _) = try makeHandler()
        let beginGate = DispatchSemaphore(value: 0)
        defer { beginGate.signal() }
        let begun = expectation(description: "begin entered")
        let native = FakeReconcileNative(
            engine: FakeCoreTxoEngine(), gate: .skip(code: UInt8(TXO_RECONCILE_SKIP_NOT_STEADY_STATE)))
        let entries = Counter()
        native.onBegin = {
            if entries.next() == 1 {
                begun.fulfill()
                XCTAssertEqual(beginGate.wait(timeout: .now() + 5), .success)
            }
        }
        let manager = try await makeManager(handle: 45, handler: handler, native: native)
        let walletId = self.walletId
        let baseline = PlatformWalletPollBaseline(
            spvProgress: .empty, spvIsRunning: false, spvPeers: [],
            platformAddressSyncIsSyncing: false, shieldedSyncIsSyncing: false,
            dashPaySyncIsSyncing: false, spvTipBlockTime: nil, pendingAccountBuilds: [:])
        var snapshot = PlatformWalletPollSnapshot()
        snapshot.txoReconcileDueWallets = [walletId, Data(repeating: 0x99, count: 32)]

        manager.applyManagerSnapshot(snapshot, baseline: baseline)
        await fulfillment(of: [begun], timeout: 5)
        // A second tick while the first launch is still in its begin.
        manager.applyManagerSnapshot(snapshot, baseline: baseline)
        XCTAssertEqual(manager.coreTxoReconcileLaunched, [walletId])
        beginGate.signal()
        try await waitUntil { manager.coreTxoReconcileLaunched.isEmpty }
        XCTAssertEqual(native.begins, [walletId], "one launch, for the loaded wallet only")
        XCTAssertTrue(native.finishes.isEmpty, "the gate refused: nothing to finish")

        // Once it returned, the next report launches again.
        manager.applyManagerSnapshot(snapshot, baseline: baseline)
        try await waitUntil { native.begins.count == 2 }
        XCTAssertEqual(native.begins, [walletId, walletId])
        _ = await manager.shutdown()
        PlatformWalletManager.destroyQueue.sync {}
    }

    func testTheManagerRefusesAReconcileOnceShutdownHasBegun() async throws {
        let manager = PlatformWalletManager.makeForTesting(handle: 42, calls: Self.okTeardownCalls())
        let epochBefore = manager.coreTxoReconcileEpoch.current()
        _ = await manager.shutdown()
        PlatformWalletManager.destroyQueue.sync {}
        XCTAssertGreaterThan(manager.coreTxoReconcileEpoch.current(), epochBefore, "shutdown stales in-flight runs")

        let outcome = try await manager.reconcileCoreTxoStore(for: walletId)
        XCTAssertEqual(outcome, .skipped(.notConfigured))
    }

    func testShouldCancelParkedReconcileBeforeBlockingShieldedStop() async throws {
        let (handler, container) = try makeHandler()
        let pageEntered = expectation(description: "Reconcile entered first page")
        let stopEntered = expectation(description: "Shielded stop entered")
        let finished = expectation(description: "Reconcile stopped while shielded stop remained blocked")
        let pageGate = DispatchSemaphore(value: 0)
        let stopGate = DispatchSemaphore(value: 0)
        defer { pageGate.signal(); stopGate.signal() }
        let base = FakeCoreTxoEngine(inventory: [engineUtxo(0x41), engineUtxo(0x42)])
        let engine = PausedPageEngine(base: base, entered: pageEntered, gate: pageGate)
        let ok: @Sendable (Handle) -> PlatformWalletFFIResult = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let manager = PlatformWalletManager.makeForTesting(
            handle: 43,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: ok, platformAddressSyncStop: ok,
                shieldedSyncStop: { handle in
                    stopEntered.fulfill()
                    XCTAssertEqual(stopGate.wait(timeout: .now() + 10), .success)
                    return ok(handle)
                },
                dashPaySyncStop: ok, dpnsSyncStop: ok, destroy: ok))
        let epoch = manager.coreTxoReconcileEpoch
        let generation = epoch.current()
        let walletId = self.walletId
        let box = ReportBox()
        manager.coreTxoReconcileQueue.async {
            box.set(PlatformWalletManager.runCoreTxoReconcile(
                walletId: walletId, pageSize: 1,
                engine: engine, handler: handler,
                isCancelled: { epoch.current() != generation }))
            finished.fulfill()
        }
        await fulfillment(of: [pageEntered], timeout: 5)
        let shutdown = Task { await manager.shutdown() }
        await fulfillment(of: [stopEntered], timeout: 5)
        pageGate.signal()
        await fulfillment(of: [finished], timeout: 5)

        let report = try XCTUnwrap(box.get())
        XCTAssertFalse(report.completed)
        XCTAssertEqual(base.pageCalls, 1, "No second engine read may start during early stop")
        XCTAssertEqual(report.mutations, 0, "The old page must not reach persistence after shutdown")
        XCTAssertEqual(try txoCount(container), 0)
        stopGate.signal()
        _ = await shutdown.value
    }

    // MARK: The C API adapter

    /// Against a handle the registry never issues, every call fails cleanly
    /// and nothing is held; the stale-page answer is Rust's limit.
    func testTheFFIAdapterHoldsNothingOnAFailedCall() throws {
        let engine = FFICoreTxoReconcileEngine(handle: NULL_HANDLE, walletId: walletId)
        XCTAssertThrowsError(try engine.healPage(after: nil, limit: 4))
        XCTAssertThrowsError(try engine.classifyHeld([CoreOutpointOwnershipQuery(
            account: bip44, txid: txid(0x17), vout: 0, scriptPubKey: fixtureScript)]))
        engine.releaseVerdicts()
        XCTAssertEqual(try engine.classifyHeld([]), [], "an empty page asks nothing")
        XCTAssertTrue(engine.retryStalePage(staleInARow: 1))
        XCTAssertEqual(
            engine.retryStalePage(staleInARow: 6),
            platform_wallet_txo_reconcile_retry_stale_page(6)
        )
        XCTAssertThrowsError(try CoreTxoReconcileFFI.begin(NULL_HANDLE, walletId: walletId))
        XCTAssertThrowsError(try CoreTxoReconcileFFI.finish(NULL_HANDLE, walletId: walletId, ran: false))
        XCTAssertThrowsError(try CoreTxoReconcileFFI.dueWallets(NULL_HANDLE))
    }

    // MARK: Helpers

    /// Whether the caller runs on the persistence handler's serial queue
    /// (also inside a `sync` onto it from another thread).
    private nonisolated static func onPersistenceQueue() -> Bool {
        String(cString: __dispatch_queue_get_label(nil)) == "org.dash.platform-wallet.persistence"
    }

    private func waitUntil(
        timeout: Duration = .seconds(5),
        _ condition: () -> Bool
    ) async throws {
        let deadline = ContinuousClock.now + timeout
        while !condition() {
            if ContinuousClock.now > deadline {
                XCTFail("condition not met within \(timeout)")
                return
            }
            try await Task.sleep(for: .milliseconds(10))
        }
    }

    private final class PausedPageEngine: CoreTxoReconcileEngine, @unchecked Sendable {
        let base: FakeCoreTxoEngine
        let entered: XCTestExpectation
        let gate: DispatchSemaphore

        init(base: FakeCoreTxoEngine, entered: XCTestExpectation, gate: DispatchSemaphore) {
            self.base = base
            self.entered = entered
            self.gate = gate
        }

        func healPage(after: CoreEngineUtxo?, limit: Int) throws -> (rows: [CoreTxoHealRow], hasMore: Bool) {
            if after == nil {
                entered.fulfill()
                XCTAssertEqual(gate.wait(timeout: .now() + 10), .success)
            }
            return try base.healPage(after: after, limit: limit)
        }

        func classifyHeld(_ queries: [CoreOutpointOwnershipQuery]) throws -> [CoreOutpointClass] {
            try base.classifyHeld(queries)
        }

        func releaseVerdicts() {
            base.releaseVerdicts()
        }

        func retryStalePage(staleInARow: Int) -> Bool {
            base.retryStalePage(staleInARow: staleInARow)
        }
    }

    private final class Counter: @unchecked Sendable {
        private let lock = NSLock()
        private var value = 0
        func next() -> Int { lock.withLock { value += 1; return value } }
    }

    private final class ReportBox: @unchecked Sendable {
        private let lock = NSLock()
        private var report: CoreTxoReconcileReport?
        func set(_ report: CoreTxoReconcileReport) { lock.withLock { self.report = report } }
        func get() -> CoreTxoReconcileReport? { lock.withLock { report } }
    }
}
