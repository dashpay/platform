package org.dashfoundation.example.state

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
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

    /** Restore persisted preferences; called once from bootstrap. */
    suspend fun restorePreferences() {
        val prefs = dataStore.data.first()
        prefs[KEY_NETWORK]?.let { name ->
            Network.fromNetworkName(name)?.let { _currentNetwork.value = it }
        }
        _useDockerSetup.value = prefs[KEY_DOCKER] ?: false
        _useLocalhostCore.value = prefs[KEY_USE_LOCAL_CORE] ?: false
        _localCorePeers.value = prefs[KEY_LOCAL_CORE_PEERS] ?: ""
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
            )
            val old = _sdk.value
            _sdk.value = newSdk
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
    }
}
