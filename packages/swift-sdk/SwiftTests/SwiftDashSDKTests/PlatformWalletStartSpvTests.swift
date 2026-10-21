import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the async off-main `startSpv(config:)`.
///
/// Tests inject the native SPV start (and stop) through the teardown table,
/// so the production orchestration stays under test: the native start runs
/// off the main thread while the main actor keeps running, the config reaches
/// the native call, the SPV calls that would race or park behind an in-flight
/// start are refused, an async stop issued meanwhile runs after it,
/// `shutdown()` waits for an in-flight start, and a start after shutdown is
/// rejected before any native work.
@MainActor
final class PlatformWalletStartSpvTests: XCTestCase {

    private final class EventLog: @unchecked Sendable {
        private let lock = NSLock()
        private var entries: [String] = []
        func append(_ event: String) { lock.withLock { entries.append(event) } }
        var events: [String] { lock.withLock { entries } }
    }

    /// Fake native SPV start: records which thread ran it and the arguments
    /// it received, and, for the first call only, blocks on `gate` so a test
    /// can act while the start is in flight. The gate wait gives up after
    /// 10 s, so a start that ran on the main thread (blocking the test that
    /// would open the gate) fails the test instead of hanging it.
    private final class StartRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let gate: DispatchSemaphore?
        private let eventLog: EventLog
        private let code: PlatformWalletFFIResultCode
        private var mainThreadFlags: [Bool] = []
        private var handles: [Handle] = []
        private var configs: [PlatformSpvStartConfig] = []

        init(
            gate: DispatchSemaphore?,
            eventLog: EventLog,
            code: PlatformWalletFFIResultCode = PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS
        ) {
            self.gate = gate
            self.eventLog = eventLog
            self.code = code
        }

        func start(_ handle: Handle, _ config: PlatformSpvStartConfig) -> PlatformWalletFFIResult {
            let isFirst = lock.withLock { () -> Bool in
                mainThreadFlags.append(Thread.isMainThread)
                handles.append(handle)
                configs.append(config)
                return mainThreadFlags.count == 1
            }
            eventLog.append("spv_start:begin")
            if isFirst { _ = gate?.wait(timeout: .now() + 10) }
            eventLog.append("spv_start:end")
            // A fake message would be handed to the Rust-side free by
            // `PlatformWalletResult`'s deinit, so fakes never carry one.
            return PlatformWalletFFIResult(code: code, message: nil)
        }

