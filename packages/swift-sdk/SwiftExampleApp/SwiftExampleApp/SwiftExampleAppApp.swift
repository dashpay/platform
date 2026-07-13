//
//  SwiftExampleAppApp.swift
//  SwiftExampleApp
//
//  Created by Sam Westrich on 8/6/25.
//

import SwiftUI
import SwiftData

// Pull in logging helpers for SDKLogger and presets
import SwiftDashSDK

/// UI-only state that used to live on UnifiedAppState. Small bag of flags that
/// coordinate between views.
@MainActor
final class AppUIState: ObservableObject {
    /// Whether the detailed sync banner should be shown on the Wallets tab.
    @Published var showWalletsSyncDetails: Bool = true

    /// Root tab selection. Lives here (not as ContentView @State) so views
    /// deep inside other tabs' navigation stacks can deep-link — e.g.
    /// IdentityDetailView's "Contacts" row jumps to the DashPay tab with
    /// that identity pre-selected.
    @Published var selectedTab: RootTab = .sync
}

@main
struct SwiftExampleAppApp: App {
    // SwiftData container — shared across services and views.
    private let modelContainer: ModelContainer

    // Platform identity / document / contract state.
    @StateObject private var platformState = AppState()

    // Per-network wallet managers. The Rust `PlatformWalletManager`
    // is network-locked at `configure(...)` time, so swapping
    // networks at runtime needs a different manager instance —
    // not a reconfigured one. The store lazy-creates a manager per
    // network and republishes the active one so the SwiftUI
    // environment rebinds to the right instance on switch.
    @StateObject private var walletManagerStore: WalletManagerStore

    // Remaining services.
    @StateObject private var shieldedService = ShieldedService()
    @StateObject private var platformBalanceSyncService = PlatformBalanceSyncService()
    @StateObject private var transitionState = TransitionState()
    @StateObject private var appUIState = AppUIState()

    /// Current manager exposed to views via the env object pipeline.
    /// Reads from the published `activeManager` on every body
    /// invocation so the env object rebinds whenever
    /// `walletManagerStore.activate(...)` swaps the active manager.
    private var walletManager: PlatformWalletManager {
        walletManagerStore.activeManager
    }

    @State private var isInitialized = false
    @State private var bootstrapError: Error?
    @State private var bootstrapTask: Task<Void, Never>?

    /// Guards the launch-time Core SPV auto-start so it fires at most
    /// once per process, even if `bootstrap()` re-runs via
    /// `retryBootstrap`.
    @State private var didAutoStartCoreSpv = false

    /// Resolver that backs the platform-wallet-ffi `MnemonicResolverHandle`
    /// for shielded wallet binding. Reuses the default `WalletStorage`
    /// keychain access — same shape as the identity-key signing path.
    /// Held for the lifetime of the App so the underlying handle is
    /// valid across every `bind_shielded` call.
    private let shieldedResolver = MnemonicResolver()

    init() {
        // Suppress auto layout constraint warnings in debug builds
        // These are typically harmless keyboard-related warnings
        #if DEBUG
        UserDefaults.standard.set(false, forKey: "_UIConstraintBasedLayoutLogUnsatisfiable")
        #endif

        // Wipe legacy keychain residue before any SDK component gets
        // a chance to read under a stale service name. Covers the
        // old `org.dash.wallet` / `com.dash.sdk.keys` /
        // `com.dash.swiftexampleapp.keys` services (now consolidated
        // under `org.dashfoundation.wallet`) and the pre-per-wallet
        // `wallet.seed` / `wallet.mnemonic` / `wallet.pin` accounts.
        // Best-effort — failures are silently ignored inside
        // `cleanupLegacyItems` itself.
        WalletStorage.cleanupLegacyItems()

        let container: ModelContainer
        do {
            container = try DashModelContainer.create()
        } catch {
            fatalError("Failed to create ModelContainer: \(error)")
        }
        self.modelContainer = container
        // Build the store eagerly so the autoclosure for
        // `@StateObject` can capture the local `container`
        // directly — referencing `self.modelContainer` here would
        // make the autoclosure capture `self` mutating, which the
        // compiler rejects. Same `container` is handed to every
        // per-network manager the store lazy-creates.
        _walletManagerStore = StateObject(
            wrappedValue: WalletManagerStore(modelContainer: container)
        )
    }

