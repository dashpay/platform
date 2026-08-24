import XCTest
@testable import SwiftDashSDK

/// Coverage for `PlatformWalletManager.shutdown()` — the explicit, off-main
/// replacement for the old synchronous `deinit` teardown.
///
/// Every test runs on the internal test seam (`makeForTesting(handle:teardown:)`
/// + `nativeTeardownOverride`): a fake non-null handle exercises the real
/// take-once / exactly-once / idempotency paths, while the injected teardown
/// closure counts invocations and can hold completion — no FFI is touched, so
/// the tests are deterministic and need no configured SDK.
///
/// Isolation: each test that leaves a live manager behind finishes it with an
/// explicit `await manager.shutdown()` (or drains `destroyQueue`), so the
/// asynchronous deinit fallback of one test can never overlap the next.
@MainActor
final class PlatformWalletShutdownTests: XCTestCase {

    /// Thread-safe invocation recorder shared with the injected teardown
    /// closure (which runs on the SDK's destroy queue, off the main actor).
    private final class TeardownRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private var callCount = 0
        private var handles: [Handle] = []
        private var ranOnMainThread: [Bool] = []

        func record(handle: Handle) {
            lock.withLock {
                callCount += 1
                handles.append(handle)
                ranOnMainThread.append(Thread.isMainThread)
            }
        }

