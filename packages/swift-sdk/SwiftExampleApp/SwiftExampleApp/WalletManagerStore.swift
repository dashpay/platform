import Foundation
import SwiftData
import SwiftUI
import SwiftDashSDK

/// Coordinator that holds one `PlatformWalletManager` per network.
///
/// The Rust `PlatformWalletManager` is intentionally network-locked
/// — its inner `WalletManager` is constructed with `sdk.network` at
/// `configure(...)` time and the SDK reference (plus the SPV
/// runtime, BLAST sync coordinator, and persistence handler hung off
/// it) all stay bound to that single network for the manager's
/// lifetime. Switching networks at runtime therefore needs a
/// **different manager**, not a reconfigured one.
///
/// `WalletManagerStore` materializes that design on the iOS side:
///
///   * Lazy-creates a `PlatformWalletManager` the first time a
///     network is activated, keying on `Network` so subsequent
///     activations of the same network return the warm manager.
///   * Each manager's persistence handler is constructed with the
///     same network (via `configure(sdk:modelContainer:)`), which
///     scopes `loadWalletList` to that network's `PersistentWallet`
///     rows — multiple managers writing into the shared SwiftData
///     store don't trip over each other on hydration.
///   * Publishes `activeManager` so the SwiftUI scene can re-inject
///     it via `.environmentObject(...)` on every switch. Existing
///     view consumers (`@EnvironmentObject var walletManager:
///     PlatformWalletManager`) keep working unchanged — they just
///     see the manager for the active network on each render after
///     a switch.
///
/// Inactive managers stay alive in the background. SPV / BLAST sync
/// continue running for them; the user can flip Testnet ↔ Local ↔
/// ... and resume per-network state without losing in-flight sync
/// progress. The cost is N×manager memory for N networks the user
/// has touched this session — tractable for the 4-network upper
/// bound the app supports today, and the right trade-off for a
/// dev-focused example app where flipping is common.
@MainActor
final class WalletManagerStore: ObservableObject {
    /// Currently-active manager. Reassigned when the user switches
    /// networks; SwiftUI re-injects this into the env object so
    /// every `@EnvironmentObject var walletManager:
    /// PlatformWalletManager` consumer rebinds to the right
    /// instance on each render.
    ///
    /// Starts as a placeholder, unconfigured `PlatformWalletManager`
    /// — the bootstrap path replaces it via `activate(...)` once
    /// the SDK for the initial network is ready, and views are
    /// gated behind `isInitialized` until that happens (same
    /// gating the pre-refactor single-manager flow used).
    @Published private(set) var activeManager: PlatformWalletManager

    /// Monotonic token bumped whenever a network switch / SDK rebuild
    /// supersedes any in-flight `CoreSpvLauncher.start`.
    ///
    /// A start captures this synchronously before it begins and
    /// re-checks it (alongside manager identity) after its off-main
    /// peer resolution; a mismatch means the start is stale and must
    /// not revive a stopped or replaced manager. Identity alone is not
    /// enough because of the stop→activate gap: OptionsView stops the
    /// old manager, then awaits the SDK rebuild, and the store swaps
    /// `activeManager` only after that. This token is invalidated at
    /// the *earliest* synchronous point of each such path, closing the
    /// window. See the guard in `CoreSpvLauncher.start`.
    ///
    /// Not `@Published` — nothing renders off it; it's read
    /// synchronously by start call sites and their revalidation
    /// closures only.
    private(set) var spvStartGeneration: UInt64 = 0

    /// Invalidate every in-flight SPV start.
    ///
    /// Invariant: **every user-visible or programmatic stop / replace of
    /// the active manager's SPV must bump the generation first.** Call
    /// this at the earliest synchronous point of any such path (network
    /// switch, devnet/SDK rebuild, peer-config change, a Stop/Pause
    /// button) so a start suspended in the stop→activate gap bails
    /// instead of reviving the superseded manager. User-facing stop
    /// buttons should route through `stopSpvCancellingPendingStarts(_:)`
    /// so they can't forget; supersession flows (see `OptionsView`)
    /// invalidate explicitly at their flow's earliest sync point.
    func invalidatePendingSpvStarts() {
        spvStartGeneration &+= 1
    }

    /// Stop `manager`'s SPV client after cancelling any in-flight
    /// `CoreSpvLauncher.start`. The blessed path for every user-visible
    /// or programmatic SPV stop of the active manager: it bumps the
    /// generation first, synchronously, so a start suspended in peer
    /// resolution can't resume and revive sync after the stop. The stop
    /// itself goes through the SDK's async `stopSpv()`: a start already
    /// admitted by the SDK (past the generation check, awaiting the native
    /// start) cannot be cancelled, and the blocking `stopSpv()` would be
    /// refused while it runs; the async one queues behind it on the SDK's
    /// SPV lifecycle queue and stops the client that start brings up.
    /// Best-effort (errors are logged, not surfaced), matching the raw
    /// call sites it replaces.
    func stopSpvCancellingPendingStarts(_ manager: PlatformWalletManager) {
        stopSpvCancellingPendingStarts(stop: { try await manager.stopSpv() })
    }

