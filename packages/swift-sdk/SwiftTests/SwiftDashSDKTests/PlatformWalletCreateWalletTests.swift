import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the async off-main `createWallet(mnemonic:)` overload.
///
/// Tests inject the native create call (and the teardown table, for the
/// shutdown-interplay cases), so the production `performCreateWallet`
/// orchestration stays under test: off-main execution, result mapping,
/// the shutdown drain (an admitted create completes before teardown takes
/// the handle; new creates are rejected while the drain runs), and the
/// FIFO ordering with teardown on the shared destroy queue — all without
/// invoking a production create or teardown call.
@MainActor
final class PlatformWalletCreateWalletTests: XCTestCase {

    /// Records create invocations and (optionally) blocks inside the native
    /// call, so a test can interleave a `shutdown()` with an in-flight
    /// off-main create.
    private final class CreateRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let gate: DispatchSemaphore?
        private let failingCode: PlatformWalletFFIResultCode?
        private let eventLog: EventLog?
        private var invocations: [(handle: Handle, mnemonic: String, ranOnMainThread: Bool)] = []
        private var inFlight = 0
        private var maxInFlightSeen = 0

        init(
            gate: DispatchSemaphore? = nil,
            failingCode: PlatformWalletFFIResultCode? = nil,
            eventLog: EventLog? = nil
        ) {
            self.gate = gate
            self.failingCode = failingCode
            self.eventLog = eventLog
        }