    var body: some Scene {
        WindowGroup {
            ContentView(isInitialized: isInitialized, bootstrapError: bootstrapError, onRetry: retryBootstrap)
                .environmentObject(platformState)
                // Re-injected on every body invocation. When the
                // store swaps `activeManager` (network switch), the
                // computed `walletManager` returns the new instance
                // and SwiftUI rebinds the env object — the 40+
                // `@EnvironmentObject var walletManager:
                // PlatformWalletManager` consumers see the right
                // network's manager without any view changes.
                .environmentObject(walletManager)
                // Inject the store itself so flows that need to
                // operate on a non-active network's manager
                // (orphan-mnemonic recovery — wallets restored
                // from keychain may belong to networks the user
                // isn't currently looking at) can route through
                // `backgroundManager(for:)` without flipping the
                // user's active view.
                .environmentObject(walletManagerStore)
                .environmentObject(shieldedService)
                .environmentObject(platformBalanceSyncService)
                .environmentObject(transitionState)
                .environmentObject(appUIState)
                .environment(\.modelContext, modelContainer.mainContext)
                .task {
                    SDKLogger.log("🚀 SwiftExampleApp: Starting initialization...", minimumLevel: .medium)
                    await bootstrap()
                    SDKLogger.log("🚀 SwiftExampleApp: Initialization complete", minimumLevel: .medium)
                }
                // Rebind wallet-scoped services whenever the set of
                // managed wallets changes — wallet creation, restore
                // on launch, or a full wipe. The Rust manager holds
                // N wallets concurrently; BLAST sync is started once
                // at manager scope and iterates all of them.
                //
                // Keyed on the sorted id set so unrelated republishes
                // of the same dictionary don't retrigger.
                .onChange(of: walletManager.wallets.keys.sorted(by: {
                    $0.lexicographicallyPrecedes($1)
                })) { _, _ in
                    rebindWalletScopedServices()
                }
                // Network switch: activate the per-network manager
                // first (the store lazy-creates one configured with
                // a fresh SDK if this is the first time we see this
                // network), then rebind the wallet-scoped services
                // against it. Order matters — `rebindWalletScopedServices`
                // reads `walletManager.firstWallet`, which has to
                // resolve to the new network's manager before it
                // runs.
                .onChange(of: platformState.currentNetwork) { _, newNetwork in
                    activateManager(for: newNetwork)
                    rebindWalletScopedServices()
                }
                // Devnet→devnet rebuild from OptionsView: when the user
                // edits the quorum URL / devnet name the SDK is rebuilt
                // and `WalletManagerStore.activate` swaps the cached
                // `PlatformWalletManager`, but neither of the two
                // observers above fires (network stays `.devnet`;
                // wallet ID set stays identical after persistor reload).
                // PlatformBalanceSyncService and ShieldedService would
                // keep retaining the old manager. Listen for the explicit
                // tick OptionsView publishes after the activate completes.
                .onChange(of: platformState.walletScopedServicesRebindTick) { _, _ in
                    rebindWalletScopedServices()
                }
        }
    }

    /// Lazy-create + cache a `PlatformWalletManager` for `network`,
    /// configured against `platformState.sdk`. No-ops on the
    /// already-active network. Called from bootstrap and from
    /// `currentNetwork.onChange`.
    @MainActor
    private func activateManager(for network: Network) {
        guard let sdk = platformState.sdk else {
            SDKLogger.error(
                "Cannot activate wallet manager for \(network.displayName): "
                    + "no SDK available (still bootstrapping?)"
            )
            return
        }
        do {
            try walletManagerStore.activate(network: network, sdk: sdk)
        } catch {
            SDKLogger.error(
                "Failed to activate wallet manager for "
                    + "\(network.displayName): \(error.localizedDescription)"
            )
        }
    }