    /// Testable core of `stopSpvCancellingPendingStarts(_:)`: the actual
    /// stop is injected so a unit test can assert the generation was
    /// already bumped by the time `stop` runs (pinning the
    /// invalidate-before-stop ordering) and that a stop overlapping an
    /// admitted start runs after it, without a real, network-locked
    /// `PlatformWalletManager`. The public overload supplies the real
    /// `try await manager.stopSpv()`. The generation is bumped before this
    /// returns; the stop runs on a task that starts in order after it.
    func stopSpvCancellingPendingStarts(stop: @escaping @MainActor () async throws -> Void) {
        invalidatePendingSpvStarts()
        isStoppingSpv = true
        let task = Task { @MainActor in
            do {
                try await stop()
            } catch {
                print("stopSpvCancellingPendingStarts: stop failed: \(error)")
            }
        }
        pendingSpvStop = task
        Task { @MainActor in
            await task.value
            // A later Pause replaced the tracked stop; it clears the flag.
            guard pendingSpvStop == task else { return }
            pendingSpvStop = nil
            isStoppingSpv = false
        }
    }

    /// Whether a stop issued by `stopSpvCancellingPendingStarts` is still
    /// running. The native stop removes the client before its teardown
    /// finishes, so `spvIsRunning` turns false while the SDK still counts
    /// the stop in flight and refuses a start; the Start button is disabled
    /// on this instead, and start actions await `awaitPendingSpvStop()`.
    @Published private(set) var isStoppingSpv = false

    /// The stop `isStoppingSpv` reports; `nil` once it completed.
    private var pendingSpvStop: Task<Void, Never>?

    /// Suspend until the tracked stop, if any, has completed, so a start
    /// issued right after a Pause runs after it instead of being refused
    /// by the SDK while the stop is in flight.
    func awaitPendingSpvStop() async {
        await pendingSpvStop?.value
    }

    /// Per-network managers. Lazily populated on first activation
    /// of each network; lookup is O(1).
    private var managers: [Network: PlatformWalletManager] = [:]

    /// SDK the cached manager was configured against.
    /// Used in `activate()` to detect a stale cache when `AppState`
    /// rebuilds the SDK (network switch / `platformDAPIAddresses` or
    /// `platformQuorumURL` change in Options). On mismatch we tear
    /// down the cached manager and rebuild against the fresh SDK —
    /// otherwise the cached manager keeps using its own Sdk clone
    /// with stale DAPI / quorum endpoints, and proof verification
    /// fails forever ("no available addresses to use").
    ///
    /// Held strongly and compared by identity: comparing raw handles
    /// would let a new SDK allocated at a freed SDK's address pass as
    /// the cached one.
    private var managerSdks: [Network: SDK] = [:]

    /// SwiftData container shared across every manager. Each
    /// manager's persistence handler narrows its `loadWalletList`
    /// fetch to its own network so the shared store doesn't cause
    /// cross-network bleed at restore time.
    private let modelContainer: ModelContainer

    init(modelContainer: ModelContainer) {
        self.modelContainer = modelContainer
        // Placeholder. Held until the first `activate(...)` call
        // replaces it with a configured manager. Views gate against
        // bootstrap via `isInitialized` so they never see this
        // intermediate value.
        self.activeManager = PlatformWalletManager()
    }

