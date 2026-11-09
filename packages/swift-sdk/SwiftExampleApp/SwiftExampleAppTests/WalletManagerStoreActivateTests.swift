import Combine
import DashSDKFFI
import SwiftData
import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins how `WalletManagerStore.activate` replaces a cached manager that was
/// configured against an SDK the app has since rebuilt:
///
/// - the replaced manager's native teardown finishes before its replacement
///   becomes the active manager. Its sync workers keep writing the network's
///   persisted wallets until that teardown ends, and dropping the manager only
///   schedules the teardown for whenever its last reference goes, so two
///   managers would run against the same wallets;
/// - a replacement that fails to configure leaves the current manager cached,
///   active and running, so the app keeps working on the SDK it keeps, and a
///   later retry still replaces it.
///
/// Real regtest SDKs and native managers: building either needs no network,
/// and nothing here starts a sync.
@MainActor
final class WalletManagerStoreActivateTests: XCTestCase {

    private final class EventLog: @unchecked Sendable {
        private let lock = NSLock()
        private var entries: [String] = []
        func append(_ event: String) { lock.withLock { entries.append(event) } }
        var events: [String] { lock.withLock { entries } }
    }

    func testShouldFinishShuttingDownTheReplacedManagerBeforeItsReplacementTakesOver() async throws {
        let store = WalletManagerStore(modelContainer: try DashModelContainer.createInMemory())
        try await store.activate(
            network: .regtest, sdk: try SDK(network: .regtest, contextProvider: .spv))
        let replaced = store.activeManager
        XCTAssertTrue(replaced.isConfigured)

        // Record when the replaced manager's native destroy returns, and when
        // the store swaps its active manager.
        let log = EventLog()
        let calls = replaced.nativeTeardownCalls
        replaced.nativeTeardownCalls = PlatformWalletNativeTeardownCalls(
            spvStop: calls.spvStop,
            platformAddressSyncStop: calls.platformAddressSyncStop,
            shieldedSyncStop: calls.shieldedSyncStop,
            dashPaySyncStop: calls.dashPaySyncStop,
            dpnsSyncStop: calls.dpnsSyncStop,
            destroy: { handle in
                let result = calls.destroy(handle)
                log.append("destroy")
                return result
            },
            spvStart: calls.spvStart)
        let swaps = store.$activeManager.dropFirst().sink { _ in log.append("swap") }
        defer { swaps.cancel() }

        try await store.activate(
            network: .regtest, sdk: try SDK(network: .regtest, contextProvider: .spv))

        XCTAssertNotIdentical(store.activeManager, replaced)
        XCTAssertIdentical(store.manager(for: .regtest), store.activeManager)
        XCTAssertFalse(replaced.isConfigured)
        XCTAssertEqual(
            log.events, ["destroy", "swap"],
            "the replaced manager must be torn down before its replacement becomes active"
        )
        await store.activeManager.shutdown()
    }

    func testShouldKeepTheCurrentManagerWhenItsReplacementFailsToConfigure() async throws {
        let store = WalletManagerStore(modelContainer: try DashModelContainer.createInMemory())
        try await store.activate(
            network: .regtest, sdk: try SDK(network: .regtest, contextProvider: .spv))
        let current = store.activeManager

        // Only one manager can hold an SDK's SPV quorum source slot, so a
        // rebuilt SDK whose slot is already held makes the replacement's
        // `configure` throw.
        let rebuilt = try SDK(network: .regtest, contextProvider: .spv)
        let slotHolder = try PlatformWalletManager(sdk: rebuilt)
        do {
            try await store.activate(network: .regtest, sdk: rebuilt)
            XCTFail("a second manager on one SDK must fail to configure")
        } catch {
            XCTAssertTrue(
                error.localizedDescription.contains("already serves this SDK's quorum keys"),
                error.localizedDescription)
        }

        XCTAssertIdentical(store.activeManager, current)
        XCTAssertIdentical(store.manager(for: .regtest), current)
        XCTAssertTrue(
            current.isConfigured,
            "a failed replacement must not shut down the manager the app keeps"
        )

        // Once the slot is free, the same rebuild replaces the kept manager.
        await slotHolder.shutdown()
        try await store.activate(network: .regtest, sdk: rebuilt)
        XCTAssertNotIdentical(store.activeManager, current)
        XCTAssertFalse(current.isConfigured)
        await store.activeManager.shutdown()
    }
}
