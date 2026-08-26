import XCTest
@testable import SwiftDashSDK

/// Unit coverage for the async `loadFromPersistor()` overload.
///
/// Tests inject the native-load call table (bulk restore, id list, and
/// per-wallet lookup) plus the teardown table for the shutdown-interplay
/// cases, so the production `performLoadFromPersistor` orchestration stays
/// under test: off-main execution, bulk error mapping, per-wallet
/// skip-and-continue parity with the sync overload, and participation in
/// the shutdown drain (an admitted load completes before teardown takes
/// the handle; new loads are rejected while the drain runs), and the
/// synchronous-overload admission gate — all without invoking a production
/// load or teardown call. The keychain unlock epilogue is exercised only
/// as far as its watch-only fast path (test wallets have no stored
/// mnemonic, so the verify FFI is never reached).
@MainActor
final class PlatformWalletLoadFromPersistorTests: XCTestCase {
    /// Thread-recording, optionally gated native-load table.
    private final class LoadRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let gate: DispatchSemaphore?
        private let bulkFailingCode: PlatformWalletFFIResultCode?
        private let walletIds: [Data]
        private let failingLookups: [Data: PlatformWalletFFIResultCode]
        private var bulkInvocations: [(handle: Handle, ranOnMainThread: Bool)] = []
        private var lookupIds: [Data] = []

        init(
            walletIds: [Data] = [],
            gate: DispatchSemaphore? = nil,
            bulkFailingCode: PlatformWalletFFIResultCode? = nil,
            failingLookups: [Data: PlatformWalletFFIResultCode] = [:]
        ) {
            self.walletIds = walletIds
            self.gate = gate
            self.bulkFailingCode = bulkFailingCode
            self.failingLookups = failingLookups
        }

        func makeCalls() -> PlatformWalletNativeLoadCalls {
            PlatformWalletNativeLoadCalls(
                loadFromPersistor: { [self] handle in
                    lock.withLock {
                        bulkInvocations.append((handle, Thread.isMainThread))
                    }
                    gate?.wait()
                    if let bulkFailingCode {
                        return PlatformWalletFFIResult(code: bulkFailingCode, message: nil)
                    }
                    return PlatformWalletFFIResult(
                        code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
                },
                restorableWalletIds: { [self] _ in walletIds },
                getWallet: { [self] _, walletId in
                    lock.withLock { lookupIds.append(walletId) }
                    if let failingCode = failingLookups[walletId] {
                        return (
                            PlatformWalletFFIResult(code: failingCode, message: nil),
                            NULL_HANDLE
                        )
                    }
                    // `ManagedPlatformWallet.deinit` destroys its handle
                    // through the live FFI. Keep the fake handle at zero so
                    // it can never collide with a real process-global Rust
                    // registry entry; the distinct wallet ids are sufficient
                    // to tell results apart.
                    return (
                        PlatformWalletFFIResult(
                            code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil),
                        NULL_HANDLE
                    )
                }
            )
        }

        var bulkCount: Int { lock.withLock { bulkInvocations.count } }
        var bulkMainThreadFlags: [Bool] { lock.withLock { bulkInvocations.map(\.ranOnMainThread) } }
        var requestedLookupIds: [Data] { lock.withLock { lookupIds } }
    }

    private final class EventLog: @unchecked Sendable {
        private let lock = NSLock()
        private var entries: [String] = []
        func append(_ event: String) { lock.withLock { entries.append(event) } }
        var events: [String] { lock.withLock { entries } }
    }

    private nonisolated static func makeTeardownCalls(log: EventLog) -> PlatformWalletNativeTeardownCalls {
        func step(_ name: String) -> PlatformWalletNativeTeardownCalls.Call {
            { _ in
                log.append("teardown:\(name)")
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            }
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: step("spv_stop"),
            platformAddressSyncStop: step("platform_address_sync_stop"),
            shieldedSyncStop: step("shielded_sync_stop"),
            dashPaySyncStop: step("dashpay_sync_stop"),
            dpnsSyncStop: step("dpns_sync_stop"),
            destroy: step("destroy")
        )
    }

    private func makeManager(
        handle: Handle,
        recorder: LoadRecorder,
        teardownLog: EventLog = EventLog()
    ) -> PlatformWalletManager {
        let manager = PlatformWalletManager.makeForTesting(
            handle: handle,
            calls: Self.makeTeardownCalls(log: teardownLog)
        )
        manager.nativeLoadCalls = recorder.makeCalls()
        return manager
    }