    /// Drive manager-wide BLAST sync state from the set of loaded
    /// wallets. With no wallets present on the active manager, sync
    /// is stopped and the per-wallet `PlatformBalanceSyncService`
    /// UI surface is reset — so the Sync Status tab shows zeros for
    /// a network the user hasn't created a wallet on yet, instead
    /// of leaking values from a wallet on a different network.
    /// Otherwise, we bind the balance service to a deterministic
    /// wallet on the active manager. (Detail views reconfigure the
    /// service per-wallet themselves.)
    ///
    /// The active manager is per-network now, so its `firstWallet`
    /// is already correctly scoped — no need for a separate
    /// network-filtering pass at this layer.
    @MainActor
    private func rebindWalletScopedServices() {
        let wallet = walletManager.firstWallet
        guard let wallet else {
            do {
                try walletManager.stopPlatformAddressSync()
                try walletManager.stopShieldedSync()
                try walletManager.stopDashPaySync()
            } catch {
                SDKLogger.error(
                    "Failed to stop sync coordinators: \(error.localizedDescription)"
                )
            }
            platformBalanceSyncService.reset()
            shieldedService.reset()
            return
        }
        do {
            let platformAddressWallet = try wallet.platformAddressWallet()
            platformBalanceSyncService.configure(
                platformAddressWallet: platformAddressWallet,
                walletManager: walletManager,
                persistenceHandler: walletManager.persistence,
                walletId: wallet.walletId
            )
            if try !walletManager.isPlatformAddressSyncRunning() {
                try walletManager.startPlatformAddressSync()
            }
            SDKLogger.log(
                "🔗 BLAST sync running; balance-sync UI bound to wallet \(wallet.walletId.prefix(4).map { String(format: "%02x", $0) }.joined())… on \(platformState.currentNetwork.displayName) (of \(walletManager.wallets.count) loaded)",
                minimumLevel: .medium
            )

            // Bind the shielded service against the same wallet.
            // The bind is best-effort — failures (no mnemonic in
            // keychain, biometric prompt declined, etc.) leave the
            // service in a "not bound" state and the user can
            // retry from the Sync Status surface.
            shieldedService.bind(
                walletManager: walletManager,
                walletId: wallet.walletId,
                network: platformState.currentNetwork,
                resolver: shieldedResolver
            )

            // Engine-bind every OTHER loaded wallet into the shared
            // network-scoped shielded coordinator. `firstWallet` above
            // already drives the UI mirror AND its own engine
            // registration via `bind(...)`; this loop registers the
            // remaining wallets so a single shielded sync pass
            // trial-decrypts against the union of every wallet's viewing
            // keys (SH-14/15/16 cross-wallet flows). Each bind is
            // best-effort + independent — one wallet's missing mnemonic
            // must not block the others. Reading each mnemonic is a
            // device-unlock-only keychain read (no biometric prompt), so
            // eager binding at startup is safe. The iteration seam is a
            // pure free function (`engineBindOtherWallets`) so its
            // "visit every non-mirror wallet" contract can be
            // unit-tested without a configured manager.
            //
            // Runs BEFORE the shielded/DashPay start calls below:
            // engine-binding must not depend on those fallible calls — a
            // throw there (e.g. `startShieldedSync` failing) must not
            // leave the non-mirror wallets unbound for the rest of the
            // session.
            engineBindOtherWallets(
                allWalletIds: walletManager.wallets.keys,
                mirrorWalletId: wallet.walletId
            ) { otherWalletId in
                shieldedService.bindEngine(
                    walletManager: walletManager,
                    walletId: otherWalletId,
                    network: platformState.currentNetwork,
                    resolver: shieldedResolver
                )
            }

            if try !walletManager.isShieldedSyncRunning() {
                try walletManager.startShieldedSync()
            }

            // DashPay contact-request + profile sweep (background
            // loop). Wallet-driven — every registered wallet is swept
            // each pass — so manager scope is the right place to start
            // it, same as the address / shielded loops above.
            // Idempotent: starting while running is a no-op.
            if try !walletManager.isDashPaySyncRunning() {
                try walletManager.startDashPaySync()
            }
        } catch {
            SDKLogger.error(
                "Failed to bind wallet-scoped services: \(error.localizedDescription)"
            )
        }
    }

    /// Auto-start Core SPV sync for the launch network. The gate matrix
    /// (once-per-launch latch / wallet-gated / no-double-start) lives in
    /// the pure `CoreSpvAutoStart.decision` so it can be unit-tested; see
    /// its doc for why a no-wallets launch doesn't latch but an
    /// already-running client does.
    ///
    /// `CoreSpvLauncher.start` is `async` (it resolves peers off the main
    /// actor), so we **latch before the first `await`**: the latch write
    /// happens synchronously on the main actor, closing the window where
    /// a concurrent `bootstrap` (e.g. `retryBootstrap` overlapping the
    /// original `.task`) could pass the gate during the off-main peer
    /// resolution and double-start.
    ///
    /// Best-effort: a start failure is logged, never fatal — it must not
    /// propagate into `bootstrap`'s catch and trip the error UI.
    @MainActor
    private func autoStartCoreSpvIfNeeded() async {
        let network = platformState.currentNetwork
        let manager = walletManager
        let store = walletManagerStore
        let decision = CoreSpvAutoStart.decision(
            alreadyLatched: didAutoStartCoreSpv,
            hasWallets: !manager.wallets.isEmpty,
            spvRunning: manager.spvIsRunning
        )
        if decision.shouldLatch { didAutoStartCoreSpv = true }
        guard decision.shouldStart else { return }

        do {
            try await CoreSpvLauncher.start(
                network: network,
                on: manager,
                stillCurrent: { store.activeManager === manager }
            )
            SDKLogger.log(
                "🟢 Auto-started Core SPV sync for " + network.displayName,
                minimumLevel: .medium
            )
        } catch is CancellationError {
            // The user switched networks during the peer lookup, so the
            // launch network's auto-start window is over. The latch stays
            // set (set before the await) — we deliberately do NOT unlatch
            // and re-fire for a network the user has left; the now-active
            // network's `rebindWalletScopedServices` + its own auto-start
            // own its sync.
            SDKLogger.log(
                "ℹ️ Core SPV auto-start superseded by a network switch",
                minimumLevel: .medium
            )
        } catch {
            SDKLogger.error(
                "Auto-start Core SPV sync failed: \(error.localizedDescription)"
            )
        }
    }

