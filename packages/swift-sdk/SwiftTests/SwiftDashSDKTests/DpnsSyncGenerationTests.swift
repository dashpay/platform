import XCTest
import DashSDKFFI
@testable import SwiftDashSDK

/// Regression coverage for DPNS completion callbacks that cross the native
/// teardown → main-actor boundary.
@MainActor
final class DpnsSyncGenerationTests: XCTestCase {

    private nonisolated static func noOpCalls() -> PlatformWalletNativeTeardownCalls {
        let success: PlatformWalletNativeTeardownCalls.Call = { _ in
            PlatformWalletFFIResult(
                code: PLATFORM_WALLET_FFI_RESULT_CODE_SUCCESS,
                message: nil
            )
        }
        return PlatformWalletNativeTeardownCalls(
            spvStop: success,
            platformAddressSyncStop: success,
            shieldedSyncStop: success,
            dashPaySyncStop: success,
            dpnsSyncStop: success,
            destroy: success
        )
    }

    private func makeManager() -> PlatformWalletManager {
        PlatformWalletManager.makeForTesting(handle: 71, calls: Self.noOpCalls())
    }

    private func makeEvent(_ timestamp: UInt64) -> DpnsSyncEvent {
        DpnsSyncEvent(syncUnixSeconds: timestamp, walletResults: [])
    }

    func testCurrentGenerationCompletionPublishesWhileConfigured() async {
        let manager = makeManager()
        let generation = manager.dpnsSyncGeneration.current()

        manager.handleDpnsSyncCompleted(makeEvent(1_000), generation: generation)

        XCTAssertEqual(manager.lastDpnsSyncEvent?.syncUnixSeconds, 1_000)
        await manager.shutdown()
    }

    func testStaleCompletionIsDroppedAfterGenerationBump() async {
        let manager = makeManager()
        let staleGeneration = manager.dpnsSyncGeneration.current()
        manager.dpnsSyncGeneration.bump()

        manager.handleDpnsSyncCompleted(makeEvent(2_000), generation: staleGeneration)

        XCTAssertNil(manager.lastDpnsSyncEvent)
        await manager.shutdown()
    }

    /// A generation check alone is insufficient: native teardown can dispatch
    /// a callback after shutdown has bumped the counter. Its snapshot then
    /// matches the current generation, so terminal `isConfigured` must reject
    /// it as well.
    func testPostShutdownCurrentGenerationCompletionIsDropped() async {
        let manager = makeManager()
        let beforeShutdown = manager.dpnsSyncGeneration.current()

        await manager.shutdown()

        let afterShutdown = manager.dpnsSyncGeneration.current()
        XCTAssertNotEqual(beforeShutdown, afterShutdown)
        manager.handleDpnsSyncCompleted(makeEvent(3_000), generation: afterShutdown)
        XCTAssertNil(manager.lastDpnsSyncEvent)
    }

    func testStaleCompletionDoesNotOverwritePublishedEvent() async {
        let manager = makeManager()
        let staleGeneration = manager.dpnsSyncGeneration.current()
        manager.handleDpnsSyncCompleted(makeEvent(4_000), generation: staleGeneration)
        manager.dpnsSyncGeneration.bump()

        manager.handleDpnsSyncCompleted(makeEvent(5_000), generation: staleGeneration)

        XCTAssertEqual(manager.lastDpnsSyncEvent?.syncUnixSeconds, 4_000)
        await manager.shutdown()
    }
}
