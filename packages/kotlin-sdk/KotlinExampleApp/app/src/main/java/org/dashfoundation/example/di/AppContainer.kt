package org.dashfoundation.example.di

import android.content.Context
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
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
class AppContainer(context: Context) {

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
}