    /// Activate the manager for `network`, lazily creating + caching
    /// one configured against `sdk` if it's not already warm.
    ///
    /// Idempotent: re-activating the current network is a no-op.
    /// A failure in the fresh manager's `configure` propagates to the
    /// caller. The cache and the active manager stay untouched in that
    /// case, including a cached manager the fresh one would have
    /// replaced, so a later retry can succeed.
    ///
    /// Replacing a cached manager configured against another SDK awaits
    /// that manager's `shutdown()` after the fresh one is configured and
    /// before it restores wallets, so the old manager's teardown has
    /// finished before the fresh one touches the network's persisted
    /// wallets. That can take as long as the old manager's workers need to
    /// stop. Until then the old manager stays cached, and active if it
    /// was, but its handle is already revoked: `manager(for:)` and
    /// `backgroundManager(for:)` still return it, and calls on it fail
    /// with `invalidHandle`. Calls must not overlap for the same network:
    /// callers run one rebuild at a time.
    ///
    /// `makeActive == false` materializes the manager into the cache
    /// without swapping `activeManager`, unless the manager it replaces is
    /// the active one. Used by background flows (e.g. orphan recovery
    /// routing wallets to their original network) that need a manager for
    /// a non-active network without triggering a user-visible network
    /// switch.
    func activate(network: Network, sdk: SDK, makeActive: Bool = true) async throws {
        // Stale-cache check: the cached manager's Sdk clone is locked
        // in at `configure` time (the FFI is single-shot — see
        // `PlatformWalletManager.configure`'s `precondition(!isConfigured)`).
        // If `AppState` has rebuilt the SDK since (network switch or
        // UserDefaults endpoint override change), the cached manager
        // is still pointing at the old endpoints and would keep
        // failing proof verification. The manager built below against
        // the fresh SDK replaces it once `configure` succeeds.
        if let existing = managers[network] {
            if managerSdks[network] === sdk {
                if makeActive && existing !== activeManager {
                    // Swapping to a different warm manager supersedes any
                    // in-flight SPV start against the outgoing one.
                    // Defensive backstop so every present/future activate
                    // path is covered even if a caller forgets to
                    // invalidate at its own supersession point.
                    invalidatePendingSpvStarts()
                    activeManager = existing
                }
                return
            }
            SDKLogger.event(
                "manager_cache_invalidated",
                category: .lifecycle,
                fields: [
                    "network": .publicText(network.displayName),
                    "reason": .publicText("sdk_changed"),
                ]
            )
            // The stale entry stays cached, and active if it was, until
            // the manager built below replaces it. If `configure` throws,
            // the store still tracks the manager that stays active on the
            // SDK the app keeps.
        }
        let manager = PlatformWalletManager()
        try manager.configure(sdk: sdk, modelContainer: modelContainer)
        // A replaced active manager is shut down below, so its replacement
        // must become active even if the caller did not ask for that.
        let becomesActive = makeActive || managers[network] === activeManager
        if let stale = managers[network] {
            // Nothing below can fail the replacement, so the stale manager
            // goes now, before the fresh one loads this network's wallets.
            // Its sync workers keep writing them until its native teardown
            // ends, and `deinit` only schedules that teardown for whenever
            // the last reference is released. Configuring first leaves it
            // usable if `configure` throws: creating a native manager reads
            // and writes no persisted state and starts no sync.
            if stale === activeManager {
                // Shutting it down stops the active manager's SPV, so a
                // start suspended in peer resolution must bail instead of
                // resuming against the torn-down manager.
                invalidatePendingSpvStarts()
            }
            await stale.shutdown()
        }
        restoreWallets(into: manager, network: network)
        managers[network] = manager
        managerSdks[network] = sdk
        if becomesActive {
            // A freshly (re)built active manager supersedes any in-flight
            // SPV start against the previous one (defensive backstop; the
            // supersession call sites also invalidate at their earliest
            // synchronous point).
            invalidatePendingSpvStarts()
            activeManager = manager
        }
        SDKLogger.event(
            "manager_materialized",
            category: .lifecycle,
            fields: [
                "active": .boolean(becomesActive),
                "network": .publicText(network.displayName),
                "wallet_count": .integer(Int64(manager.wallets.count)),
            ]
        )
    }

    /// Best-effort: a fresh manager comes up with no wallets in
    /// memory; restore from the network-scoped persistence
    /// handler so reopening the app on this network resurfaces
    /// the wallets the user already created here. Failures here
    /// are non-fatal — the user can still create / import
    /// wallets, the in-memory set just starts empty.
    ///
    /// Synchronous so the restore runs on the calling thread right before
    /// `activate` caches and swaps the manager: called from `activate`'s
    /// async body, `loadFromPersistor()` would resolve to its off-main
    /// overload and add a suspension there.
    private func restoreWallets(into manager: PlatformWalletManager, network: Network) {
        do {
            _ = try manager.loadFromPersistor()
        } catch {
            SDKLogger.event(
                "manager_restore_failed",
                category: .lifecycle,
                severity: .error,
                fields: ["network": .publicText(network.displayName)],
                error: error
            )
        }
    }

    /// Manager for `network` if one has been activated this session;
    /// otherwise `nil`. Used for diagnostics surfaces (e.g. Wallet
    /// Memory Explorer) that want to inspect a specific network's
    /// state without forcing it active.
    func manager(for network: Network) -> PlatformWalletManager? {
        managers[network]
    }

    /// Get-or-build the manager for `network` without changing the
    /// currently active one. Builds a fresh `SDK` for that network
    /// on demand the first time it's requested.
    ///
    /// Used by orphan-mnemonic recovery, which needs to route each
    /// rebuilt wallet to the manager bound to the network the
    /// wallet was originally created on — otherwise wallets bound
    /// to non-active networks at creation time get re-derived and
    /// persisted under the active network's manager (the Rust
    /// manager stamps `wallet.network = self.sdk.network` at
    /// registration time, so a regtest mnemonic recovered through
    /// the testnet manager lands as a testnet row).
    func backgroundManager(for network: Network) async throws -> PlatformWalletManager {
        if let existing = managers[network] {
            return existing
        }
        let sdk = try SDK(network: network, contextProvider: .adaptive)
        try await activate(network: network, sdk: sdk, makeActive: false)
        guard let manager = managers[network] else {
            throw PlatformWalletError.invalidParameter(
                "Failed to materialize manager for \(network.displayName)"
            )
        }
        return manager
    }
}
