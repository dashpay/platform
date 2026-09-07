import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the progress poller's off-main native reads.
///
/// Tests inject the poll call table (and the teardown / create tables the
/// fixture needs), so the production loop stays under test: every read
/// runs off the main thread, a read that parks does not stall the main
/// actor, ticks never overlap, a failed read keeps the previously
/// published value, and no read runs after `shutdown()` took the handle —
/// all without invoking a production FFI call.
@MainActor
final class PlatformWalletProgressPollTests: XCTestCase {

    /// Records poll invocations and parks the FIRST per-wallet count read
    /// on a semaphore, modelling the `wallet_manager.read()` wait behind a
    /// slow persister commit that used to freeze the main thread.
    private final class PollRecorder: @unchecked Sendable {
        private let lock = NSLock()
        private let firstPendingCountGate: DispatchSemaphore?
        private var gated = false
        private var invocations: [(name: String, handle: Handle, ranOnMainThread: Bool)] = []

        init(firstPendingCountGate: DispatchSemaphore? = nil) {
            self.firstPendingCountGate = firstPendingCountGate
        }

        private func record(_ name: String, _ handle: Handle) {
            lock.withLock { invocations.append((name, handle, Thread.isMainThread)) }
        }

        func makeCalls() -> PlatformWalletNativePollCalls {
            PlatformWalletNativePollCalls(
                syncProgress: { [self] handle in
                    record("sync_progress", handle)
                    throw PlatformWalletError.invalidHandle("simulated failure")
                },
                isSpvRunning: { [self] handle in
                    record("spv_is_running", handle)
                    return true
                },
                connectedSpvPeers: { [self] handle in
                    record("spv_connected_peers", handle)
                    return [PlatformSpvPeerInfo(address: "1.2.3.4:9999", nodeType: .unknown)]
                },
                spvTipBlockTime: { [self] handle in
                    record("spv_tip", handle)
                    return Date(timeIntervalSince1970: 1_700_000_000)
                },
                isPlatformAddressSyncing: { [self] handle in
                    record("platform_address_syncing", handle)
                    return false
                },
                isShieldedSyncing: { [self] handle in
                    record("shielded_syncing", handle)
                    return false
                },
                isDashPaySyncing: { [self] handle in
                    record("dashpay_syncing", handle)
                    throw PlatformWalletError.invalidHandle("simulated failure")
                },
                pendingAccountBuildCount: { [self] handle in
                    record("pending_account_build_count", handle)
                    let parkThisCall = lock.withLock { () -> Bool in
                        if gated { return false }
                        gated = true
                        return true
                    }
                    if parkThisCall {
                        firstPendingCountGate?.wait()
                    }
                    return 3
                }
            )
        }

        var count: Int { lock.withLock { invocations.count } }
        func count(named name: String) -> Int {
            lock.withLock { invocations.filter { $0.name == name }.count }
        }
        var handles: [Handle] { lock.withLock { invocations.map(\.handle) } }
        var mainThreadFlags: [Bool] { lock.withLock { invocations.map(\.ranOnMainThread) } }
    }