        var count: Int { lock.withLock { callCount } }
        var recordedHandles: [Handle] { lock.withLock { handles } }
        var recordedMainThread: [Bool] { lock.withLock { ranOnMainThread } }
    }

    /// `nonisolated static` on purpose: the injected teardown closures are
    /// `@Sendable` and run on the destroy queue, off the main actor.
    private nonisolated static func makeMetrics(code: Int32 = 0) -> PlatformWalletShutdownMetrics {
        PlatformWalletShutdownMetrics(
            steps: [.init(name: "destroy", ffiCode: code, milliseconds: 1)],
            totalMilliseconds: 1,
            ranOffMainThread: !Thread.isMainThread
        )
    }

    /// Drain the destroy queue so any scheduled (fallback) teardown has
    /// provably run before the test asserts.
    private func drainDestroyQueue() {
        PlatformWalletManager.destroyQueue.sync {}
    }

    // MARK: - Idempotency

    /// Two concurrent callers await ONE teardown and receive the same metrics.
    func testConcurrentShutdownRunsTeardownExactlyOnce() async {
        let recorder = TeardownRecorder()
        let manager = PlatformWalletManager.makeForTesting(handle: 42) { handle in
            recorder.record(handle: handle)
            return PlatformWalletShutdownMetrics(
                steps: [.init(name: "destroy", ffiCode: 0, milliseconds: 7)],
                totalMilliseconds: 7,
                ranOffMainThread: !Thread.isMainThread
            )
        }

        async let first = manager.shutdown()
        async let second = manager.shutdown()
        let (m1, m2) = await (first, second)

        XCTAssertEqual(recorder.count, 1, "one native teardown for two callers")
        XCTAssertEqual(recorder.recordedHandles, [42], "the taken handle reaches the teardown")
        XCTAssertEqual(m1.totalMilliseconds, m2.totalMilliseconds, "both callers get the same metrics")
        XCTAssertEqual(m1.steps.map(\.name), m2.steps.map(\.name))

        // A third, late caller still gets the recorded outcome, no new run.
        let third = await manager.shutdown()
        XCTAssertEqual(recorder.count, 1)
        XCTAssertEqual(third.totalMilliseconds, m1.totalMilliseconds)
    }

    // MARK: - Take-once handle

    /// The first `shutdown()` consumes the handle: it reads NULL afterwards
    /// and every guarded entry point rejects.
    func testShutdownConsumesHandleExactlyOnce() async {
        let recorder = TeardownRecorder()
        let manager = PlatformWalletManager.makeForTesting(handle: 7) { handle in
            recorder.record(handle: handle)
            return Self.makeMetrics()
        }
        XCTAssertEqual(manager.handle, 7)
        XCTAssertTrue(manager.isConfigured)

        await manager.shutdown()

        XCTAssertEqual(manager.handle, NULL_HANDLE, "handle is zeroed by the take-once")
        XCTAssertFalse(manager.isConfigured)
        XCTAssertThrowsError(try manager.ensureConfigured(), "operations must fail fast after shutdown")
        XCTAssertEqual(recorder.count, 1)
    }

    /// A never-configured manager (NULL handle) shuts down as a no-op —
    /// no teardown call, empty metrics — and stays idempotent.
    func testShutdownWithoutHandleIsANoOp() async {
        let manager = PlatformWalletManager()
        let metrics = await manager.shutdown()
        XCTAssertTrue(metrics.steps.isEmpty)
        let again = await manager.shutdown()
        XCTAssertTrue(again.steps.isEmpty)
    }

    // MARK: - Caller cancellation

    /// Cancelling the calling task does not interrupt the native teardown:
    /// it runs to completion exactly once and the cancelled caller still
    /// receives the metrics.
    func testCallerCancellationDoesNotInterruptTeardown() async {
        let recorder = TeardownRecorder()
        let gate = DispatchSemaphore(value: 0)
        let manager = PlatformWalletManager.makeForTesting(handle: 9) { handle in
            gate.wait() // hold completion until the test releases it
            recorder.record(handle: handle)
            return Self.makeMetrics()
        }

        let caller = Task { await manager.shutdown() }
        caller.cancel()
        gate.signal()

        let metrics = await caller.value
        XCTAssertEqual(recorder.count, 1, "teardown ran despite the cancelled caller")
        XCTAssertEqual(metrics.steps.map(\.name), ["destroy"])
    }

    // MARK: - Deinit interplay

    /// After an explicit `shutdown()`, deallocating the manager schedules no
    /// second teardown (the handle was already consumed).
    func testDeinitAfterShutdownRunsNoSecondTeardown() async {
        let recorder = TeardownRecorder()
        var manager: PlatformWalletManager? = PlatformWalletManager.makeForTesting(handle: 11) { handle in
            recorder.record(handle: handle)
            return Self.makeMetrics()
        }

        await manager?.shutdown()
        XCTAssertEqual(recorder.count, 1)

        manager = nil
        drainDestroyQueue()
        XCTAssertEqual(recorder.count, 1, "deinit must not run a second teardown")
    }

    /// Dropping a live manager WITHOUT `shutdown()` triggers the emergency
    /// fallback: exactly one teardown, off the main thread (the whole point —
    /// the blocking destroy must never run on whatever thread ARC releases on).
    func testDeinitFallbackRunsTeardownOffMainExactlyOnce() {
        let recorder = TeardownRecorder()
        var manager: PlatformWalletManager? = PlatformWalletManager.makeForTesting(handle: 13) { handle in
            recorder.record(handle: handle)
            return PlatformWalletShutdownMetrics(
                steps: [], totalMilliseconds: 0, ranOffMainThread: !Thread.isMainThread)
        }
        withExtendedLifetime(manager) {}

        manager = nil // MainActor release → deinit schedules the fallback
        drainDestroyQueue()

        XCTAssertEqual(recorder.count, 1, "fallback teardown runs exactly once")
        XCTAssertEqual(recorder.recordedMainThread, [false], "fallback teardown must not run on the main thread")
        XCTAssertEqual(recorder.recordedHandles, [13])
    }

    // MARK: - Error propagation

    /// A failing FFI step's code travels in the returned metrics — the
    /// lifecycle layer logs it; it never turns into a thrown error (destroy
    /// of a live handle reports Success by Rust contract, and step failures
    /// carry no control-flow decision).
    func testFailingStepCodeLandsInMetrics() async {
        let failingCode = PlatformWalletResultCode.errorInvalidHandle.rawValue
        let manager = PlatformWalletManager.makeForTesting(handle: 21) { _ in
            PlatformWalletShutdownMetrics(
                steps: [
                    .init(name: "spv_stop", ffiCode: 0, milliseconds: 2),
                    .init(name: "destroy", ffiCode: failingCode, milliseconds: 3),
                ],
                totalMilliseconds: 5,
                ranOffMainThread: !Thread.isMainThread
            )
        }

        let metrics = await manager.shutdown()

        XCTAssertEqual(metrics.steps.count, 2)
        XCTAssertEqual(metrics.steps.last?.ffiCode, failingCode, "step failure code is preserved for the host to log")
        XCTAssertTrue(metrics.ranOffMainThread)
    }
}
