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
    /// enough because of the stop→activate gap: an SDK rebuild stops the
    /// old manager, then awaits the SDK build, and the store swaps
    /// `activeManager` only after that. Each such path invalidates this
    /// token before it stops SPV (`stopSpvForReconfiguration`), so a start
    /// resuming after the stop bails. See the guard in
    /// `CoreSpvLauncher.start`.
    ///
    /// Not `@Published` — nothing renders off it; it's read
    /// synchronously by start call sites and their revalidation
    /// closures only.
    private(set) var spvStartGeneration: UInt64 = 0

    /// Invalidate every in-flight SPV start.
    ///
    /// Invariant: **every user-visible or programmatic stop / replace of
    /// the active manager's SPV must bump the generation first.** Call
    /// this before any such path stops SPV (network switch, devnet/SDK
    /// rebuild, peer-config change, a Stop/Pause button) so a start
    /// suspended in the stop→activate gap bails instead of reviving the
    /// superseded manager. User-facing stop buttons route through
    /// `stopSpvCancellingPendingStarts(_:)` and reconfigurations through
    /// `stopSpvForReconfiguration()`, so they can't forget.
    func invalidatePendingSpvStarts() {
        spvStartGeneration &+= 1
    }

    /// Stop the active manager's SPV before an SDK rebuild or a peer change
    /// reconfigures it, after cancelling any in-flight
    /// `CoreSpvLauncher.start`, and wait for the stop. It goes through the
    /// SDK's async `stopSpv()`, which queues behind a start the SDK already
    /// admitted and stops the client that start brings up.
    func stopSpvForReconfiguration() async throws {
        invalidatePendingSpvStarts()
        try await activeManager.stopSpv()
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

    /// Networks with an `activate` call still running. The store keeps its
    /// own invariant, one activation per network at a time, instead of
    /// relying on every caller being serialized: `AppState.rebuildSdk` runs
    /// one SDK rebuild at a time, which is the rebuild's invariant, not the
    /// store's. A replacement suspends while the manager it replaces shuts
    /// down, and a second call in that window would replace the same cached
    /// manager and publish over the first one's without shutting it down.
    private var activatingNetworks: Set<Network> = []

    /// Thrown by `activate` while another activation for the same network is
    /// still running. The refused call configured and published nothing.
    struct ActivationInProgress: LocalizedError {
        let network: Network

        var errorDescription: String? {
            "The \(network.displayName) wallet manager is already being replaced, so this change was not applied."
        }
    }

    /// SwiftData container shared across every manager. Each
    /// manager's persistence handler narrows its `loadWalletList`
    /// fetch to its own network so the shared store doesn't cause
    /// cross-network bleed at restore time.
    private let modelContainer: ModelContainer

    /// The app's SDK builder (`AppState.makeSdk`), which applies the user's
    /// quorum mode. `backgroundManager(for:)` builds its SDKs with it.
    private let makeSdk: @MainActor (Network) throws -> SDK

    /// Why opening each network's shielded store failed, until a later
    /// attempt succeeds. The DashPay views show it while the wallet's
    /// private payments wait for that store, and retry it
    /// (`configureShieldedAgain(_:)`).
    @Published private(set) var shieldedStoreFailures: [Network: String] = [:]

    init(modelContainer: ModelContainer, makeSdk: @escaping @MainActor (Network) throws -> SDK) {
        self.modelContainer = modelContainer
        self.makeSdk = makeSdk
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
    /// with `invalidHandle`.
    ///
    /// A call for a network whose activation is still running throws
    /// `ActivationInProgress` before it configures anything or suspends:
    /// both would replace the same cached manager, and the later one would
    /// publish over the earlier one without shutting it down. The store
    /// refuses it whatever its callers do; SDK rebuilds, serialized by
    /// `AppState.rebuildSdk`, do not reach the refusal.
    ///
    /// `makeActive == false` materializes the manager into the cache
    /// without swapping `activeManager`, unless the manager it replaces is
    /// the active one. Used by background flows (e.g. orphan recovery
    /// routing wallets to their original network) that need a manager for
    /// a non-active network without triggering a user-visible network
    /// switch.
    func activate(network: Network, sdk: SDK, makeActive: Bool = true) async throws {
        guard activatingNetworks.insert(network).inserted else {
            throw ActivationInProgress(network: network)
        }
        defer { activatingNetworks.remove(network) }
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
                // A shielded store that did not open last time is tried
                // again: until it opens, contact requests stay held (72).
                if shieldedStoreFailures[network] != nil {
                    configureShieldedStore(existing, network: network)
                }
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
        // Open the shielded store before any wallet is loaded or created:
        // this app's persister keeps the private contact request scan state,
        // so Rust holds every wallet published while no store is configured
        // (code 72) until its private payment journal is read, and a later
        // bind may never come (no mnemonic, shielded off). With the store
        // open first, each wallet's journal is read as it is published, and
        // the binds' own calls on the same path are no-ops. A failure is
        // recorded and retried; the wallets stay held meanwhile. It opens
        // after the replaced manager's shutdown, which releases the same
        // store path.
        configureShieldedStore(manager, network: network)
        restoreWallets(into: manager, network: network)
        managers[network] = manager
        managerSdks[network] = sdk
        if becomesActive {
            // A freshly (re)built active manager supersedes any in-flight
            // SPV start against the previous one (defensive backstop; a
            // rebuild also invalidates before it stops SPV, in
            // `stopSpvForReconfiguration`).
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

    /// Open `network`'s shielded store on `manager`, recording why it failed
    /// for the DashPay views, or clearing that once it opens. A repeat on the
    /// same path does not wait on the store, so it is safe on the main
    /// thread.
    private func configureShieldedStore(_ manager: PlatformWalletManager, network: Network) {
        do {
            try manager.configureShielded(dbPath: ShieldedService.dbPath(for: network))
            shieldedStoreFailures[network] = nil
        } catch {
            shieldedStoreFailures[network] = error.localizedDescription
            SDKLogger.event(
                "manager_shielded_configure_failed",
                category: .lifecycle,
                severity: .error,
                fields: ["network": .publicText(network.displayName)],
                error: error
            )
        }
    }

    /// Why opening the shielded store of `manager`'s network failed last, or
    /// nil when it opened (or `manager` is not one of this store's).
    func shieldedStoreFailure(of manager: PlatformWalletManager) -> String? {
        guard let network = managers.first(where: { $0.value === manager })?.key else { return nil }
        return shieldedStoreFailures[network]
    }

    /// Open `manager`'s shielded store again and return why it failed, or
    /// nil once it is open. On a store already open this asks the wallet to
    /// retry restoring every wallet whose private payments are still held,
    /// a restoration that failed after the store opened included; it never
    /// waits on a busy store (a busy store defers the restoration to a
    /// task), so it is safe on the main thread.
    func configureShieldedAgain(_ manager: PlatformWalletManager) -> String? {
        guard let network = managers.first(where: { $0.value === manager })?.key else { return nil }
        configureShieldedStore(manager, network: network)
        return shieldedStoreFailures[network]
    }

    /// Manager for `network` if one has been activated this session;
    /// otherwise `nil`. Used for diagnostics surfaces (e.g. Wallet
    /// Memory Explorer) that want to inspect a specific network's
    /// state without forcing it active.
    func manager(for network: Network) -> PlatformWalletManager? {
        managers[network]
    }

    /// Get-or-build the manager for `network` without changing the
    /// currently active one. Builds an `SDK` for that network with the
    /// app's builder the first time it's requested, so it verifies proofs
    /// with the quorum mode the user picked. In SPV mode its proven queries
    /// fail closed, with no trusted keys and no node banned, unless that
    /// network's manager runs SPV.
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
        let sdk = try makeSdk(network)
        try await activate(network: network, sdk: sdk, makeActive: false)
        guard let manager = managers[network] else {
            throw PlatformWalletError.invalidParameter(
                "Failed to materialize manager for \(network.displayName)"
            )
        }
        return manager
    }
}