        func record(
            handle: Handle,
            params: PlatformWalletCreateParams
        ) -> (result: PlatformWalletFFIResult, walletHandle: Handle, walletId: Data) {
            lock.withLock {
                invocations.append((handle, params.mnemonic, Thread.isMainThread))
                inFlight += 1
                maxInFlightSeen = max(maxInFlightSeen, inFlight)
            }
            eventLog?.append("create:begin")
            gate?.wait()
            let index = lock.withLock { invocations.count }
            defer {
                eventLog?.append("create:end")
                lock.withLock { inFlight -= 1 }
            }
            if let failingCode {
                return (
                    PlatformWalletFFIResult(code: failingCode, message: nil),
                    NULL_HANDLE,
                    Data()
                )
            }
            // `ManagedPlatformWallet.deinit` destroys its handle through the
            // live FFI. Keep the fake handle at zero so it can never collide
            // with a real process-global Rust registry entry; the distinct
            // wallet ids are sufficient for the concurrency assertions.
            return (
                PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil),
                NULL_HANDLE,
                Data(repeating: UInt8(index), count: 32)
            )
        }

        var count: Int { lock.withLock { invocations.count } }
        var handles: [Handle] { lock.withLock { invocations.map(\.handle) } }
        var mainThreadFlags: [Bool] { lock.withLock { invocations.map(\.ranOnMainThread) } }
        /// Peak number of native creates running at once — 1 proves the
        /// shared queue actually serialized concurrent callers.
        var maxInFlight: Int { lock.withLock { maxInFlightSeen } }
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
        await manager.shutdown()
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
        await manager.shutdown()
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

    /// A shutdown DURING the off-main create window must WAIT: the admitted
    /// create completes its full transaction (native create + publish) and
    /// only then does the teardown take the handle and run — an admitted
    /// create is never failed retroactively after its FFI persisted wallet
    /// data (the caller would roll back its mnemonic and orphan the rows).
    func testShutdownDuringCreateWindowWaitsForCreateThenTearsDown() async throws {
        let gate = DispatchSemaphore(value: 0)
        let log = EventLog()
        let recorder = CreateRecorder(gate: gate, eventLog: log)
        let manager = makeManager(handle: 11, createRecorder: recorder, teardownLog: log)

        let createTask = Task { try await manager.createWallet(mnemonic: "m", network: .testnet) }

        // Wait until the create was admitted and is blocked inside the
        // native call on the destroy queue.
        while recorder.count == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }

        // Start the shutdown; it must drain the in-flight create BEFORE
        // taking the handle — so the handle stays live while the gate holds.
        let shutdownTask = Task { await manager.shutdown() }
        try await Task.sleep(for: .milliseconds(30))
        XCTAssertEqual(
            manager.handle, 11,
            "shutdown must not take the handle while an admitted create is in flight")

        gate.signal()
        let wallet = try await createTask.value
        let metrics = await shutdownTask.value

        XCTAssertTrue(
            manager.wallets[wallet.walletId] === wallet,
            "the drained create must have completed its publish")
        XCTAssertEqual(manager.handle, NULL_HANDLE)
        XCTAssertEqual(metrics.steps.count, 6)
        // The full ordering is in the shared event log: the create ended
        // before the first teardown step began.
        XCTAssertEqual(
            log.events.prefix(2), ["create:begin", "create:end"],
            "unexpected event order: \(log.events)")
        XCTAssertEqual(log.events.count, 8)
        XCTAssertEqual(log.events[2], "teardown:spv_stop")
    }

    /// New async and synchronous creates arriving while a shutdown is draining
    /// are rejected up front — before any native work.
    func testAllCreateOverloadsDuringShutdownDrainAreRejectedBeforeNativeWork() async throws {
        let gate = DispatchSemaphore(value: 0)
        let recorder = CreateRecorder(gate: gate)
        // Use a value outside any realistic monotonic registry range so even a
        // regression that reaches the live FFI can only produce a safe miss.
        let manager = makeManager(handle: Handle.max, createRecorder: recorder)

        let firstCreate = Task { try await manager.createWallet(mnemonic: "a", network: .testnet) }
        while recorder.count == 0 {
            try await Task.sleep(for: .milliseconds(5))
        }
        let shutdownTask = Task { await manager.shutdown() }

        // Poll through the seed overload with deliberately invalid input. It
        // cannot enter FFI before shutdown, and once shutdown closes admission
        // the shared creation guard must take precedence over seed validation.
        while true {
            do {
                _ = try manager.createWallet(seed: Data(), network: .testnet)
                XCTFail("an empty seed must never create a wallet")
                break
            } catch PlatformWalletError.invalidParameter {
                await Task.yield()
            } catch PlatformWalletError.invalidHandle(let message) {
                XCTAssertEqual(
                    message,
                    "manager shutdown is in progress; wallet creation rejected")
                break
            } catch {
                XCTFail("unexpected error while waiting for shutdown admission to close: \(error)")
                break
            }
        }

        func assertSynchronousShutdownRejection(
            _ operation: () throws -> ManagedPlatformWallet,
            file: StaticString = #filePath,
            line: UInt = #line
        ) {
            do {
                _ = try operation()
                XCTFail("expected rejection during the shutdown drain", file: file, line: line)
            } catch PlatformWalletError.invalidHandle(let message) {
                XCTAssertEqual(
                    message,
                    "manager shutdown is in progress; wallet creation rejected",
                    file: file,
                    line: line)
            } catch {
                XCTFail("unexpected error: \(error)", file: file, line: line)
            }
        }

        // The explicit synchronous function type prevents async-overload
        // selection in this async test context.
        let createFromMnemonicSynchronously: () throws -> ManagedPlatformWallet = {
            try manager.createWallet(mnemonic: "sync", network: .testnet)
        }
        assertSynchronousShutdownRejection(createFromMnemonicSynchronously)

        do {
            _ = try await manager.createWallet(mnemonic: "b", network: .testnet)
            XCTFail("expected rejection during the shutdown drain")
        } catch let error as PlatformWalletError {
            guard case .invalidHandle(let message) = error else {
                return XCTFail("expected invalidHandle, got \(error)")
            }
            XCTAssertEqual(
                message,
                "manager shutdown is in progress; wallet creation rejected")
        }

        gate.signal()
        _ = try await firstCreate.value
        _ = await shutdownTask.value
        XCTAssertEqual(recorder.count, 1, "rejected creates must never reach native work")
    }

    // MARK: - Concurrent creates

    func testConcurrentCreatesBothSerializeAndPublish() async throws {
        let recorder = CreateRecorder()
        let manager = makeManager(handle: 21, createRecorder: recorder)

        async let first = manager.createWallet(mnemonic: "a", network: .testnet)
        async let second = manager.createWallet(mnemonic: "b", network: .testnet)
        let (w1, w2) = try await (first, second)

        XCTAssertEqual(recorder.count, 2)
        XCTAssertEqual(
            recorder.maxInFlight, 1,
            "the shared serial queue must never run two native creates at once")
        XCTAssertNotEqual(w1.walletId, w2.walletId)
        XCTAssertEqual(manager.wallets.count, 2)
        XCTAssertTrue(manager.wallets[w1.walletId] === w1)
        XCTAssertTrue(manager.wallets[w2.walletId] === w2)
        await manager.shutdown()
    }
}