    private nonisolated static func makeTeardownCalls() -> PlatformWalletNativeTeardownCalls {
        let ok: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil)
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: ok,
            platformAddressSyncStop: ok,
            shieldedSyncStop: ok,
            dashPaySyncStop: ok,
            dpnsSyncStop: ok,
            destroy: ok
        )
    }

    /// A create table that inserts a wallet with a `NULL_HANDLE` native
    /// handle (never collides with a live registry entry) so the poller has
    /// one per-wallet read to run.
    private nonisolated static func makeCreateCalls() -> PlatformWalletNativeCreateCalls {
        PlatformWalletNativeCreateCalls(createFromMnemonic: { _, _ in
            (
                PlatformWalletFFIResult(code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS, message: nil),
                NULL_HANDLE,
                Data(repeating: 1, count: 32)
            )
        })
    }

    private func waitUntil(
        timeout: Duration = .seconds(5),
        _ condition: () -> Bool
    ) async throws {
        let deadline = ContinuousClock.now + timeout
        while !condition() {
            if ContinuousClock.now > deadline {
                XCTFail("condition not met within \(timeout)")
                return
            }
            try await Task.sleep(for: .milliseconds(5))
        }
    }

    func testPollRunsOffMainStaysResponsiveWhileParkedAndStopsAtShutdown() async throws {
        let gate = DispatchSemaphore(value: 0)
        let recorder = PollRecorder(firstPendingCountGate: gate)
        let manager = PlatformWalletManager.makeForTesting(handle: 77, calls: Self.makeTeardownCalls())
        manager.nativeCreateCalls = Self.makeCreateCalls()
        manager.nativePollCalls = recorder.makeCalls()
        manager.progressPollInterval = .milliseconds(10)
        let wallet = try await manager.createWallet(mnemonic: "m", network: .testnet)

        manager.startProgressPolling()

        // The first tick is parked inside the per-wallet read on the poll queue.
        try await waitUntil { recorder.count(named: "pending_account_build_count") == 1 }

        // The main actor stays responsive while the native read is parked: a
        // main-queue round trip completes promptly, and nothing from the
        // parked tick has been published yet.
        let started = ContinuousClock.now
        await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
            DispatchQueue.main.async { continuation.resume() }
        }
        XCTAssertLessThan(ContinuousClock.now - started, .milliseconds(500))
        XCTAssertFalse(manager.spvIsRunning, "nothing publishes before the tick completes")
        XCTAssertEqual(recorder.count(named: "spv_is_running"), 1, "a slow tick must not be overlapped")
        XCTAssertEqual(recorder.handles.first, 77)
        XCTAssertFalse(recorder.mainThreadFlags.contains(true), "native reads never run on the main thread")

        gate.signal()
        try await waitUntil {
            manager.spvIsRunning
                && manager.dashPayUnlockStatus[wallet.walletId]?.pendingAccountBuilds == 3
        }
        XCTAssertEqual(manager.spvPeers.map(\.address), ["1.2.3.4:9999"])
        XCTAssertEqual(manager.spvTipBlockTime, Date(timeIntervalSince1970: 1_700_000_000))
        XCTAssertFalse(manager.dashPaySyncIsSyncing, "a failed read keeps the previously published value")

        // Ticks keep coming (the gate parks only the first per-wallet read).
        try await waitUntil { recorder.count(named: "spv_is_running") >= 3 }
        XCTAssertFalse(recorder.mainThreadFlags.contains(true))

        await manager.shutdown()
        manager.pollQueue.sync {}  // drain a tick that was already in flight
        let countAfterShutdown = recorder.count
        try await Task.sleep(for: .milliseconds(100))  // ≈10 poll intervals
        XCTAssertEqual(
            recorder.count, countAfterShutdown,
            "no native read may run after shutdown took the handle")
        XCTAssertEqual(manager.handle, NULL_HANDLE)
    }

    func testPerformPollLeavesFailedReadsNilAndFoldsWallets() {
        let walletA = Data(repeating: 0xA, count: 32)
        let walletB = Data(repeating: 0xB, count: 32)
        let calls = PlatformWalletNativePollCalls(
            syncProgress: { _ in throw PlatformWalletError.invalidHandle("no spv") },
            isSpvRunning: { _ in true },
            connectedSpvPeers: { _ in [] },
            spvTipBlockTime: { _ in nil },
            isPlatformAddressSyncing: { _ in false },
            isShieldedSyncing: { _ in true },
            isDashPaySyncing: { _ in throw PlatformWalletError.invalidHandle("no dashpay") },
            pendingAccountBuildCount: { handle in
                if handle == 20 { throw PlatformWalletError.invalidHandle("gone") }
                return 3
            }
        )

        let snapshot = PlatformWalletManager.performPoll(
            5,
            wallets: [(walletId: walletA, handle: 10), (walletId: walletB, handle: 20)],
            calls: calls
        )

        XCTAssertNil(snapshot.spvProgress)
        XCTAssertNil(snapshot.dashPaySyncIsSyncing)
        XCTAssertEqual(snapshot.spvIsRunning, true)
        XCTAssertEqual(snapshot.shieldedSyncIsSyncing, true)
        XCTAssertEqual(snapshot.platformAddressSyncIsSyncing, false)
        XCTAssertEqual(snapshot.spvPeers, [])
        XCTAssertNil(snapshot.spvTipBlockTime)
        XCTAssertEqual(snapshot.pendingAccountBuilds, [walletA: 3])
    }
}
