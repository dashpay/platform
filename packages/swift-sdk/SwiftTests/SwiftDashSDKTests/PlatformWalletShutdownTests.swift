import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for `PlatformWalletManager.shutdown()` — the explicit, off-main
/// replacement for the old synchronous `deinit` teardown.
///
/// Tests inject the six individual native functions, not the teardown result.
/// The production `performNativeTeardown` orchestration therefore remains
/// under test: order, handle propagation, result mapping, off-main execution,
/// take-once behavior, and idempotency are all exercised without calling FFI.
@MainActor
final class PlatformWalletShutdownTests: XCTestCase {

    private final class TeardownRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let failingStep: String?
        private let firstCallGate: DispatchSemaphore?
        private var invocations: [(name: String, handle: Handle, ranOnMainThread: Bool)] = []

        init(failingStep: String? = nil, firstCallGate: DispatchSemaphore? = nil) {
            self.failingStep = failingStep
            self.firstCallGate = firstCallGate
        }

        func record(name: String, handle: Handle) -> PlatformWalletFFIResult {
            if name == "spv_stop" {
                firstCallGate?.wait()
            }
            lock.withLock {
                invocations.append((name, handle, Thread.isMainThread))
            }
            let code = name == failingStep
                ? PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_INVALID_HANDLE
                : PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS
            return PlatformWalletFFIResult(code: code, message: nil)
        }

