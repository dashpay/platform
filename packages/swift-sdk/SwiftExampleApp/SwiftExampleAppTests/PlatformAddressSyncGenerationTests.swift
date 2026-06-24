import XCTest
@testable import SwiftDashSDK

/// Regression coverage for the platform-address (BLAST/DIP-17) sync
/// generation guard in
/// `PlatformWalletManager.handlePlatformAddressSyncCompleted(_:generation:)`.
///
/// A completion event is snapshotted with the current generation on the FFI
/// callback thread, then re-dispatched onto the main actor. `stopPlatformAddressSync`
/// / `resetPlatformAddressSyncState` (the Clear path) bump the generation
/// counter, so a trailing event the main actor delivers *after* the stop/reset
/// must be dropped — otherwise it repaints chain-tip height, last-sync time, and
/// metrics over the just-cleared sync-status UI (the exact bug a generation-less
/// platform path had, which Shielded Clear already guards against).
///
/// `handlePlatformAddressSyncCompleted` does not touch the FFI handle — it only
/// reads `platformAddressSyncGeneration` and assigns `lastPlatformAddressSyncEvent`
/// — so a bare, unconfigured `PlatformWalletManager()` is safe to drive directly.
@MainActor
final class PlatformAddressSyncGenerationTests: XCTestCase {

    private func makeEvent(syncUnixSeconds: UInt64) -> PlatformAddressSyncEvent {
        PlatformAddressSyncEvent(syncUnixSeconds: syncUnixSeconds, walletResults: [])
    }

    /// The exact race Clear closes: a completion captured under generation N is
    /// dropped once the counter is bumped to N+1 before delivery —
    /// `lastPlatformAddressSyncEvent` stays nil.
    func testStaleCompletionIsDroppedAfterGenerationBump() {
        let manager = PlatformWalletManager()
        XCTAssertNil(manager.lastPlatformAddressSyncEvent, "fresh manager has no event")

        // Snapshot at "enqueue" time the way the FFI callback thread does,
        // then advance it (as stop/reset would) before delivery.
        let capturedGeneration = manager.platformAddressSyncGeneration.current()
        manager.platformAddressSyncGeneration.bump()

        manager.handlePlatformAddressSyncCompleted(
            makeEvent(syncUnixSeconds: 1_000),
            generation: capturedGeneration
        )

        XCTAssertNil(
            manager.lastPlatformAddressSyncEvent,
            "a completion captured under a superseded generation must be dropped"
        )
    }

    /// A completion captured under the CURRENT generation is published, so a
    /// normal sync (no intervening Clear) still updates the UI.
    func testCurrentGenerationCompletionIsPublished() {
        let manager = PlatformWalletManager()

        let currentGeneration = manager.platformAddressSyncGeneration.current()
        manager.handlePlatformAddressSyncCompleted(
            makeEvent(syncUnixSeconds: 2_000),
            generation: currentGeneration
        )

        XCTAssertEqual(
            manager.lastPlatformAddressSyncEvent?.syncUnixSeconds,
            2_000,
            "a completion captured under the current generation must be published"
        )
    }

    /// A previously-published event must not be clobbered by a late straggler
    /// carrying a superseded (pre-bump) generation — the cleared/updated state
    /// stays put.
    func testStaleCompletionDoesNotOverwriteAlreadyPublishedEvent() {
        let manager = PlatformWalletManager()

        let firstGeneration = manager.platformAddressSyncGeneration.current()
        manager.handlePlatformAddressSyncCompleted(
            makeEvent(syncUnixSeconds: 5_000),
            generation: firstGeneration
        )
        XCTAssertEqual(manager.lastPlatformAddressSyncEvent?.syncUnixSeconds, 5_000)

        // Bump (stop/reset), then deliver a straggler carrying the old snapshot.
        manager.platformAddressSyncGeneration.bump()
        manager.handlePlatformAddressSyncCompleted(
            makeEvent(syncUnixSeconds: 6_000),
            generation: firstGeneration
        )

        XCTAssertEqual(
            manager.lastPlatformAddressSyncEvent?.syncUnixSeconds,
            5_000,
            "a superseded straggler must not overwrite the last valid event"
        )
    }

    /// After a reset the retained published mirror must be dropped. The
    /// generation guard only blocks *future* stale callbacks; the value
    /// already held on `lastPlatformAddressSyncEvent` would otherwise replay to
    /// a fresh `configure()` subscriber (Combine emits the current `@Published`
    /// value on subscribe) and repaint the cleared sync-status UI.
    func testResetPublishedMirrorDropsRetainedEvent() {
        let manager = PlatformWalletManager()
        manager.lastPlatformAddressSyncEvent = makeEvent(syncUnixSeconds: 7_000)
        XCTAssertNotNil(manager.lastPlatformAddressSyncEvent)

        manager.resetPlatformAddressPublishedMirror()

        XCTAssertNil(
            manager.lastPlatformAddressSyncEvent,
            "reset must drop the retained completion so it can't replay on re-subscribe"
        )
        XCTAssertFalse(
            manager.platformAddressSyncIsSyncing,
            "reset must clear the syncing mirror so a re-subscribe doesn't replay a stale spinner"
        )
    }
}
