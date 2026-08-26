import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the async off-main `createWallet(mnemonic:)` overload.
///
/// Tests inject the native create call (and the teardown table, for the
/// shutdown-interplay cases), so the production `performCreateWallet`
/// orchestration stays under test: off-main execution, result mapping,
/// the shutdown-race epilogue, and the FIFO ordering with teardown on the
/// shared destroy queue — all without calling FFI.
@MainActor
final class PlatformWalletCreateWalletTests: XCTestCase {

    /// Records create invocations and (optionally) blocks inside the native
    /// call, so a test can interleave a `shutdown()` with an in-flight
    /// off-main create.
    private final class CreateRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let gate: DispatchSemaphore?
        private let failingCode: PlatformWalletFFIResultCode?
        private var invocations: [(handle: Handle, mnemonic: String, ranOnMainThread: Bool)] = []

        init(gate: DispatchSemaphore? = nil, failingCode: PlatformWalletFFIResultCode? = nil) {
            self.gate = gate
            self.failingCode = failingCode
        }

        func record(
            handle: Handle,
            params: PlatformWalletCreateParams
        ) -> (result: PlatformWalletFFIResult, walletHandle: Handle, walletId: Data) {
            lock.withLock {
                invocations.append((handle, params.mnemonic, Thread.isMainThread))
            }
            gate?.wait()
            let index = lock.withLock { invocations.count }
            if let failingCode {
                return (
                    PlatformWalletFFIResult(code: failingCode, message: nil),
                    NULL_HANDLE,
                    Data()
                )
            }
            // Distinct per-invocation wallet handle/id so concurrent creates
            // stay distinguishable.
            return (
                PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil),
                Handle(100 + index),
                Data(repeating: UInt8(index), count: 32)
            )
        }

        var count: Int { lock.withLock { invocations.count } }
        var handles: [Handle] { lock.withLock { invocations.map(\.handle) } }
        var mainThreadFlags: [Bool] { lock.withLock { invocations.map(\.ranOnMainThread) } }
    }

    private nonisolated static func makeCreateCalls(
        recorder: CreateRecorder
    ) -> PlatformWalletNativeCreateCalls {
        PlatformWalletNativeCreateCalls(
            createFromMnemonic: { handle, params in
                recorder.record(handle: handle, params: params)
            }
        )
    }

    /// Teardown table whose steps append to a shared event log, so the
    /// FIFO ordering of create vs teardown on the shared queue is provable.
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
        createRecorder: CreateRecorder,
        teardownLog: EventLog = EventLog()
    ) -> PlatformWalletManager {
        let manager = PlatformWalletManager.makeForTesting(
            handle: handle,
            calls: Self.makeTeardownCalls(log: teardownLog)
        )
        manager.nativeCreateCalls = Self.makeCreateCalls(recorder: createRecorder)
        return manager
    }

    // MARK: - Off-main execution + success path

    func testCreateRunsOffMainAndPublishesWallet() async throws {
        let recorder = CreateRecorder()
        let manager = makeManager(handle: 42, createRecorder: recorder)

        let wallet = try await manager.createWallet(mnemonic: "m", network: .testnet)

        XCTAssertEqual(recorder.count, 1)
        XCTAssertEqual(recorder.handles, [42])
        XCTAssertEqual(recorder.mainThreadFlags, [false], "the native create must run off the main thread")
        XCTAssertEqual(wallet.walletId, Data(repeating: 1, count: 32))
        XCTAssertTrue(manager.wallets[wallet.walletId] === wallet)
    }

    // MARK: - Error mapping

    func testCreateErrorCodeMapsToTypedError() async {
        let recorder = CreateRecorder(
            failingCode: PLATFORM_WALLET_FFI_RESULT_CODE_ERROR_WALLET_ALREADY_EXISTS)
        let manager = makeManager(handle: 7, createRecorder: recorder)

        do {
            _ = try await manager.createWallet(mnemonic: "m", network: .testnet)
            XCTFail("expected walletAlreadyExists")
        } catch let error as PlatformWalletError {
            guard case .walletAlreadyExists = error else {
                return XCTFail("expected walletAlreadyExists, got \(error)")
            }
        } catch {
            XCTFail("unexpected error: \(error)")
        }
        XCTAssertTrue(manager.wallets.isEmpty)
    }

    // MARK: - Shutdown interplay

    func testCreateAfterShutdownThrowsWithoutInvokingNativeCall() async {
        let recorder = CreateRecorder()
        let manager = makeManager(handle: 9, createRecorder: recorder)

        await manager.shutdown()

        do {
            _ = try await manager.createWallet(mnemonic: "m", network: .testnet)
            XCTFail("expected a throw after shutdown")
        } catch is PlatformWalletError {
            // expected: ensureConfigured rejects the torn-down manager
        } catch {
            XCTFail("unexpected error: \(error)")
        }
        XCTAssertEqual(recorder.count, 0, "the native create must never be reached after shutdown")
    }

    /// A shutdown DURING the off-main create window: the epilogue must not
    /// publish into the torn-down manager, and the shared-queue FIFO must
    /// run the native teardown only after the in-flight create returned.
    func testShutdownDuringCreateWindowDiscardsWalletAndTearsDownAfterCreate() async throws {
        let gate = DispatchSemaphore(value: 0)
        let recorder = CreateRecorder(gate: gate)
        let log = EventLog()
        let manager = makeManager(handle: 11, createRecorder: recorder, teardownLog: log)

        let createTask = Task { try await manager.createWallet(mnemonic: "m", network: .testnet) }

        // Wait until the create was admitted and is blocked inside the
        // native call on the destroy queue.
        while recorder.count == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }

        // Start the shutdown; its main-actor prologue takes the handle
        // immediately, while its teardown block queues behind the gated
        // create on the shared queue.
        let shutdownTask = Task { await manager.shutdown() }
        while manager.handle != NULL_HANDLE {
            try await Task.sleep(for: .milliseconds(5))
        }

        gate.signal()
        _ = await shutdownTask.value

        do {
            _ = try await createTask.value
            XCTFail("expected invalidHandle from the epilogue")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
        }
        XCTAssertTrue(manager.wallets.isEmpty, "a shutdown-raced create must not publish")
        XCTAssertEqual(
            log.events.first, "teardown:spv_stop",
            "teardown must have queued AFTER the gated create released the shared queue")
        XCTAssertEqual(log.events.count, 6)
    }

    // MARK: - Concurrent creates

    func testConcurrentCreatesBothSerializeAndPublish() async throws {
        let recorder = CreateRecorder()
        let manager = makeManager(handle: 21, createRecorder: recorder)

        async let first = manager.createWallet(mnemonic: "a", network: .testnet)
        async let second = manager.createWallet(mnemonic: "b", network: .testnet)
        let (w1, w2) = try await (first, second)

        XCTAssertEqual(recorder.count, 2)
        XCTAssertNotEqual(w1.walletId, w2.walletId)
        XCTAssertEqual(manager.wallets.count, 2)
        XCTAssertTrue(manager.wallets[w1.walletId] === w1)
        XCTAssertTrue(manager.wallets[w2.walletId] === w2)
    }
}
