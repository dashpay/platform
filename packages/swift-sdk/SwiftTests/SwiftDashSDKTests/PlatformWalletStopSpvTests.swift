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
    /// The gate wait gives up after 10 s, so a stop that ran on the main
    /// thread (blocking the test that would open the gate) fails the test
    /// instead of hanging it.
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
            if isFirst { _ = gate?.wait(timeout: .now() + 10) }
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

    private struct TimedOut: Error, CustomStringConvertible {
        let what: String
        let timeout: Duration
        var description: String { "\(what) did not happen within \(timeout)" }
    }

    /// Polls `condition` on the main actor, and fails the test instead of
    /// hanging when it does not hold within `timeout`.
    private func waitUntil(
        _ what: String,
        timeout: Duration = .seconds(5),
        _ condition: () -> Bool
    ) async throws {
        let deadline = ContinuousClock.now + timeout
        while !condition() {
            guard ContinuousClock.now < deadline else { throw TimedOut(what: what, timeout: timeout) }
            try await Task.sleep(for: .milliseconds(5))
        }
    }

    private func waitUntilStopStarted(_ recorder: StopRecorder) async throws {
        try await waitUntil("the native stop starting") { recorder.count > 0 }
    }

    /// Asserts `call` throws the in-flight-stop refusal, not some other error.
    private func assertRefusedWhileStopping(
        _ call: @autoclosure () throws -> Void,
        _ name: String,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertThrowsError(try call(), name, file: file, line: line) { error in
            guard case PlatformWalletError.walletOperation(let message) = error,
                  message.contains("SPV stop in progress") else {
                return XCTFail("\(name): expected the in-flight stop refusal, got \(error)", file: file, line: line)
            }
        }
    }

    // MARK: - Off-main execution

    /// The native stop runs off the main thread, and the main actor keeps
    /// running work while that stop is blocked: this test runs on the main
    /// actor and polls for the stop to start while its gate is still closed.
    func testStopRunsOffMainWhileTheMainActorStaysFree() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 21, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntilStopStarted(recorder)

        XCTAssertFalse(
            log.events.contains("spv_stop:end"),
            "the main actor must run while the native stop is still blocked")

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
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 22, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntilStopStarted(recorder)

        assertRefusedWhileStopping(
            try manager.startSpv(config: PlatformSpvStartConfig(dataDir: "/tmp/unused", network: .testnet)),
            "startSpv")

        gate.signal()
        try await stopTask.value
        XCTAssertEqual(manager.spvStopsInFlight, 0, "the in-flight count must be released after the stop")
        await manager.shutdown()
    }

    /// The blocking stop and the storage clear are refused the same way: the
    /// blocking stop would wait on the same teardown on the calling thread,
    /// and the stopping client can still hold the data directory.
    func testBlockingStopAndStorageClearAreRefusedWhileAnAsyncStopIsInFlight() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 25, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntilStopStarted(recorder)

        // A synchronous context resolves `stopSpv()` to the blocking overload.
        func blockingStop() throws { try manager.stopSpv() }
        assertRefusedWhileStopping(try blockingStop(), "blocking stopSpv()")
        assertRefusedWhileStopping(try manager.clearSpvStorage(), "clearSpvStorage()")

        gate.signal()
        try await stopTask.value
        await manager.shutdown()
    }

    // MARK: - Shutdown interplay

    /// A shutdown during an in-flight stop waits for it: every teardown step
    /// except the early shielded stop runs after the admitted SPV stop ends.
    func testShutdownWaitsForAnInFlightStopBeforeTearingDown() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 23, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntilStopStarted(recorder)

        let shutdownTask = Task { await manager.shutdown() }
        // The early shielded stop is the step right before shutdown waits for
        // admitted native operations; destroy can only follow that wait.
        try await waitUntil("the early shielded stop") {
            log.events.contains("teardown:shielded_sync_stop")
        }
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
