import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the async off-main `stopSpv()` overload.
///
/// Tests inject the native SPV stop through the teardown table, so the
/// production orchestration stays under test: the native stop runs off the
/// main thread, a start is refused while it runs, `shutdown()` waits for an
/// in-flight stop before tearing the manager down, and a stop after shutdown
/// is rejected before any native work.
@MainActor
final class PlatformWalletStopSpvTests: XCTestCase {

    private final class EventLog: @unchecked Sendable {
        private let lock = NSLock()
        private var entries: [String] = []
        func append(_ event: String) { lock.withLock { entries.append(event) } }
        var events: [String] { lock.withLock { entries } }
    }

    /// Fake native SPV stop: records which thread ran it and, for the first
    /// call only, blocks on `gate` so a test can act while the stop is in
    /// flight. Later calls (the teardown's own SPV stop step) return at once.
    private final class StopRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let gate: DispatchSemaphore?
        private let eventLog: EventLog
        private var mainThreadFlags: [Bool] = []

        init(gate: DispatchSemaphore?, eventLog: EventLog) {
            self.gate = gate
            self.eventLog = eventLog
        }

        func stop() -> PlatformWalletFFIResult {
            let isFirst = lock.withLock { () -> Bool in
                mainThreadFlags.append(Thread.isMainThread)
                return mainThreadFlags.count == 1
            }
            eventLog.append("spv_stop:begin")
            if isFirst { gate?.wait() }
            eventLog.append("spv_stop:end")
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }

        var count: Int { lock.withLock { mainThreadFlags.count } }
        var ranOnMainThread: [Bool] { lock.withLock { mainThreadFlags } }
    }

    private nonisolated static func makeTeardownCalls(
        recorder: StopRecorder,
        log: EventLog
    ) -> PlatformWalletNativeTeardownCalls {
        func step(_ name: String) -> PlatformWalletNativeTeardownCalls.Call {
            { _ in
                log.append("teardown:\(name)")
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            }
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: { _ in recorder.stop() },
            platformAddressSyncStop: step("platform_address_sync_stop"),
            shieldedSyncStop: step("shielded_sync_stop"),
            dashPaySyncStop: step("dashpay_sync_stop"),
            dpnsSyncStop: step("dpns_sync_stop"),
            destroy: step("destroy")
        )
    }

    private func makeManager(handle: Handle, recorder: StopRecorder, log: EventLog) -> PlatformWalletManager {
        PlatformWalletManager.makeForTesting(
            handle: handle,
            calls: Self.makeTeardownCalls(recorder: recorder, log: log))
    }

    private func waitUntilStopStarted(_ recorder: StopRecorder) async throws {
        while recorder.count == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }
    }

    // MARK: - Off-main execution

    /// The native stop runs off the main thread, and the main actor keeps
    /// running work while that stop is blocked — this test's own polling
    /// loop runs on the main actor for the whole blocked window.
    func testStopRunsOffMainWhileTheMainActorStaysFree() async throws {
        let gate = DispatchSemaphore(value: 0)
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 21, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntilStopStarted(recorder)

        var mainActorTurns = 0
        for _ in 0..<3 {
            try await Task.sleep(for: .milliseconds(5))
            mainActorTurns += 1
        }
        XCTAssertEqual(mainActorTurns, 3, "the main actor must stay free while the native stop blocks")
        XCTAssertFalse(log.events.contains("spv_stop:end"), "the native stop must still be blocked")

        gate.signal()
        try await stopTask.value

        XCTAssertEqual(recorder.ranOnMainThread, [false], "the native stop must run off the main thread")
        await manager.shutdown()
    }

    // MARK: - Start while stopping

    /// With the main actor free during the stop, a start issued meanwhile
    /// must be refused rather than race the teardown of the old client.
    func testStartSpvIsRefusedWhileAnAsyncStopIsInFlight() async throws {
        let gate = DispatchSemaphore(value: 0)
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 22, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntilStopStarted(recorder)

        XCTAssertThrowsError(
            try manager.startSpv(config: PlatformSpvStartConfig(dataDir: "/tmp/unused", network: .testnet))
        ) { error in
            guard case PlatformWalletError.walletOperation = error else {
                return XCTFail("expected walletOperation, got \(error)")
            }
        }

        gate.signal()
        try await stopTask.value
        XCTAssertEqual(manager.spvStopsInFlight, 0, "the in-flight count must be released after the stop")
        await manager.shutdown()
    }

    // MARK: - Shutdown interplay

    /// A shutdown during an in-flight stop waits for it: every teardown step
    /// except the early shielded stop runs after the admitted SPV stop ends.
    func testShutdownWaitsForAnInFlightStopBeforeTearingDown() async throws {
        let gate = DispatchSemaphore(value: 0)
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 23, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntilStopStarted(recorder)

        let shutdownTask = Task { await manager.shutdown() }
        try await Task.sleep(for: .milliseconds(30))
        XCTAssertFalse(
            log.events.contains("teardown:destroy"),
            "shutdown must not destroy the handle while an admitted stop runs")

        gate.signal()
        try await stopTask.value
        let metrics = await shutdownTask.value

        let afterShieldedStop = log.events.filter { $0 != "teardown:shielded_sync_stop" }
        XCTAssertEqual(
            afterShieldedStop.prefix(2), ["spv_stop:begin", "spv_stop:end"],
            "unexpected event order: \(log.events)")
        XCTAssertEqual(afterShieldedStop.last, "teardown:destroy")
        XCTAssertEqual(metrics.steps.count, 6)
    }

    /// A stop requested after shutdown is rejected up front, before any
    /// native work.
    func testStopAfterShutdownThrowsWithoutInvokingNativeCall() async {
        let log = EventLog()
        let recorder = StopRecorder(gate: nil, eventLog: log)
        let manager = makeManager(handle: 24, recorder: recorder, log: log)

        await manager.shutdown()
        let stopsDuringTeardown = recorder.count

        do {
            try await manager.stopSpv()
            XCTFail("expected a throw after shutdown")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        } catch {
            XCTFail("unexpected error: \(error)")
        }
        XCTAssertEqual(recorder.count, stopsDuringTeardown, "the native stop must never be reached after shutdown")
    }
}
