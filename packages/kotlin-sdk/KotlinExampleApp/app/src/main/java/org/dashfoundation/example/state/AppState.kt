package org.dashfoundation.example.state

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig

/**
 * Platform-side app state — port of `AppState.swift`.
 *
 * Owns the [Sdk] instance and the current network selection (persisted to
 * DataStore, like the Swift `didSet` → UserDefaults mirror). Network
 * switching rebuilds the SDK (and, via the AppRoot observer, the per-network
 * wallet manager) — never reconfigures in place.
 */
class AppState(
    private val dataStore: DataStore<Preferences>,
    private val scope: CoroutineScope,
) {
    private val _sdk = MutableStateFlow<Sdk?>(null)
    val sdk: StateFlow<Sdk?> = _sdk.asStateFlow()

    private val _currentNetwork = MutableStateFlow(Network.DEFAULT)
    val currentNetwork: StateFlow<Network> = _currentNetwork.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage.asStateFlow()

    /**
     * The connected network's protocol version, learned via
     * `Sdk.system.refreshProtocolVersion()` after every SDK (re)build —
     * port of the iOS `AppState.platformProtocolVersion`. Null until the
     * refresh completes (or if it failed); consumers gating behavior on a
     * protocol version (e.g. the shielded denomination picker in
     * `CreateIdentityScreen`) should fall back to the currently-active
     * network behavior when null.
     */
    private val _platformProtocolVersion = MutableStateFlow<Int?>(null)
    val platformProtocolVersion: StateFlow<Int?> = _platformProtocolVersion.asStateFlow()

    private val _useDockerSetup = MutableStateFlow(false)
    val useDockerSetup: StateFlow<Boolean> = _useDockerSetup.asStateFlow()

    /**
     * "Use Custom SPV Peers" toggle — port of the iOS `useLocalhostCore`
     * UserDefaults key (OptionsView.swift; read by
     * `CoreContentView.spvPeerOverride`). When on, [localCorePeers]
     * overrides the network's built-in seed nodes for SPV.
     */
    private val _useLocalhostCore = MutableStateFlow(false)
    val useLocalhostCore: StateFlow<Boolean> = _useLocalhostCore.asStateFlow()

    /** Comma-separated `host:port` SPV peers — iOS `localCorePeers` key. */
    private val _localCorePeers = MutableStateFlow("")
    val localCorePeers: StateFlow<String> = _localCorePeers.asStateFlow()

    /**
     * Bumped when wallet-scoped services must rebind without a network
     * change (devnet SDK rebuild) — port of the Swift
     * `walletScopedServicesRebindTick`.
     */
    private val _rebindTick = MutableStateFlow(0)
    val walletScopedServicesRebindTick: StateFlow<Int> = _rebindTick.asStateFlow()

    /** Which quorum source proof verification uses — port of the Swift `AppState.QuorumMode`. */
    enum class QuorumMode(val rawValue: String, val label: String, val sdkMode: Sdk.QuorumMode) {
        /** SPV once its masternode list is synced, trusted before that and for any key SPV cannot serve. */
        AUTO("auto", "Auto", Sdk.QuorumMode.AUTO),

        /** SPV only; proofs fail closed whenever SPV is not synced. */
        SPV("spv", "SPV", Sdk.QuorumMode.SPV),

        /** The trusted HTTP quorum service only. */
        TRUSTED("trusted", "Trusted", Sdk.QuorumMode.TRUSTED);

        companion object {
            fun fromRawValue(rawValue: String): QuorumMode? =
                entries.firstOrNull { it.rawValue == rawValue }
        }
    }

    /**
     * The quorum source the user picked — port of the Swift
     * `AppState.quorumMode`, kept in DataStore under its UserDefaults key and
     * raw values. Applied to every SDK [initializeSdk] builds before it is
     * published, and to the current SDK by [setQuorumMode]. This app has one
     * SDK at a time, so that reaches every SDK it built.
     */
    private val _quorumMode = MutableStateFlow(QuorumMode.AUTO)
    val quorumMode: StateFlow<QuorumMode> = _quorumMode.asStateFlow()

    /**
     * The source the current SDK asks first for quorum keys — port of the
     * Swift `AppState.quorumSource`. Drives the settings indicator; it can
     * lag [quorumMode] while SPV is not synced.
     */
    private val _quorumSource = MutableStateFlow(Sdk.QuorumSource.TRUSTED)
    val quorumSource: StateFlow<Sdk.QuorumSource> = _quorumSource.asStateFlow()

    /**
     * Held while a pick is persisted and applied, and while [initializeSdk]
     * applies the mode to the SDK it publishes. Each holder reads the latest
     * pick, and the lock is granted in request order, so the last pick is
     * the one saved and applied, on whichever SDK is current.
     */
    private val quorumModeLock = Mutex()

    /** Restore persisted preferences; called once from bootstrap. */
    suspend fun restorePreferences() {
        val prefs = dataStore.data.first()
        prefs[KEY_NETWORK]?.let { name ->
            Network.fromNetworkName(name)?.let { _currentNetwork.value = it }
        }
        _useDockerSetup.value = prefs[KEY_DOCKER] ?: false
        _useLocalhostCore.value = prefs[KEY_USE_LOCAL_CORE] ?: false
        _localCorePeers.value = prefs[KEY_LOCAL_CORE_PEERS] ?: ""
        prefs[KEY_QUORUM_MODE]?.let { QuorumMode.fromRawValue(it) }?.let { _quorumMode.value = it }
    }

    /**
     * Build (or rebuild) the SDK for [network] with the given overrides —
     * port of `AppState.initializeSDK`. The old instance closes AFTER the
     * new one is published so observers never see a dead handle.
     */
    suspend fun initializeSdk(
        network: Network = _currentNetwork.value,
        dapiAddresses: String? = null,
        quorumUrl: String? = null,
    ) {
        _isLoading.value = true
        try {
            val newSdk = Sdk.create(
                SdkConfig(
                    network = network,
                    dapiAddresses = dapiAddresses,
                    quorumUrl = quorumUrl,
                    useDockerSetup = _useDockerSetup.value,
                ),
                Sdk.ContextProviderKind.ADAPTIVE,
            )
            // Apply the picked mode before anything can verify a proof with
            // this SDK (Swift `AppState.makeSdk`): built in Auto and switched
            // later, it would take trusted keys in between. A pick still
            // being applied finishes on the old SDK first.
            val old = quorumModeLock.withLock {
                try {
                    newSdk.setQuorumMode(_quorumMode.value.sdkMode)
                } catch (e: Exception) {
                    newSdk.closeSuspending()
                    throw e
                }
                _sdk.value.also { _sdk.value = newSdk }
            }
            refreshQuorumSource()
            // Eagerly learn the network's protocol version (iOS
            // `refreshProtocolVersion(for:)` parity): reset first so a
            // network switch never carries the previous network's
            // version, then refresh off the UI path. Non-fatal — the
            // SDK still ratchets from response metadata if this fails.
            _platformProtocolVersion.value = null
            scope.launch {
                try {
                    val version = newSdk.system.refreshProtocolVersion()
                    // Drop a stale result if the SDK was swapped again
                    // while the refresh was in flight.
                    if (version != null && _sdk.value === newSdk) {
                        _platformProtocolVersion.value = version
                    }
                } catch (e: Exception) {
                    android.util.Log.w(
                        "AppState",
                        "protocol version refresh failed (non-fatal): ${e.message}",
                    )
                }
            }
            // Suspending close: awaits queries already in flight against
            // the old instance (they run under Sdk.queryGate), so the
            // native handle is never freed mid-JNI-call. New queries
            // against the old instance fail fast once this begins.
            old?.closeSuspending()
            _errorMessage.value = null
        } catch (e: Exception) {
            _errorMessage.value = e.message ?: "Failed to initialize SDK"
            throw e
        } finally {
            _isLoading.value = false
        }
    }

    /**
     * Switch networks: rebuild the SDK first, then persist the choice.
     * A failed build (e.g. devnet without a valid quorum URL) must leave
     * both [_currentNetwork] and the persisted preference on the old
     * network — committing first would strand the UI on a network with
     * no SDK and replay the bad choice on next launch.
     */
    suspend fun switchNetwork(
        network: Network,
        dapiAddresses: String? = null,
        quorumUrl: String? = null,
    ) {
        if (network == _currentNetwork.value && _sdk.value != null) return
        initializeSdk(network, dapiAddresses, quorumUrl)
        dataStore.edit { it[KEY_NETWORK] = network.networkName }
        _currentNetwork.value = network
    }

    /**
     * Pick the quorum source — port of the Swift `AppState.quorumMode`
     * `didSet`: persist it, apply it to the current SDK and refresh the
     * indicator, on this object's scope so leaving the screen does not cut
     * that short. Picks are persisted and applied one at a time under
     * [quorumModeLock], each reading the latest pick, so quick successive
     * picks leave the saved mode and the SDK on the last one. A save or a
     * refusal that fails is reported through [errorMessage]; neither stops
     * the other.
     */
    fun setQuorumMode(mode: QuorumMode) {
        _quorumMode.value = mode
        scope.launch {
            quorumModeLock.withLock {
                val picked = _quorumMode.value
                try {
                    dataStore.edit { it[KEY_QUORUM_MODE] = picked.rawValue }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    _errorMessage.value = "Failed to save quorum mode: ${e.message}"
                }
                val sdk = _sdk.value
                if (sdk != null) {
                    try {
                        sdk.setQuorumMode(picked.sdkMode)
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        _errorMessage.value = "Failed to apply quorum mode: ${e.message}"
                    }
                }
            }
            refreshQuorumSource()
        }
    }

    /**
     * Read the current SDK's quorum source into [quorumSource] — port of
     * the Swift `AppState.refreshQuorumSource`. In Auto the adaptive
     * provider follows SPV readiness by itself, so SPV progress needs only
     * this refresh, not a mode change. An SDK a rebuild closed meanwhile is
     * skipped, as is a failed read: the indicator keeps its last value, and a
     * rebuild refreshes for its replacement.
     */
    suspend fun refreshQuorumSource() {
        val sdk = _sdk.value ?: return
        val source = try {
            sdk.quorumSource()
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            return
        }
        if (_sdk.value === sdk) _quorumSource.value = source
    }

    fun setUseDockerSetup(enabled: Boolean) {
        _useDockerSetup.value = enabled
        scope.launch { dataStore.edit { it[KEY_DOCKER] = enabled } }
    }

    fun setUseLocalhostCore(enabled: Boolean) {
        _useLocalhostCore.value = enabled
        scope.launch { dataStore.edit { it[KEY_USE_LOCAL_CORE] = enabled } }
    }

    fun setLocalCorePeers(peers: String) {
        _localCorePeers.value = peers
        scope.launch { dataStore.edit { it[KEY_LOCAL_CORE_PEERS] = peers } }
    }

    /**
     * The SPV peer override for the current settings — port of
     * `CoreContentView.spvPeerOverride()`'s custom-peers branch: honor
     * [localCorePeers] verbatim when [useLocalhostCore] is on, else empty
     * (the FFI falls back to the network's built-in seed nodes).
     */
    fun spvPeerOverride(): List<String> {
        if (!_useLocalhostCore.value) return emptyList()
        return _localCorePeers.value
            .split(",")
            .map { it.trim() }
            .filter { it.isNotEmpty() }
    }

    /** Devnet config changed without a network switch — force rebinds. */
    fun bumpRebindTick() {
        _rebindTick.value += 1
    }

    fun clearError() {
        _errorMessage.value = null
    }

    private companion object {
        val KEY_NETWORK = stringPreferencesKey("currentNetwork")
        val KEY_DOCKER = booleanPreferencesKey("useDockerSetup")

        // Verbatim iOS UserDefaults key names (OptionsView.swift).
        val KEY_USE_LOCAL_CORE = booleanPreferencesKey("useLocalhostCore")
        val KEY_LOCAL_CORE_PEERS = stringPreferencesKey("localCorePeers")
        val KEY_QUORUM_MODE = stringPreferencesKey("quorumMode")
    }
}
