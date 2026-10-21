import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the async off-main `stopShieldedSync()` overload.
///
/// Tests inject the native shielded stop through the teardown table, so the
/// production orchestration stays under test: the native stop runs off the
/// main thread, the shielded event handlers drop events while it runs, a
/// drained stop bumps the generation, the shielded lifecycle calls are
/// refused while it runs, and `shutdown()` waits for it.
@MainActor
final class PlatformWalletStopShieldedSyncTests: XCTestCase {

    private static let walletId = Data(repeating: 7, count: 32)

    private final class EventLog: @unchecked Sendable {
        private let lock = NSLock()
        private var entries: [String] = []
        func append(_ event: String) { lock.withLock { entries.append(event) } }
        var events: [String] { lock.withLock { entries } }
    }

    /// Fake native shielded stop. The first call records its thread, blocks
    /// on `gate`, runs `beforeFirstReturn` and returns `firstResultCode`;
    /// later calls (a second stop, or shutdown's early stop) return success
    /// at once. The gate wait gives up after 10 s, so a stop that ran on the
    /// main thread (blocking the test that would open the gate) fails the
    /// test instead of hanging it.
    private final class StopRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let gate: DispatchSemaphore?
        private let eventLog: EventLog
        private let firstResultCode: PlatformWalletFFIResultCode
        private var mainThreadFlags: [Bool] = []
        private var hook: (@Sendable () -> Void)?

        init(
            gate: DispatchSemaphore?,
            eventLog: EventLog,
            firstResultCode: PlatformWalletFFIResultCode = PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS
        ) {
            self.gate = gate
            self.eventLog = eventLog
            self.firstResultCode = firstResultCode
        }

        /// Runs on the stop's queue right before the first call returns.
        var beforeFirstReturn: (@Sendable () -> Void)? {
            get { lock.withLock { hook } }
            set { lock.withLock { hook = newValue } }
        }