        var count: Int { lock.withLock { mainThreadFlags.count } }
        var ranOnMainThread: [Bool] { lock.withLock { mainThreadFlags } }
        var receivedHandles: [Handle] { lock.withLock { handles } }
        var receivedConfigs: [PlatformSpvStartConfig] { lock.withLock { configs } }
    }

    /// Fake native SPV stop with the same gating; later calls (the teardown's
    /// own SPV stop step) return at once.
    private final class StopRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let gate: DispatchSemaphore?
        private let eventLog: EventLog
        private var calls = 0

        init(gate: DispatchSemaphore?, eventLog: EventLog) {
            self.gate = gate
            self.eventLog = eventLog
        }

        func stop() -> PlatformWalletFFIResult {
            let isFirst = lock.withLock { () -> Bool in
                calls += 1
                return calls == 1
            }
            eventLog.append("spv_stop:begin")
            if isFirst { _ = gate?.wait(timeout: .now() + 10) }
            eventLog.append("spv_stop:end")
            return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }

        var count: Int { lock.withLock { calls } }
    }

    private nonisolated static func makeTeardownCalls(
        start: StartRecorder,
        stop: StopRecorder,
        log: EventLog
    ) -> PlatformWalletNativeTeardownCalls {
        func step(_ name: String) -> PlatformWalletNativeTeardownCalls.Call {
            { _ in
                log.append("teardown:\(name)")
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            }
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: { _ in stop.stop() },
            platformAddressSyncStop: step("platform_address_sync_stop"),
            shieldedSyncStop: step("shielded_sync_stop"),
            dashPaySyncStop: step("dashpay_sync_stop"),
            dpnsSyncStop: step("dpns_sync_stop"),
            destroy: step("destroy"),
            spvStart: { handle, config in start.start(handle, config) }
        )
    }

    private func makeManager(
        handle: Handle,
        start: StartRecorder,
        stop: StopRecorder,
        log: EventLog
    ) -> PlatformWalletManager {
        PlatformWalletManager.makeForTesting(
            handle: handle,
            calls: Self.makeTeardownCalls(start: start, stop: stop, log: log))
    }

    private static let config = PlatformSpvStartConfig(
        dataDir: "/tmp/unused",
        network: .testnet,
        userAgent: "/test-agent/",
        peers: ["127.0.0.1:19999", "127.0.0.2:19999"],
        restrictToConfiguredPeers: true,
        startFromHeight: 1_000
    )

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

    private func waitUntilStartBegan(_ recorder: StartRecorder) async throws {
        try await waitUntil("the native start beginning") { recorder.count > 0 }
    }

    private func waitUntilStopBegan(_ recorder: StopRecorder) async throws {
        try await waitUntil("the native stop beginning") { recorder.count > 0 }
    }

    /// Asserts `call` throws the in-flight refusal for `inFlight` ("start" or
    /// "stop"), not some other error.
    private func assertRefused(
        while inFlight: String,
        _ name: String,
        file: StaticString = #filePath,
        line: UInt = #line,
        _ call: () async throws -> Void
    ) async {
        do {
            try await call()
            XCTFail("\(name): expected the in-flight \(inFlight) refusal, got no error", file: file, line: line)
        } catch {
            guard case PlatformWalletError.walletOperation(let message) = error,
                  message.contains("SPV \(inFlight) in progress") else {
                return XCTFail(
                    "\(name): expected the in-flight \(inFlight) refusal, got \(error)",
                    file: file, line: line)
            }
        }
    }

    // MARK: - Off-main execution

    /// The native start runs off the main thread, and the main actor keeps
    /// running work while that start is blocked: this test runs on the main
    /// actor and polls for the start to begin while its gate is still closed.
    /// A store open that runs a multi-second storage migration must never
    /// freeze the UI the way the former blocking overload did.
    func testStartRunsOffMainWhileTheMainActorStaysFree() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let start = StartRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 31, start: start, stop: StopRecorder(gate: nil, eventLog: log), log: log)

        let startTask = Task { try await manager.startSpv(config: Self.config) }
        try await waitUntilStartBegan(start)

        XCTAssertFalse(
            log.events.contains("spv_start:end"),
            "the main actor must run while the native start is still blocked")
        XCTAssertEqual(manager.spvStartsInFlight, 1)

        gate.signal()
        try await startTask.value

        XCTAssertEqual(start.ranOnMainThread, [false], "the native start must run off the main thread")
        XCTAssertEqual(manager.spvStartsInFlight, 0, "the in-flight count must be released after the start")
        await manager.shutdown()
    }

    /// The handle and the whole config reach the native call unchanged.
    func testStartHandsTheHandleAndConfigToTheNativeCall() async throws {
        let log = EventLog()
        let start = StartRecorder(gate: nil, eventLog: log)
        let manager = makeManager(handle: 32, start: start, stop: StopRecorder(gate: nil, eventLog: log), log: log)

        try await manager.startSpv(config: Self.config)

        XCTAssertEqual(start.receivedHandles, [32])
        let received = try XCTUnwrap(start.receivedConfigs.first)
        XCTAssertEqual(received.dataDir, Self.config.dataDir)
        XCTAssertEqual(received.network, Self.config.network)
        XCTAssertEqual(received.userAgent, Self.config.userAgent)
        XCTAssertEqual(received.peers, Self.config.peers)
        XCTAssertEqual(received.restrictToConfiguredPeers, Self.config.restrictToConfiguredPeers)
        XCTAssertEqual(received.startFromHeight, Self.config.startFromHeight)
        XCTAssertNil(received.devnetName)
        await manager.shutdown()
    }

    // MARK: - Refusals

    /// With the main actor free during an async stop, a start issued
    /// meanwhile must be refused rather than race the teardown of the old
    /// client.
    func testStartIsRefusedWhileAnAsyncStopIsInFlight() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let start = StartRecorder(gate: nil, eventLog: log)
        let stop = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 33, start: start, stop: stop, log: log)

        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntilStopBegan(stop)

        await assertRefused(while: "stop", "startSpv") {
            try await manager.startSpv(config: Self.config)
        }
        XCTAssertEqual(start.count, 0, "the native start must not be reached while a stop is in flight")

        gate.signal()
        try await stopTask.value
        await manager.shutdown()
    }

    /// With the main actor free during an async start, a second start, the
    /// blocking stop and the storage clear are refused: they would park the
    /// calling thread behind the in-flight start on the FFI's registry guard,
    /// or clear the directory the starting client is opening.
    func testSecondStartAndBlockingSpvCallsAreRefusedWhileAnAsyncStartIsInFlight() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let start = StartRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 34, start: start, stop: StopRecorder(gate: nil, eventLog: log), log: log)

        let startTask = Task { try await manager.startSpv(config: Self.config) }
        try await waitUntilStartBegan(start)

        await assertRefused(while: "start", "second startSpv") {
            try await manager.startSpv(config: Self.config)
        }
        // A synchronous context resolves `stopSpv()` to the blocking overload.
        func blockingStop() throws { try manager.stopSpv() }
        await assertRefused(while: "start", "blocking stopSpv()") { try blockingStop() }
        await assertRefused(while: "start", "clearSpvStorage()") { try manager.clearSpvStorage() }
        XCTAssertEqual(start.count, 1, "the refused second start must not reach the native call")

        gate.signal()
        try await startTask.value
        XCTAssertEqual(manager.spvStartsInFlight, 0, "the in-flight count must be released after the start")
        await manager.shutdown()
    }

    /// An async stop issued during an async start is not refused: it queues
    /// behind the start on the shared lifecycle queue, so the client the
    /// start brings up is the one it stops, and the FFI never sees the two
    /// at once.
    func testAsyncStopIssuedDuringAnAsyncStartRunsAfterIt() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let start = StartRecorder(gate: gate, eventLog: log)
        let stop = StopRecorder(gate: nil, eventLog: log)
        let manager = makeManager(handle: 35, start: start, stop: stop, log: log)

        let startTask = Task { try await manager.startSpv(config: Self.config) }
        try await waitUntilStartBegan(start)
        let stopTask = Task { try await manager.stopSpv() }
        try await waitUntil("the stop being admitted") { manager.spvStopsInFlight == 1 }
        XCTAssertEqual(stop.count, 0, "the native stop must wait for the in-flight start")

        gate.signal()
        try await startTask.value
        try await stopTask.value

        XCTAssertEqual(
            log.events, ["spv_start:begin", "spv_start:end", "spv_stop:begin", "spv_stop:end"],
            "unexpected event order: \(log.events)")
        await manager.shutdown()
    }

    // MARK: - Shutdown interplay

    /// A shutdown during an in-flight start waits for it: every teardown step
    /// except the early shielded stop runs after the admitted SPV start ends.
    func testShutdownWaitsForAnInFlightStartBeforeTearingDown() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let start = StartRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 36, start: start, stop: StopRecorder(gate: nil, eventLog: log), log: log)

        let startTask = Task { try await manager.startSpv(config: Self.config) }
        try await waitUntilStartBegan(start)

        let shutdownTask = Task { await manager.shutdown() }
        // The early shielded stop is the step right before shutdown waits for
        // admitted native operations; destroy can only follow that wait.
        try await waitUntil("the early shielded stop") {
            log.events.contains("teardown:shielded_sync_stop")
        }
        XCTAssertFalse(
            log.events.contains("teardown:destroy"),
            "shutdown must not destroy the handle while an admitted start runs")

        gate.signal()
        try await startTask.value
        let metrics = await shutdownTask.value

        let afterShieldedStop = log.events.filter { $0 != "teardown:shielded_sync_stop" }
        XCTAssertEqual(
            afterShieldedStop.prefix(2), ["spv_start:begin", "spv_start:end"],
            "unexpected event order: \(log.events)")
        XCTAssertEqual(afterShieldedStop.last, "teardown:destroy")
        XCTAssertEqual(metrics.steps.count, 6)
    }

    /// A start requested after shutdown is rejected up front, before any
    /// native work.
    func testStartAfterShutdownThrowsWithoutInvokingNativeCall() async {
        let log = EventLog()
        let start = StartRecorder(gate: nil, eventLog: log)
        let manager = makeManager(handle: 37, start: start, stop: StopRecorder(gate: nil, eventLog: log), log: log)

        await manager.shutdown()

        do {
            try await manager.startSpv(config: Self.config)
            XCTFail("expected a throw after shutdown")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        } catch {
            XCTFail("unexpected error: \(error)")
        }
        XCTAssertEqual(start.count, 0, "the native start must never be reached after shutdown")
    }

    // MARK: - Result mapping

    /// A native failure surfaces as the mapped `PlatformWalletError`, and the
    /// in-flight count is released so the next start is not refused.
    func testNativeFailureIsThrownAndTheInFlightCountReleased() async throws {
        let log = EventLog()
        let start = StartRecorder(
            gate: nil, eventLog: log, code: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_OPERATION)
        let manager = makeManager(handle: 38, start: start, stop: StopRecorder(gate: nil, eventLog: log), log: log)

        do {
            try await manager.startSpv(config: Self.config)
            XCTFail("expected the native failure to be thrown")
        } catch let error as PlatformWalletError {
            guard case .walletOperation = error else {
                return XCTFail("expected walletOperation, got \(error)")
            }
        } catch {
            XCTFail("unexpected error: \(error)")
        }
        XCTAssertEqual(manager.spvStartsInFlight, 0, "the in-flight count must be released after a failed start")
        XCTAssertEqual(start.count, 1)
        await manager.shutdown()
    }
}
