import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Shutdown and race coverage for the store reconcile: it stops between
/// pages when cancelled, it defers a step while a Rust persistence round is
/// open and completes once the round closes, and the manager refuses it
/// once shutdown has begun.
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

    func testACancelledRunDoesNothingAndSaysSo() throws {
        let (handler, container) = try makeHandler()
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(0x11)])

        let report = PlatformWalletManager.runCoreTxoReconcile(
            walletId: walletId, tipHeight: 2_535_898, engine: engine, handler: handler,
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
            walletId: walletId, tipHeight: 2_535_898, pageSize: 2, engine: engine, handler: handler,
            // The run checks once before each engine read and once before
            // each store step: the first two admit page one and its heal,
            // the third (before page two) reports the epoch bumped.
            isCancelled: { checks.next() >= 3 }
        )

        XCTAssertFalse(report.completed)
        XCTAssertEqual(engine.pageCalls, 1)
        XCTAssertEqual(report.inserted, 2, "the page that landed stays")
        XCTAssertEqual(try txoCount(container), 2)
    }

    /// A Rust round open on the persistence queue defers every store step:
    /// the run retries until the round closes, then completes normally.
    func testAStepDeferredBehindAnOpenRoundCompletesOnceTheRoundCloses() throws {
        let (handler, container) = try makeHandler()
        let engine = FakeCoreTxoEngine(inventory: [engineUtxo(0x31)])

        handler.beginChangeset(walletId: walletId)
        let finished = expectation(description: "reconcile finished")
        let box = ReportBox()
        let walletId = self.walletId
        DispatchQueue.global(qos: .utility).async {
            let report = PlatformWalletManager.runCoreTxoReconcile(
                walletId: walletId, tipHeight: 2_535_898, engine: engine, handler: handler,
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

    func testTheManagerRefusesAReconcileOnceShutdownHasBegun() async throws {
        let ok: @Sendable (Handle) -> PlatformWalletFFIResult = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        let manager = PlatformWalletManager.makeForTesting(
            handle: 42,
            calls: PlatformWalletNativeTeardownCalls(
                spvStop: ok, platformAddressSyncStop: ok, shieldedSyncStop: ok,
                dashPaySyncStop: ok, dpnsSyncStop: ok, destroy: ok
            )
        )
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
                walletId: walletId, tipHeight: 2_535_898, pageSize: 1,
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

    private final class PausedPageEngine: CoreTxoEngineInventory, @unchecked Sendable {
        let base: FakeCoreTxoEngine
        let entered: XCTestExpectation
        let gate: DispatchSemaphore

        init(base: FakeCoreTxoEngine, entered: XCTestExpectation, gate: DispatchSemaphore) {
            self.base = base
            self.entered = entered
            self.gate = gate
        }

        func utxoPage(after: CoreEngineUtxo?, limit: Int) throws -> (rows: [CoreEngineUtxo], hasMore: Bool) {
            if after == nil {
                entered.fulfill()
                XCTAssertEqual(gate.wait(timeout: .now() + 10), .success)
            }
            return try base.utxoPage(after: after, limit: limit)
        }

        func classify(_ queries: [CoreOutpointOwnershipQuery]) throws -> [CoreOutpointClass] {
            try base.classify(queries)
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