        func stop() -> PlatformWalletFFIResult {
            let isFirst = lock.withLock { () -> Bool in
                mainThreadFlags.append(Thread.isMainThread)
                return mainThreadFlags.count == 1
            }
            eventLog.append("shielded_stop:begin")
            if isFirst {
                _ = gate?.wait(timeout: .now() + 10)
                beforeFirstReturn?()
            }
            eventLog.append("shielded_stop:end")
            return PlatformWalletFFIResult(
                code: isFirst ? firstResultCode : PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS,
                message: nil)
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
            spvStop: step("spv_stop"),
            platformAddressSyncStop: step("platform_address_sync_stop"),
            shieldedSyncStop: { _ in recorder.stop() },
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
                  message.contains("Shielded sync stop in progress") else {
                return XCTFail("\(name): expected the in-flight stop refusal, got \(error)", file: file, line: line)
            }
        }
    }

    /// Async variant of the refusal assertion, for the manual sync calls.
    private func assertRefusedWhileStopping(
        _ call: () async throws -> Void,
        _ name: String,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async {
        do {
            try await call()
            XCTFail("\(name): expected the in-flight stop refusal", file: file, line: line)
        } catch PlatformWalletError.walletOperation(let message)
            where message.contains("Shielded sync stop in progress") {
            // Expected.
        } catch {
            XCTFail("\(name): expected the in-flight stop refusal, got \(error)", file: file, line: line)
        }
    }

    private static func event(_ seconds: UInt64) -> ShieldedSyncEvent {
        ShieldedSyncEvent(syncUnixSeconds: seconds, walletResults: [])
    }

    private func assertNothingPublished(
        _ manager: PlatformWalletManager,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertNil(manager.lastShieldedSyncEvent, file: file, line: line)
        XCTAssertNil(manager.currentShieldedSyncScanned, file: file, line: line)
        XCTAssertNil(manager.currentShieldedSyncBlockHeight, file: file, line: line)
        XCTAssertNil(manager.currentShieldedTreeCommitted, file: file, line: line)
        XCTAssertNil(manager.currentShieldedTreeTotal, file: file, line: line)
    }

    // MARK: - Off-main execution

    /// The native stop runs off the main thread, and the main actor keeps
    /// running work while that stop is blocked.
    func testStopRunsOffMainWhileTheMainActorStaysFree() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 31, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)

        XCTAssertFalse(
            log.events.contains("shielded_stop:end"),
            "the main actor must run while the native stop is still blocked")
        XCTAssertEqual(manager.shieldedStopsInFlight, 1)

        gate.signal()
        try await stopTask.value

        XCTAssertEqual(recorder.ranOnMainThread, [false], "the native stop must run off the main thread")
        XCTAssertEqual(manager.shieldedStopsInFlight, 0)
        await manager.shutdown()
    }

    // MARK: - Events

    /// With the main actor free during the drain, events of the pass being
    /// stopped arrive while their generation is still current. Progress is
    /// dropped and the completion held; a stop that drains the pass then
    /// drops the held completion.
    func testEventsDuringADrainingStopAreNeverPublished() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 32, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)

        let generation = manager.shieldedSyncGeneration.current()
        manager.handleShieldedSyncCompleted(Self.event(1_000), generation: generation)
        manager.handleShieldedSyncProgress(cumulativeScanned: 10, blockHeight: 20, generation: generation)
        manager.handleShieldedTreeProgress(committed: 30, total: 40, generation: generation)
        assertNothingPublished(manager)

        gate.signal()
        try await stopTask.value
        assertNothingPublished(manager)
        XCTAssertNil(manager.shieldedCompletionHeldByStop)
        await manager.shutdown()
    }

    /// A completion that lands while a stop is timing out belongs to a pass
    /// that keeps running: the stop publishes it once it returns.
    func testCompletionHeldDuringATimedOutStopIsPublishedWhenItReturns() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(
            gate: gate, eventLog: log,
            firstResultCode: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHUTDOWN_INCOMPLETE)
        let manager = makeManager(handle: 42, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)
        manager.handleShieldedSyncCompleted(Self.event(1_000), generation: manager.shieldedSyncGeneration.current())
        XCTAssertNil(manager.lastShieldedSyncEvent, "the completion waits for the stop to settle it")

        gate.signal()
        do {
            try await stopTask.value
            XCTFail("expected shutdownIncomplete")
        } catch PlatformWalletError.shutdownIncomplete {
            // Expected.
        } catch {
            XCTFail("unexpected error: \(error)")
        }

        XCTAssertEqual(manager.lastShieldedSyncEvent?.syncUnixSeconds, 1_000)
        XCTAssertNil(manager.shieldedCompletionHeldByStop)
        await manager.shutdown()
    }

    /// Like the blocking overload, the progress mirrors are cleared only by
    /// a stop that drains the pass: after a timed-out stop the pass keeps
    /// running, and its progress stays visible.
    func testProgressIsClearedOnlyByAStopThatDrains() async throws {
        let log = EventLog()
        let drained = makeManager(handle: 43, recorder: StopRecorder(gate: nil, eventLog: log), log: log)
        let timedOut = makeManager(
            handle: 44,
            recorder: StopRecorder(
                gate: nil, eventLog: log,
                firstResultCode: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHUTDOWN_INCOMPLETE),
            log: log)
        for manager in [drained, timedOut] {
            let generation = manager.shieldedSyncGeneration.current()
            manager.handleShieldedSyncProgress(cumulativeScanned: 10, blockHeight: 20, generation: generation)
            manager.handleShieldedTreeProgress(committed: 30, total: 40, generation: generation)
        }

        try await drained.stopShieldedSync()
        do {
            try await timedOut.stopShieldedSync()
            XCTFail("expected shutdownIncomplete")
        } catch PlatformWalletError.shutdownIncomplete {
            // Expected.
        }

        assertNothingPublished(drained)
        XCTAssertEqual(timedOut.currentShieldedSyncScanned, 10)
        XCTAssertEqual(timedOut.currentShieldedSyncBlockHeight, 20)
        XCTAssertEqual(timedOut.currentShieldedTreeCommitted, 30)
        XCTAssertEqual(timedOut.currentShieldedTreeTotal, 40)
        await drained.shutdown()
        await timedOut.shutdown()
    }

    /// An event the pass dispatches just before the native stop returns
    /// snapshots the old generation and reaches the main actor around the
    /// stop's end: it is dropped either by the in-flight guard or by the
    /// bump. An event of a later pass is published.
    func testEventDispatchedAsTheStopReturnsIsDroppedAndALaterOneIsPublished() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 33, recorder: recorder, log: log)
        let delivered = expectation(description: "the trailing event reached the main actor")
        recorder.beforeFirstReturn = { [weak manager] in
            // What the FFI trampoline does: snapshot on the callback thread,
            // then hop to the main actor.
            let generation = manager?.shieldedSyncGeneration.current() ?? 0
            Task { @MainActor [weak manager] in
                manager?.handleShieldedSyncCompleted(Self.event(1_000), generation: generation)
                delivered.fulfill()
            }
        }
        let generationBefore = manager.shieldedSyncGeneration.current()

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)
        gate.signal()
        try await stopTask.value
        await fulfillment(of: [delivered], timeout: 5)

        XCTAssertNil(manager.lastShieldedSyncEvent, "the stopped pass's trailing event must be dropped")
        XCTAssertEqual(manager.shieldedSyncGeneration.current(), generationBefore + 1, "a drained stop bumps once")

        manager.handleShieldedSyncCompleted(Self.event(2_000), generation: manager.shieldedSyncGeneration.current())
        XCTAssertEqual(manager.lastShieldedSyncEvent?.syncUnixSeconds, 2_000)
        await manager.shutdown()
    }

    /// A stop whose pass does not drain in time throws `shutdownIncomplete`
    /// and, like the blocking overload, leaves the generation alone: the pass
    /// keeps running, and its later events are real data.
    func testTimedOutStopThrowsAndKeepsTheGeneration() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(
            gate: gate, eventLog: log,
            firstResultCode: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_SHUTDOWN_INCOMPLETE)
        let manager = makeManager(handle: 34, recorder: recorder, log: log)
        let generationBefore = manager.shieldedSyncGeneration.current()

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)
        gate.signal()
        do {
            try await stopTask.value
            XCTFail("expected shutdownIncomplete")
        } catch PlatformWalletError.shutdownIncomplete {
            // Expected.
        } catch {
            XCTFail("unexpected error: \(error)")
        }

        XCTAssertEqual(manager.shieldedSyncGeneration.current(), generationBefore)
        XCTAssertEqual(manager.shieldedStopsInFlight, 0)
        manager.handleShieldedSyncCompleted(Self.event(1_000), generation: generationBefore)
        XCTAssertEqual(
            manager.lastShieldedSyncEvent?.syncUnixSeconds, 1_000,
            "after a timed-out stop the still-running pass's events are published")
        await manager.shutdown()
    }

    /// Any other native failure still releases the in-flight count and the
    /// admission, so a later shutdown completes.
    func testOtherNativeFailureReleasesTheInFlightCountAndAdmission() async throws {
        let log = EventLog()
        let recorder = StopRecorder(
            gate: nil, eventLog: log,
            firstResultCode: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_DESERIALIZATION)
        let manager = makeManager(handle: 35, recorder: recorder, log: log)

        do {
            try await manager.stopShieldedSync()
            XCTFail("expected the native failure to throw")
        } catch {
            if case PlatformWalletError.walletOperation(let message) = error {
                XCTAssertFalse(message.contains("Shielded sync stop in progress"))
            }
        }

        XCTAssertEqual(manager.shieldedStopsInFlight, 0)
        let metrics = await manager.shutdown()
        XCTAssertEqual(metrics.steps.last?.name, "destroy")
    }

    // MARK: - Lifecycle calls while stopping

    /// With the main actor free during the stop, the shielded calls that
    /// would restart the loop, start a pass that outlives the stop, or wait
    /// on the same drain are refused, and the synchronous native operations
    /// refuse to run beside the admitted stop.
    func testLifecycleCallsAreRefusedWhileAnAsyncStopIsInFlight() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 36, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)

        // A synchronous context resolves `stopShieldedSync()` to the
        // blocking overload.
        func blockingStop() throws { try manager.stopShieldedSync() }
        assertRefusedWhileStopping(try manager.startShieldedSync(), "startShieldedSync")
        assertRefusedWhileStopping(try blockingStop(), "blocking stopShieldedSync()")
        assertRefusedWhileStopping(try manager.clearShielded(), "clearShielded")
        let resolver = MnemonicResolver()
        assertRefusedWhileStopping(
            try manager.bindShielded(walletId: Self.walletId, resolver: resolver), "bindShielded")
        await assertRefusedWhileStopping({ try await manager.syncShieldedNow() }, "syncShieldedNow")
        await assertRefusedWhileStopping(
            { try await manager.syncShieldedWalletNow(walletId: Self.walletId) }, "syncShieldedWalletNow")

        func syncCreate() throws {
            _ = try manager.createWallet(mnemonic: "unused", network: .testnet)
        }
        XCTAssertThrowsError(try syncCreate()) { error in
            guard case PlatformWalletError.invalidHandle(let message) = error,
                  message.contains("async native operation is in flight") else {
                return XCTFail("expected the synchronous create to be refused, got \(error)")
            }
        }

        gate.signal()
        try await stopTask.value
        XCTAssertEqual(recorder.count, 1, "no refused call may reach the native stop")
        await manager.shutdown()
    }

    /// Two overlapping async stops run one after the other on the serial
    /// stop queue, and both release their count.
    func testOverlappingStopsRunOneAfterTheOther() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 37, recorder: recorder, log: log)

        let first = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)
        let second = Task { try await manager.stopShieldedSync() }
        try await waitUntil("the second stop's admission") { manager.shieldedStopsInFlight == 2 }
        XCTAssertEqual(recorder.count, 1, "the second native stop must wait for the first")

        gate.signal()
        try await first.value
        try await second.value

        XCTAssertEqual(recorder.count, 2)
        XCTAssertEqual(
            log.events,
            ["shielded_stop:begin", "shielded_stop:end", "shielded_stop:begin", "shielded_stop:end"])
        XCTAssertEqual(manager.shieldedStopsInFlight, 0)
        await manager.shutdown()
    }

    /// Cancelling the caller's task does not cut the stop short: the native
    /// stop finishes, and the bump and the count release still run.
    func testCancellingTheCallerDoesNotSkipTheStepAfterTheStop() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 38, recorder: recorder, log: log)
        let generationBefore = manager.shieldedSyncGeneration.current()

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)
        stopTask.cancel()
        gate.signal()
        try await stopTask.value

        XCTAssertEqual(manager.shieldedSyncGeneration.current(), generationBefore + 1)
        XCTAssertEqual(manager.shieldedStopsInFlight, 0)
        await manager.shutdown()
    }

    // MARK: - Shutdown interplay

    /// A shutdown during an in-flight stop waits for it: its early shielded
    /// stop shares the serial stop queue and starts only after this stop's
    /// native call returns, and destroy follows the admitted stop.
    func testShutdownDuringAStopRunsItsEarlyStopAfterIt() async throws {
        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        let recorder = StopRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 39, recorder: recorder, log: log)

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)

        let shutdownTask = Task { await manager.shutdown() }
        try await waitUntil("the shutdown request") { manager.shutdownRequested }
        XCTAssertEqual(recorder.count, 1, "the early stop must queue behind the in-flight stop")
        XCTAssertFalse(log.events.contains("teardown:destroy"))

        gate.signal()
        try await stopTask.value
        let metrics = await shutdownTask.value

        XCTAssertEqual(recorder.count, 2)
        XCTAssertEqual(
            Array(log.events.prefix(4)),
            ["shielded_stop:begin", "shielded_stop:end", "shielded_stop:begin", "shielded_stop:end"],
            "unexpected event order: \(log.events)")
        XCTAssertEqual(log.events.last, "teardown:destroy")
        XCTAssertEqual(metrics.steps.count, 6)
    }

    /// A stop that finishes while shutdown drains admitted operations skips
    /// its bump: a local balance read admitted before the shutdown is
    /// delivered instead of being cancelled as obsolete.
    func testStopFinishingDuringShutdownDoesNotCancelAnAdmittedSnapshotRead() async throws {
        let stopGate = DispatchSemaphore(value: 0)
        let readGate = DispatchSemaphore(value: 0)
        defer {
            stopGate.signal()
            readGate.signal()
        }
        let log = EventLog()
        let recorder = StopRecorder(gate: stopGate, eventLog: log)
        let manager = makeManager(handle: 40, recorder: recorder, log: log)
        let readStarted = DispatchSemaphore(value: 0)
        manager.nativeShieldedLocalBalanceCalls = PlatformWalletNativeShieldedLocalBalanceCalls(
            read: { _, _ in
                readStarted.signal()
                _ = readGate.wait(timeout: .now() + 10)
                return (
                    PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil),
                    ShieldedLocalBalanceSnapshotFFI(
                        status: SHIELDED_LOCAL_BALANCE_STATUS_FFI_UNBOUND, accounts: nil, accounts_count: 0))
            },
            free: { snapshot in snapshot.pointee = ShieldedLocalBalanceSnapshotFFI() })

        let stopTask = Task { try await manager.stopShieldedSync() }
        try await waitUntilStopStarted(recorder)
        let readTask = Task { try await manager.localShieldedBalanceSnapshot(walletId: Self.walletId) }
        try await waitUntil("the native read starting") { readStarted.wait(timeout: .now()) == .success }
        let shutdownTask = Task { await manager.shutdown() }
        try await waitUntil("the shutdown request") { manager.shutdownRequested }

        stopGate.signal()
        try await stopTask.value
        readGate.signal()

        do {
            let state = try await readTask.value
            XCTAssertEqual(state, .unbound)
        } catch {
            XCTFail("the admitted read must be delivered, got \(error)")
        }
        _ = await shutdownTask.value
    }

    /// A stop requested after shutdown, or before configuration, is rejected
    /// before any native work.
    func testStopAfterShutdownOrBeforeConfigureThrowsWithoutInvokingNativeCall() async {
        let log = EventLog()
        let recorder = StopRecorder(gate: nil, eventLog: log)
        let manager = makeManager(handle: 41, recorder: recorder, log: log)

        await manager.shutdown()
        let stopsDuringTeardown = recorder.count

        do {
            try await manager.stopShieldedSync()
            XCTFail("expected a throw after shutdown")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        } catch {
            XCTFail("unexpected error: \(error)")
        }
        XCTAssertEqual(recorder.count, stopsDuringTeardown, "the native stop must never be reached after shutdown")

        let unconfigured = PlatformWalletManager()
        unconfigured.nativeTeardownCalls = Self.makeTeardownCalls(recorder: recorder, log: log)
        do {
            try await unconfigured.stopShieldedSync()
            XCTFail("expected a throw before configure")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        } catch {
            XCTFail("unexpected error: \(error)")
        }
        XCTAssertEqual(recorder.count, stopsDuringTeardown, "the native stop must never be reached before configure")
    }
}
