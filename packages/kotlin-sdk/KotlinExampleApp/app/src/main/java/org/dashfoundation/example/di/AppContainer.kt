package org.dashfoundation.example.di

import android.content.Context
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.example.state.AppState
import org.dashfoundation.example.state.AppUiState

private val Context.preferencesStore by preferencesDataStore(name = "example_prefs")

/**
 * Manual dependency container — the literal translation of the seven
 * `@StateObject` + `.environmentObject(...)` injections in
 * `SwiftExampleAppApp.swift`. No DI framework: constructor injection keeps
 * the example readable for SDK adopters.
 *
 * Lifetime: one per process, owned by [org.dashfoundation.example.ExampleApplication].
 */
class AppContainer(private val context: Context) {

    val applicationScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    val database: DashDatabase = DashDatabase.create(context)

    val dataStore = context.preferencesStore

    val appState = AppState(dataStore, applicationScope)

    val appUiState = AppUiState()

    /** Fast-cadence SPV progress feed for the global overlay (A-M4 wires the source). */
    val syncOverlayState = org.dashfoundation.example.state.SyncOverlayState()

    /** Keystore-wrapped secret store (mnemonics + identity keys). */
    val walletStorage = org.dashfoundation.dashsdk.security.WalletStorage(context)

    /**
     * Device-local DashPay contact metadata (alias / note / hidden / DPNS
     * hint). A single shared instance so a write's `version` bump invalidates
     * every DashPay screen that reads through it — a per-screen instance would
     * only recompose its own creator.
     */
    val dashPayContactMetaStore by lazy {
        org.dashfoundation.example.ui.dashpay.DashPayContactMetaStore(context)
    }

    /**
     * Auth gate for secret reveals / out-of-window key access. The
     * container is Application-scoped, so the gate is a delegating shell;
     * MainActivity binds the real Activity-bound `AuthPrompt` on create
     * (the B-M2 injection point). `KeystoreSigner` receives the same
     * instance via [walletManagerStore]'s manager factory.
     */
    val biometricGate = org.dashfoundation.example.services.auth.DelegatingBiometricGate()

    /**
     * Per-network wallet manager cache — port of `WalletManagerStore.swift`.
     * Activated during [bootstrap] and on every network switch (the AppRoot
     * observer).
     */
    val walletManagerStore = org.dashfoundation.dashsdk.wallet.WalletManagerStore(
        database = database,
        walletStorage = walletStorage,
        biometricGate = biometricGate,
    )

    /**
     * In-flight identity registrations — port of the
     * `RegistrationCoordinator` iOS hosts on `PlatformWalletManager`. Held
     * here (Application-scoped) because a single active-network manager is in
     * play at a time via [walletManagerStore]; a network switch tears down
     * the manager and any registrations belonged to the prior network
     * anyway. Its retention sweep runs on [applicationScope].
     */
    val registrationCoordinator =
        org.dashfoundation.example.services.RegistrationCoordinator(applicationScope)

    /**
     * In-flight "fund a Platform address from an asset lock" attempts —
     * port of the `AddressFundFromAssetLockCoordinator` iOS hosts on
     * `PlatformWalletManager`. App-scoped for the same reasons as
     * [registrationCoordinator]; its retention sweep runs on
     * [applicationScope].
     */
    val addressFundCoordinator =
        org.dashfoundation.example.services.assetlock.AddressFundFromAssetLockCoordinator(
            applicationScope,
        )

    /**
     * In-flight "shield funds from an asset lock" attempts — port of the
     * `ShieldedFundFromAssetLockCoordinator`. Enforces per-wallet
     * serialization on top of per-slot single-flight.
     */
    val shieldedFundCoordinator =
        org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator(
            applicationScope,
        )

    /**
     * Ephemeral pricing / purchase-eligibility for the state-transition
     * catalog — port of `TransitionState.swift`. App-scoped (the Swift
     * `@StateObject` lives on the app) so it survives category ↔ detail
     * navigation within a single build session.
     */
    val transitionState = org.dashfoundation.example.state.TransitionState()

    /** BLAST platform-address sync state reflected to the UI (A-M4). */
    val platformBalanceSyncService =
        org.dashfoundation.dashsdk.services.PlatformBalanceSyncService(database)

    /** Shielded (Orchard) sync state reflected to the UI (A-M4). */
    val shieldedService = org.dashfoundation.dashsdk.services.ShieldedService(database)

    private var spvOverlayJob: kotlinx.coroutines.Job? = null
    private var rebindJob: kotlinx.coroutines.Job? = null