    private func id(_ byte: UInt8) -> Data { Data(repeating: byte, count: 32) }

    func testLoadRunsOffMainPublishesWalletsAndSkipsFailedLookups() async throws {
        let good = id(1)
        let bad = id(2)
        let recorder = LoadRecorder(
            walletIds: [good, bad],
            failingLookups: [bad: PLATFORM_WALLET_FFI_RESULT_CODE_NOT_FOUND])
        let manager = makeManager(handle: 42, recorder: recorder)

        let restored = try await manager.loadFromPersistor()

        XCTAssertEqual(recorder.bulkCount, 1)
        XCTAssertEqual(
            recorder.bulkMainThreadFlags, [false],
            "the native load must run off the main thread")
        XCTAssertEqual(recorder.requestedLookupIds, [good, bad])
        XCTAssertEqual(restored.map(\.walletId), [good])
        XCTAssertTrue(manager.wallets[good] === restored.first)
        XCTAssertNil(manager.wallets[bad], "a failed lookup must be skipped, not published")
        XCTAssertNotNil(manager.lastError, "the skip must surface through lastError (sync parity)")
    }

    /// `lastError` parity with the sync overload: failures are replayed in
    /// wallet-id order, so the LATEST failure in sequence wins — not
    /// whichever phase (lookup vs unlock) happened to run last.
    func testLastErrorReflectsTheLatestFailureInSequenceOrder() async throws {
        let earlyBad = id(1)
        let good = id(2)
        let lateBad = id(3)
        let recorder = LoadRecorder(
            walletIds: [earlyBad, good, lateBad],
            failingLookups: [
                earlyBad: PLATFORM_WALLET_FFI_RESULT_CODE_NOT_FOUND,
                lateBad: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION,
            ])
        let manager = makeManager(handle: 42, recorder: recorder)

        let restored = try await manager.loadFromPersistor()

        XCTAssertEqual(restored.map(\.walletId), [good])
        guard case .some(.walletOperation) = manager.lastError as? PlatformWalletError else {
            return XCTFail(
                "lastError must hold the LATER failure in id order, got \(String(describing: manager.lastError))")
        }
    }

    func testShortWalletIdsNeverReachTheLookup() async throws {
        let recorder = LoadRecorder(walletIds: [Data([0x01, 0x02])])
        let manager = makeManager(handle: 42, recorder: recorder)

        let restored = try await manager.loadFromPersistor()

        XCTAssertTrue(restored.isEmpty)
        XCTAssertEqual(recorder.requestedLookupIds, [])
    }

    func testBulkErrorMapsToTypedErrorAndPublishesNothing() async {
        let recorder = LoadRecorder(
            walletIds: [id(1)],
            bulkFailingCode: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION)
        let manager = makeManager(handle: 7, recorder: recorder)

        do {
            _ = try await manager.loadFromPersistor()
            XCTFail("expected the bulk failure to throw")
        } catch let error as PlatformWalletError {
            guard case .walletOperation = error else {
                return XCTFail("expected walletOperation, got \(error)")
            }
        } catch {
            XCTFail("unexpected error: \(error)")
        }
        XCTAssertTrue(manager.wallets.isEmpty)
        XCTAssertEqual(
            recorder.requestedLookupIds, [],
            "a failed bulk load must not attempt per-wallet lookups")
    }

    /// A shutdown during the off-main load window must wait for the load's
    /// full transaction, exactly like the async create.
    func testShutdownDuringLoadWaitsForLoadThenTearsDown() async throws {
        let gate = DispatchSemaphore(value: 0)
        let recorder = LoadRecorder(walletIds: [id(3)], gate: gate)
        let log = EventLog()
        let manager = makeManager(handle: 11, recorder: recorder, teardownLog: log)

        let loadTask = Task { try await manager.loadFromPersistor() }
        while recorder.bulkCount == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }

        let shutdownTask = Task { await manager.shutdown() }
        try await Task.sleep(for: .milliseconds(30))
        XCTAssertEqual(
            manager.handle, 11,
            "shutdown must not take the handle while an admitted load is in flight")

        gate.signal()
        let restored = try await loadTask.value
        let metrics = await shutdownTask.value