        var names: [String] { lock.withLock { invocations.map(\.name) } }
        var handles: [Handle] { lock.withLock { invocations.map(\.handle) } }
        var mainThreadFlags: [Bool] { lock.withLock { invocations.map(\.ranOnMainThread) } }
        func count(named name: String) -> Int {
            lock.withLock { invocations.count { $0.name == name } }
        }
    }

    private static let expectedOrder = [
        "spv_stop",
        "platform_address_sync_stop",
        "shielded_sync_stop",
        "dashpay_sync_stop",
        "dpns_sync_stop",
        "destroy",
    ]

    private nonisolated static func makeCalls(
        recorder: TeardownRecorder
    ) -> PlatformWalletNativeTeardownCalls {
        PlatformWalletNativeTeardownCalls(
            spvStop: { recorder.record(name: "spv_stop", handle: $0) },
            platformAddressSyncStop: {
                recorder.record(name: "platform_address_sync_stop", handle: $0)
            },
            shieldedSyncStop: { recorder.record(name: "shielded_sync_stop", handle: $0) },
            dashPaySyncStop: { recorder.record(name: "dashpay_sync_stop", handle: $0) },
            dpnsSyncStop: { recorder.record(name: "dpns_sync_stop", handle: $0) },
            destroy: { recorder.record(name: "destroy", handle: $0) }
        )
    }

    private func drainDestroyQueue() {
        PlatformWalletManager.destroyQueue.sync {}
    }

    // MARK: - Idempotency

    func testConcurrentShutdownRunsTeardownExactlyOnce() async {
        let recorder = TeardownRecorder()
        let manager = PlatformWalletManager.makeForTesting(
            handle: 42,
            calls: Self.makeCalls(recorder: recorder)
        )

        async let first = manager.shutdown()
        async let second = manager.shutdown()
        let (m1, m2) = await (first, second)

        XCTAssertEqual(recorder.names, Self.expectedOrder)
        XCTAssertEqual(recorder.handles, Array(repeating: 42, count: 6))
        XCTAssertEqual(recorder.count(named: "destroy"), 1)
        XCTAssertEqual(m1.totalMilliseconds, m2.totalMilliseconds)
        XCTAssertEqual(m1.steps.map(\.name), m2.steps.map(\.name))

        let third = await manager.shutdown()
        XCTAssertEqual(recorder.names, Self.expectedOrder, "a late caller must not start teardown again")
        XCTAssertEqual(third.totalMilliseconds, m1.totalMilliseconds)
    }

    // MARK: - Take-once handle

    func testShutdownConsumesHandleExactlyOnce() async {
        let recorder = TeardownRecorder()
        let manager = PlatformWalletManager.makeForTesting(
            handle: 7,
            calls: Self.makeCalls(recorder: recorder)
        )
        XCTAssertEqual(manager.handle, 7)
        XCTAssertTrue(manager.isConfigured)

        await manager.shutdown()

        XCTAssertEqual(manager.handle, NULL_HANDLE)
        XCTAssertFalse(manager.isConfigured)
        XCTAssertThrowsError(try manager.ensureConfigured())
        XCTAssertEqual(recorder.count(named: "destroy"), 1)
    }

    func testShutdownWithoutHandleIsANoOp() async {
        let manager = PlatformWalletManager()
        let metrics = await manager.shutdown()
        XCTAssertTrue(metrics.steps.isEmpty)
        let again = await manager.shutdown()
        XCTAssertTrue(again.steps.isEmpty)
    }

    func testShutdownBeforeConfigurationDoesNotSuppressLaterTeardown() async throws {
        let recorder = TeardownRecorder()
        let manager = PlatformWalletManager()

        let noOp = await manager.shutdown()
        XCTAssertTrue(noOp.steps.isEmpty)

        try manager.configureForTesting(
            handle: 17,
            calls: Self.makeCalls(recorder: recorder)
        )

        let metrics = await manager.shutdown()
        XCTAssertEqual(recorder.names, Self.expectedOrder)
        XCTAssertEqual(recorder.handles, Array(repeating: 17, count: 6))
        XCTAssertEqual(metrics.steps.map(\.name), Self.expectedOrder)
    }

    func testDiagnosticsDoNotBlockSyncAdmissionButShutdownWaitsForThem() async throws {
        let recorder = TeardownRecorder()
        let manager = PlatformWalletManager.makeForTesting(
            handle: 19,
            calls: Self.makeCalls(recorder: recorder)
        )

        try manager.admitCoreDiagnosticsNativeOp()

        // Diagnostics only perform read-only FFI work. A synchronous wallet
        // operation must therefore pass native-op admission while diagnostics
        // are active. The empty seed then fails at its own validation seam,
        // proving admission did not reject it as an overlapping native op.
        XCTAssertThrowsError(
            try manager.createWallet(seed: Data(), network: .testnet)
        ) { error in
            guard let walletError = error as? PlatformWalletError,
                  case .invalidParameter = walletError
            else {
                return XCTFail("expected invalidParameter, got \(error)")
            }
        }

        let shutdownTask = Task { await manager.shutdown() }
        try await Task.sleep(for: .milliseconds(20))
        XCTAssertEqual(manager.handle, 19, "shutdown must not take the handle early")
        XCTAssertTrue(recorder.names.isEmpty, "native teardown must wait for diagnostics")

        manager.finishCoreDiagnosticsNativeOp()
        let metrics = await shutdownTask.value

        XCTAssertEqual(manager.handle, NULL_HANDLE)
        XCTAssertEqual(metrics.steps.map(\.name), Self.expectedOrder)
        XCTAssertEqual(recorder.names, Self.expectedOrder)
    }

    /// A completed real shutdown makes this manager terminal. Reconfiguration
    /// must fail before another native handle or callback context is installed.
    func testConfigurationAfterRealShutdownIsRejected() async {
        let recorder = TeardownRecorder()
        let calls = Self.makeCalls(recorder: recorder)
        let manager = PlatformWalletManager.makeForTesting(handle: 17, calls: calls)

        let first = await manager.shutdown()
        XCTAssertEqual(first.steps.map(\.name), Self.expectedOrder)

        XCTAssertThrowsError(try manager.configureForTesting(handle: 18, calls: calls)) { error in
            guard let walletError = error as? PlatformWalletError else {
                return XCTFail("unexpected error: \(error)")
            }
            guard case .invalidHandle(let message) = walletError else {
                return XCTFail("expected invalidHandle, got \(walletError)")
            }
            XCTAssertTrue(message.contains("cannot be configured after shutdown"))
        }

        XCTAssertEqual(manager.handle, NULL_HANDLE)
        XCTAssertFalse(manager.isConfigured)
        let repeated = await manager.shutdown()
        XCTAssertEqual(repeated.steps.map(\.name), Self.expectedOrder)
        XCTAssertEqual(recorder.names, Self.expectedOrder, "rejected configuration must not add teardown calls")
    }

    // MARK: - Caller cancellation

    func testCallerCancellationDoesNotInterruptTeardown() async {
        let gate = DispatchSemaphore(value: 0)
        let recorder = TeardownRecorder(firstCallGate: gate)
        let manager = PlatformWalletManager.makeForTesting(
            handle: 9,
            calls: Self.makeCalls(recorder: recorder)
        )

        let caller = Task { await manager.shutdown() }
        caller.cancel()
        gate.signal()

        let metrics = await caller.value
        XCTAssertEqual(recorder.names, Self.expectedOrder)
        XCTAssertEqual(recorder.count(named: "destroy"), 1)
        XCTAssertEqual(metrics.steps.map(\.name), Self.expectedOrder)
    }

    // MARK: - Deinit interplay

    func testDeinitAfterShutdownRunsNoSecondTeardown() async {
        let recorder = TeardownRecorder()
        var manager: PlatformWalletManager? = PlatformWalletManager.makeForTesting(
            handle: 11,
            calls: Self.makeCalls(recorder: recorder)
        )

        await manager?.shutdown()
        XCTAssertEqual(recorder.names, Self.expectedOrder)

        manager = nil
        drainDestroyQueue()
        XCTAssertEqual(recorder.names, Self.expectedOrder)
        XCTAssertEqual(recorder.count(named: "destroy"), 1)
    }

    func testDeinitFallbackRunsTeardownOffMainExactlyOnce() {
        let recorder = TeardownRecorder()
        var manager: PlatformWalletManager? = PlatformWalletManager.makeForTesting(
            handle: 13,
            calls: Self.makeCalls(recorder: recorder)
        )
        withExtendedLifetime(manager) {}

        manager = nil
        drainDestroyQueue()

        XCTAssertEqual(recorder.names, Self.expectedOrder)
        XCTAssertEqual(recorder.mainThreadFlags, Array(repeating: false, count: 6))
        XCTAssertEqual(recorder.handles, Array(repeating: 13, count: 6))
        XCTAssertEqual(recorder.count(named: "destroy"), 1)
    }

    // MARK: - Production orchestration

    /// The injected calls still run through `performNativeTeardown`, proving
    /// its exact order and association of each native result with its metric.
    func testNativeTeardownOrderAndResultMapping() async {
        let recorder = TeardownRecorder(failingStep: "shielded_sync_stop")
        let manager = PlatformWalletManager.makeForTesting(
            handle: 21,
            calls: Self.makeCalls(recorder: recorder)
        )

        let metrics = await manager.shutdown()

        XCTAssertEqual(recorder.names, Self.expectedOrder)
        XCTAssertEqual(recorder.handles, Array(repeating: 21, count: 6))
        XCTAssertEqual(metrics.steps.map(\.name), Self.expectedOrder)
        XCTAssertEqual(
            metrics.steps.map(\.ffiCode),
            [0, 0, PlatformWalletResultCode.errorInvalidHandle.rawValue, 0, 0, 0]
        )
        XCTAssertTrue(metrics.ranOffMainThread)
    }
}