    /**
     * Activate the manager for the current SDK/network, load wallets, and
     * rebind the wallet-scoped services — port of
     * `SwiftExampleAppApp.rebindWalletScopedServices`.
     */
    suspend fun activateManager() {
        val sdk = appState.sdk.value ?: return
        val manager = walletManagerStore.activate(sdk.network, sdk)
        manager.loadPersistedWallets()
        platformBalanceSyncService.configure(manager)

        // Bind the wallet-scoped services against the deterministic first
        // wallet now, and re-bind whenever the wallet set changes (create /
        // import / remove) so a wallet created mid-session binds and starts
        // syncing without an app relaunch. `drop(1)` skips the current set,
        // already bound by the direct call above.
        rebindWalletScopedServices(manager)
        rebindJob?.cancel()
        rebindJob = applicationScope.launch {
            manager.wallets
                .map { it.keys }
                .distinctUntilChanged()
                .drop(1)
                .collect { rebindWalletScopedServices(manager) }
        }

        // Adapt the SDK's SPV progress into the app's overlay feed. Only
        // the leaf overlay composable collects it (ContentView.swift
        // leaf-isolation note).
        val publisher = org.dashfoundation.dashsdk.services.SpvProgressPublisher(manager)
        spvOverlayJob?.cancel()
        spvOverlayJob = applicationScope.launch {
            publisher.progress.collect { p ->
                syncOverlayState.publish(
                    org.dashfoundation.example.state.SpvProgress(
                        phaseTitle = p.phaseTitle,
                        overallPercentage = p.overallPercentage,
                        isSyncing = p.isSyncing,
                    ).takeIf { p.isSyncing },
                )
            }
        }
    }

    /**
     * Wallet-scoped rebind — drive the manager-wide sync loops from the
     * deterministic first wallet (← iOS `rebindWalletScopedServices` in
     * SwiftExampleAppApp.swift). Idempotent: `configureShielded` /
     * `bindShielded` and the start-if-not-running guards make repeated calls
     * (on every wallet-set change) safe.
     */
    private suspend fun rebindWalletScopedServices(
        manager: org.dashfoundation.dashsdk.wallet.PlatformWalletManager,
    ) {
        // Mirrors iOS `PlatformWalletManager.firstWallet` — the
        // lexicographically smallest wallet id; the map keys are fixed-width
        // lowercase hex, so string order equals byte order.
        val wallet = manager.wallets.value.entries.minByOrNull { it.key }?.value
        if (wallet == null) {
            // No wallets on the active network: stop the loops and reset the
            // per-wallet service surfaces, so the Sync tab shows zeros instead
            // of leaking values from another network's wallet.
            try {
                manager.stopPlatformAddressSync()
                if (shieldedService.isAvailable) {
                    manager.stopShieldedSync()
                }
                manager.stopDashPaySync()
            } catch (e: Exception) {
                android.util.Log.w(TAG, "Failed to stop sync coordinators", e)
            }
            platformBalanceSyncService.reset()
            shieldedService.unbind()
        } else {
            try {
                // Re-attach the balance-sync UI reflector: a prior no-wallet
                // pass reset() it (nulling its manager), and without this the
                // Sync tab stays "Not synced yet" with Sync Now a no-op until
                // an app restart.
                platformBalanceSyncService.configure(manager)
                if (!manager.isPlatformAddressSyncRunning()) {
                    manager.startPlatformAddressSync()
                }
                // Best-effort like iOS — failures (no mnemonic in the store,
                // declined biometric) leave the service unbound; bind() logs
                // and doesn't throw. dbPath mirrors iOS's
                // `ShieldedService.dbPath(for:)` naming
                // (`shielded_tree_<networkName>.sqlite`), rooted in filesDir.
                shieldedService.bind(
                    manager = manager,
                    walletId = wallet.walletId,
                    dbPath = java.io.File(
                        context.filesDir,
                        "shielded_tree_${manager.network.networkName}.sqlite",
                    ).absolutePath,
                )
                if (shieldedService.isAvailable && !manager.isShieldedSyncRunning()) {
                    manager.startShieldedSync()
                }
                // DashPay contact/profile/payment sync — the load-bearing
                // prerequisite for the DashPay tab (← iOS
                // rebindWalletScopedServices starting it alongside the
                // address/shielded loops on the wallet-present branch).
                if (!manager.isDashPaySyncRunning()) {
                    manager.startDashPaySync()
                }
            } catch (e: Exception) {
                android.util.Log.e(TAG, "Failed to bind wallet-scoped services", e)
            }
        }
    }

    /** Bootstrap phase — drives the AppRoot gate (← ContentView isInitialized). */
    sealed interface BootstrapState {
        data object Loading : BootstrapState
        data object Ready : BootstrapState
        data class Failed(val error: Throwable) : BootstrapState
    }

    private val _bootstrapState = MutableStateFlow<BootstrapState>(BootstrapState.Loading)
    val bootstrapState: StateFlow<BootstrapState> = _bootstrapState.asStateFlow()

    /**
     * App bootstrap — port of `SwiftExampleAppApp.bootstrap()`:
     * one-time SDK init + logging, restore persisted network, build the
     * per-network SDK. (Wallet-manager activation and sync-service binding
     * attach here as those layers land — same ordering as iOS.)
     */
    suspend fun bootstrap() {
        _bootstrapState.value = BootstrapState.Loading
        try {
            Sdk.initialize()
            Sdk.enableLogging(Sdk.LogLevel.DEBUG)
            appState.restorePreferences()
            appState.initializeSdk()
            activateManager()
            _bootstrapState.value = BootstrapState.Ready
        } catch (e: Exception) {
            _bootstrapState.value = BootstrapState.Failed(e)
        }
    }

    private companion object {
        const val TAG = "AppContainer"
    }
}