        XCTAssertEqual(restored.map(\.walletId), [id(3)])
        XCTAssertTrue(manager.wallets[id(3)] === restored.first)
        XCTAssertEqual(manager.handle, NULL_HANDLE)
        XCTAssertEqual(metrics.steps.count, 6)
        XCTAssertEqual(log.events.first, "teardown:spv_stop")
    }

    func testLoadDuringShutdownDrainIsRejectedBeforeNativeCall() async throws {
        let gate = DispatchSemaphore(value: 0)
        let recorder = LoadRecorder(walletIds: [id(4)], gate: gate)
        let manager = makeManager(handle: 13, recorder: recorder)

        let firstLoad = Task { try await manager.loadFromPersistor() }
        while recorder.bulkCount == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }
        let shutdownTask = Task { await manager.shutdown() }
        try await Task.sleep(for: .milliseconds(20))

        do {
            _ = try await manager.loadFromPersistor()
            XCTFail("expected rejection during the shutdown drain")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        }

        gate.signal()
        _ = try await firstLoad.value
        _ = await shutdownTask.value
        XCTAssertEqual(recorder.bulkCount, 1, "the rejected load must never reach the native call")
    }

    /// The SYNCHRONOUS overload must be rejected while an async load is in
    /// flight: it would run a second Rust loader on the MainActor
    /// concurrently with the admitted one on the destroy queue, racing
    /// load.rs's two-step hydration. Rejected before any native work.
    func testSyncLoadIsRejectedWhileAsyncLoadIsInFlight() async throws {
        let gate = DispatchSemaphore(value: 0)
        let recorder = LoadRecorder(walletIds: [id(5)], gate: gate)
        // A deliberately high fake manager handle: if the admission gate
        // ever regressed, the sync overload's LIVE bulk FFI would run
        // against it — a value this large can never alias a real
        // registry entry allocated during this test process.
        let manager = makeManager(handle: 0x7FFF_FFF1, recorder: recorder)

        let asyncLoad = Task { try await manager.loadFromPersistor() }
        while recorder.bulkCount == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }

        // Explicitly non-async closure type forces the SYNC overload —
        // this async test body would otherwise resolve to the async one
        // (SE-0296).
        let syncLoad: () throws -> [ManagedPlatformWallet] = { try manager.loadFromPersistor() }
        do {
            _ = try syncLoad()
            XCTFail("expected the sync overload to be rejected during an async load")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        }

        gate.signal()
        _ = try await asyncLoad.value
        XCTAssertEqual(
            recorder.bulkCount, 1,
            "only the admitted async load may reach the native call")
    }

    /// The SYNCHRONOUS overload must also be rejected during the shutdown
    /// drain window, where the handle is intentionally still live for the
    /// admitted async op while the MainActor is reentrant at the drain's
    /// await.
    func testSyncLoadIsRejectedDuringShutdownDrain() async throws {
        let gate = DispatchSemaphore(value: 0)
        let recorder = LoadRecorder(walletIds: [id(6)], gate: gate)
        let manager = makeManager(handle: 0x7FFF_FFF2, recorder: recorder)

        let asyncLoad = Task { try await manager.loadFromPersistor() }
        while recorder.bulkCount == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }
        let shutdownTask = Task { await manager.shutdown() }
        try await Task.sleep(for: .milliseconds(20))
        XCTAssertEqual(manager.handle, 0x7FFF_FFF2, "drain must hold the handle live")

        let syncLoad: () throws -> [ManagedPlatformWallet] = { try manager.loadFromPersistor() }
        do {
            _ = try syncLoad()
            XCTFail("expected the sync overload to be rejected during the drain")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        }

        gate.signal()
        _ = try await asyncLoad.value
        _ = await shutdownTask.value
        XCTAssertEqual(recorder.bulkCount, 1)
    }

    /// `deleteWallet` is a synchronous native op too (native removal +
    /// Keychain + SwiftData wipe): interleaved with an in-flight async
    /// restore it could remove the native registration between the
    /// restore's snapshot read and its publication, or wipe data the
    /// loader then re-inserts. The shared admission gate rejects it before
    /// ANY of its work runs.
    func testDeleteWalletIsRejectedWhileAsyncLoadIsInFlight() async throws {
        let gate = DispatchSemaphore(value: 0)
        let recorder = LoadRecorder(walletIds: [id(7)], gate: gate)
        let manager = makeManager(handle: 0x7FFF_FFF3, recorder: recorder)

        let asyncLoad = Task { try await manager.loadFromPersistor() }
        while recorder.bulkCount == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }

        do {
            try manager.deleteWallet(walletId: id(7))
            XCTFail("expected deleteWallet to be rejected during an async load")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        }

        gate.signal()
        let restored = try await asyncLoad.value
        XCTAssertEqual(
            restored.map(\.walletId), [id(7)],
            "the admitted restore must complete untouched by the rejected deletion")
    }
}
