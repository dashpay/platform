import SwiftData
import XCTest
import DashSDKFFI
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins the ordering invariants of `stopSpvCancellingPendingStarts`:
///
/// - the pending-start generation is bumped synchronously, before the
///   stop runs, so a start suspended in peer resolution can't resume and
///   revive sync after the stop;
/// - a stop issued while the SDK is already running a start (past the
///   generation check, awaiting the native start) is not dropped: it goes
///   through the SDK's async `stopSpv()`, which queues behind the start on
///   the SPV lifecycle queue, so the native stop runs after the native
///   start returned.
///
/// The first uses the injected-`stop` seam; the second drives a real
/// `PlatformWalletManager` whose native SPV calls are injected, so no
/// Rust runs. `WalletManagerStore.init` is cheap, so bare construction
/// against an in-memory container is side-effect-free.
@MainActor
final class WalletManagerStoreStopSpvTests: XCTestCase {

    private final class EventLog: @unchecked Sendable {
        private let lock = NSLock()
        private var entries: [String] = []
        func append(_ event: String) { lock.withLock { entries.append(event) } }
        var events: [String] { lock.withLock { entries } }
    }

    private struct TimedOut: Error {}

    private func waitUntil(
        timeout: Duration = .seconds(5),
        _ condition: () -> Bool
    ) async throws {
        let deadline = ContinuousClock.now + timeout
        while !condition() {
            guard ContinuousClock.now < deadline else { throw TimedOut() }
            try await Task.sleep(for: .milliseconds(5))
        }
    }

    func testStopSpvCancellingPendingStartsBumpsGenerationBeforeStopping() async throws {
        let container = try DashModelContainer.createInMemory()
        let store = WalletManagerStore(modelContainer: container)

        let before = store.spvStartGeneration
        var stopInvocations = 0
        var generationWhenStopRan: UInt64?

        store.stopSpvCancellingPendingStarts(stop: {
            stopInvocations += 1
            // Read the live generation from inside the stop closure: if
            // invalidation is (correctly) ordered first, it already reads
            // the bumped value.
            generationWhenStopRan = store.spvStartGeneration
        })

        XCTAssertEqual(
            store.spvStartGeneration,
            before &+ 1,
            "generation must be bumped synchronously, before the helper returns"
        )
        try await waitUntil { stopInvocations == 1 }
        XCTAssertEqual(
            generationWhenStopRan,
            before &+ 1,
            "generation must already be bumped by the time stop runs"
        )
        XCTAssertEqual(stopInvocations, 1, "stop must be invoked exactly once")
    }

    /// The Pause button and the global sync indicator can fire while the
    /// SDK is still inside the native start (a store open with a migration
    /// takes seconds). The helper must then stop the client that start
    /// brings up, not silently give up.
    func testStopIssuedDuringAnAdmittedStartRunsAfterTheNativeStart() async throws {
        let container = try DashModelContainer.createInMemory()
        let store = WalletManagerStore(modelContainer: container)

        let gate = DispatchSemaphore(value: 0)
        defer { gate.signal() }
        let log = EventLog()
        func step(_ name: String) -> PlatformWalletNativeTeardownCalls.Call {
            { _ in
                log.append("teardown:\(name)")
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            }
        }
        let calls = PlatformWalletNativeTeardownCalls(
            spvStop: { _ in
                log.append("spv_stop")
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            },
            platformAddressSyncStop: step("platform_address_sync_stop"),
            shieldedSyncStop: step("shielded_sync_stop"),
            dashPaySyncStop: step("dashpay_sync_stop"),
            dpnsSyncStop: step("dpns_sync_stop"),
            destroy: step("destroy"),
            spvStart: { _, _ in
                log.append("spv_start:begin")
                _ = gate.wait(timeout: .now() + 10)
                log.append("spv_start:end")
                return PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
            }
        )
        let manager = PlatformWalletManager.makeForTesting(handle: 41, calls: calls)

        let startTask = Task {
            try await manager.startSpv(config: PlatformSpvStartConfig(dataDir: "/tmp/unused", network: .testnet))
        }
        try await waitUntil { log.events.contains("spv_start:begin") }

        let before = store.spvStartGeneration
        store.stopSpvCancellingPendingStarts(manager)
        XCTAssertEqual(store.spvStartGeneration, before &+ 1, "generation is still bumped synchronously")

        // The stop is admitted (not refused) and waits for the start.
        try await waitUntil { manager.spvStopsInFlight == 1 }
        XCTAssertFalse(log.events.contains("spv_stop"), "the native stop must wait for the in-flight start")

        gate.signal()
        try await startTask.value
        try await waitUntil { log.events.contains("spv_stop") }

        XCTAssertEqual(
            log.events, ["spv_start:begin", "spv_start:end", "spv_stop"],
            "the native stop must run after the native start it overlapped")
        await manager.shutdown()
    }
}
