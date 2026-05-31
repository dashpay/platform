import XCTest
@testable import SwiftDashSDK

/// Regression coverage for the shielded-sync generation guard in
/// `PlatformWalletManager.handleShieldedSyncCompleted(_:generation:)`.
///
/// The guard exists because a shielded-sync completion event is snapshotted
/// with the current generation on the FFI callback thread, then re-dispatched
/// onto the main actor. A `stop`/`clear` bumps the generation counter, so a
/// trailing event that the main actor delivers *after* stop/clear must be
/// dropped — its snapshot predates the bump. Crucially, a same-turn restart
/// must NOT re-open the gate, because the counter is monotonic and never reset.
///
/// `handleShieldedSyncCompleted` does not touch the FFI handle — it only reads
/// `shieldedSyncGeneration` and assigns `lastShieldedSyncEvent` — so a bare,
/// unconfigured `PlatformWalletManager()` is safe to drive directly.
///
/// `PlatformWalletManager` and `handleShieldedSyncCompleted` are main-actor
/// isolated, so the whole suite runs on the main actor.
@MainActor
final class ShieldedSyncGenerationTests: XCTestCase {

    /// Build a deterministic event distinguishable by its timestamp.
    private func makeEvent(syncUnixSeconds: UInt64) -> ShieldedSyncEvent {
        ShieldedSyncEvent(syncUnixSeconds: syncUnixSeconds, walletResults: [])
    }

    /// A completion captured under generation N is dropped once the counter is
    /// bumped to N+1 before delivery: `lastShieldedSyncEvent` stays nil.
    func testStaleCompletionIsDroppedAfterGenerationBump() {
        let manager = PlatformWalletManager()
        XCTAssertNil(manager.lastShieldedSyncEvent, "fresh manager has no event")

        // Snapshot the generation at "enqueue" time, the way the FFI callback
        // thread does, then advance it (as stop/clear would) before delivery.
        let capturedGeneration = manager.shieldedSyncGeneration.current()
        manager.shieldedSyncGeneration.bump()

        let staleEvent = makeEvent(syncUnixSeconds: 1_000)
        manager.handleShieldedSyncCompleted(staleEvent, generation: capturedGeneration)

        XCTAssertNil(
            manager.lastShieldedSyncEvent,
            "an event captured under a superseded generation must be dropped"
        )
    }

    /// A completion captured under the CURRENT generation is published:
    /// `lastShieldedSyncEvent` is set to that exact event.
    func testCurrentGenerationCompletionIsPublished() {
        let manager = PlatformWalletManager()

        let currentGeneration = manager.shieldedSyncGeneration.current()
        let event = makeEvent(syncUnixSeconds: 2_000)
        manager.handleShieldedSyncCompleted(event, generation: currentGeneration)

        XCTAssertEqual(
            manager.lastShieldedSyncEvent?.syncUnixSeconds,
            event.syncUnixSeconds,
            "an event captured under the current generation must be published"
        )
    }

    /// The exact race the guard closes: a stale event captured under the old
    /// generation is dropped even when a same-turn restart follows, and a
    /// later event captured under the (unchanged) current generation is still
    /// published. Proves `bump()` — not any gate reset — is what invalidates.
    func testBumpInvalidatesStaleButNotSubsequentCurrentCompletion() {
        let manager = PlatformWalletManager()

        // 1. Capture under the pre-stop generation, then bump (stop/clear).
        let staleGeneration = manager.shieldedSyncGeneration.current()
        manager.shieldedSyncGeneration.bump()

        // 2. A restart does NOT reset the counter — capture the post-bump
        //    generation that a fresh run's events would carry.
        let liveGeneration = manager.shieldedSyncGeneration.current()
        XCTAssertNotEqual(
            staleGeneration,
            liveGeneration,
            "bump must advance the generation so the stale snapshot no longer matches"
        )

        // 3. The trailing stale event (old snapshot) is delivered and dropped.
        let staleEvent = makeEvent(syncUnixSeconds: 3_000)
        manager.handleShieldedSyncCompleted(staleEvent, generation: staleGeneration)
        XCTAssertNil(
            manager.lastShieldedSyncEvent,
            "the stale completion must not leak into the restarted run"
        )

        // 4. The new run's event (current snapshot) is delivered and published.
        let liveEvent = makeEvent(syncUnixSeconds: 4_000)
        manager.handleShieldedSyncCompleted(liveEvent, generation: liveGeneration)
        XCTAssertEqual(
            manager.lastShieldedSyncEvent?.syncUnixSeconds,
            liveEvent.syncUnixSeconds,
            "the restarted run's completion must be published"
        )
    }

    /// A current-generation event followed by a stale (pre-bump) event keeps
    /// the first event in place: a late straggler must not clobber valid state.
    func testStaleCompletionDoesNotOverwriteAlreadyPublishedEvent() {
        let manager = PlatformWalletManager()

        // Publish a valid event under the current generation.
        let staleGeneration = manager.shieldedSyncGeneration.current()
        let publishedEvent = makeEvent(syncUnixSeconds: 5_000)
        manager.handleShieldedSyncCompleted(publishedEvent, generation: staleGeneration)
        XCTAssertEqual(manager.lastShieldedSyncEvent?.syncUnixSeconds, publishedEvent.syncUnixSeconds)

        // Bump (stop/clear), then deliver a straggler carrying the old snapshot.
        manager.shieldedSyncGeneration.bump()
        let straggler = makeEvent(syncUnixSeconds: 6_000)
        manager.handleShieldedSyncCompleted(straggler, generation: staleGeneration)

        XCTAssertEqual(
            manager.lastShieldedSyncEvent?.syncUnixSeconds,
            publishedEvent.syncUnixSeconds,
            "a superseded straggler must not overwrite the last valid event"
        )
    }
}