    @MainActor
    private func bootstrap() async {
        do {
            LoggingPreferences.configure()

            // Kick off Halo 2 proving-key build on a background
            // thread so the first shielded send doesn't pay the
            // ~30 s build cost inline. Idempotent — global
            // OnceLock on the Rust side guards repeat calls.
            Task.detached(priority: .background) {
                await PlatformWalletManager.warmUpShieldedProver()
            }

            platformState.initializeSDK(modelContext: modelContainer.mainContext)

            // Give the Platform SDK a moment to finish its internal init.
            try? await Task.sleep(for: .milliseconds(500))

            if let sdk = platformState.sdk {
                // Activate the per-network manager for the launch
                // network. The store creates + configures the
                // manager and runs `loadFromPersistor` against it
                // (filtered to the launch network's wallets via
                // the network-aware persistence handler), so no
                // separate restore pass is needed here.
                try walletManagerStore.activate(
                    network: platformState.currentNetwork,
                    sdk: sdk
                )
                let restoredCount = walletManager.wallets.count
                if restoredCount > 0 {
                    SDKLogger.log(
                        "🔓 Restored \(restoredCount) wallet(s) from persister "
                            + "for \(platformState.currentNetwork.displayName)",
                        minimumLevel: .medium
                    )
                }

                // Pre-warm per-network managers for any orphan
                // mnemonic whose original network differs from
                // the active one, so the orphan-recovery flow
                // doesn't have to lazy-build them mid-session.
                // SwiftData's @Query observers in the main
                // context don't always reflect rows persisted
                // through a `backgroundContext` that was
                // created mid-session — pre-warming here means
                // those backgrounds are wired up alongside the
                // main context at launch and the recovered
                // wallet appears in its correct tab on the same
                // run instead of only after a relaunch.
                preWarmOrphanNetworkManagers()

                rebindWalletScopedServices()

                // Kick off Core SPV sync for the launch network so the
                // user doesn't have to tap Start on the Sync tab.
                await autoStartCoreSpvIfNeeded()
            }

            isInitialized = true
        } catch {
            bootstrapError = error
        }
    }

    @MainActor
    private func retryBootstrap() {
        bootstrapError = nil
        bootstrapTask?.cancel()
        bootstrapTask = Task {
            await bootstrap()
        }
    }

    // MARK: - Helpers

    /// Read local Core peers from UserDefaults (comma-separated addresses).
    private func readLocalCorePeers() -> [String] {
        if let csv = UserDefaults.standard.string(forKey: "localCorePeers"), !csv.isEmpty {
            return csv.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        }
        return ["127.0.0.1"]
    }

    /// Materialize a `PlatformWalletManager` for every network that
    /// has an orphan keychain mnemonic, except the already-active
    /// one. Used during bootstrap so the orphan-recovery flow has
    /// pre-warmed managers when the user authorizes recovery —
    /// avoids a SwiftData edge case where a mid-session-created
    /// background `ModelContext` doesn't propagate writes back to
    /// the launch-time main context's `@Query` observers.
    @MainActor
    private func preWarmOrphanNetworkManagers() {
        let storage = WalletStorage()
        let keychainIds = (try? storage.listWalletIdsWithMnemonic()) ?? []
        guard !keychainIds.isEmpty else { return }

        var orphanNetworks: Set<Network> = []
        for walletId in keychainIds {
            guard let metadata = (try? storage.metadata(for: walletId)) ?? nil,
                  let resolved = metadata.resolvedNetworks.first
            else { continue }
            orphanNetworks.insert(resolved)
        }

        let active = platformState.currentNetwork
        for network in orphanNetworks where network != active {
            do {
                _ = try walletManagerStore.backgroundManager(for: network)
                SDKLogger.log(
                    "🔥 Pre-warmed wallet manager for \(network.displayName) "
                        + "(orphan recovery target)",
                    minimumLevel: .medium
                )
            } catch {
                SDKLogger.error(
                    "Failed to pre-warm \(network.displayName) manager: "
                        + error.localizedDescription
                )
            }
        }
    }
}
