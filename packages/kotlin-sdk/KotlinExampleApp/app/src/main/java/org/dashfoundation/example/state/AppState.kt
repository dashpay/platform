package org.dashfoundation.example.state

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.MutablePreferences
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
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
    /**
     * Resolves a devnet name to its quorum service URL, refusing names the
     * Rust trusted context provider refuses. Tests pass a stand-in so they
     * load no native code.
     */
    private val devnetQuorumBaseUrl: (String) -> String = Sdk::devnetQuorumBaseUrl,
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
     * The devnet's name (`dashd -devnet=<name>`) — iOS `platformDevnetName`
     * key (OptionsView.swift). SPV needs it on devnet: it goes into the user
     * agent, and Dash Core devnet peers drop handshakes without it. It also
     * gives the default quorum URL when [devnetQuorumUrl] is empty.
     */
    private val _devnetName = MutableStateFlow("")
    val devnetName: StateFlow<String> = _devnetName.asStateFlow()

    /**
     * Devnet quorum service base URL — iOS `platformQuorumURL` key. Empty
     * means "derive it from [devnetName]" (see [resolvedDevnetQuorumUrl]).
     * Both the DAPI list and the SPV peers come from its `/masternodes`.
     */
    private val _devnetQuorumUrl = MutableStateFlow("")
    val devnetQuorumUrl: StateFlow<String> = _devnetQuorumUrl.asStateFlow()

    /** The devnet settings an SDK was built from, kept with that SDK. */
    data class AppliedDevnet(val name: String?, val quorumUrl: String)

    /**
     * The devnet settings the current SDK was built from; null off devnet.
     * The Settings fields change [devnetName] and [devnetQuorumUrl] as the
     * user types, but SPV starts from this snapshot, so it never joins a
     * devnet other than the one the SDK and wallet manager were built for.
     */
    private val _appliedDevnet = MutableStateFlow<AppliedDevnet?>(null)
    val appliedDevnet: StateFlow<AppliedDevnet?> = _appliedDevnet.asStateFlow()

    /**
     * Held while an SDK is built and published and the network choice that
     * goes with it is saved, so a devnet Apply and a network switch never
     * interleave: the last one to take the lock decides both the SDK and
     * [currentNetwork].
     */
    private val sdkLock = Mutex()

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

    /** Drops a quorum-source read that a later refresh superseded. */
    private val quorumSourceReads = LatestRead()

    /** Restore persisted preferences; called once from bootstrap. */
    suspend fun restorePreferences() {
        val prefs = dataStore.data.first()
        prefs[KEY_NETWORK]?.let { name ->
            Network.fromNetworkName(name)?.let { _currentNetwork.value = it }
        }
        _useDockerSetup.value = prefs[KEY_DOCKER] ?: false
        _useLocalhostCore.value = prefs[KEY_USE_LOCAL_CORE] ?: false
        _localCorePeers.value = prefs[KEY_LOCAL_CORE_PEERS] ?: ""
        _devnetName.value = prefs[KEY_DEVNET_NAME] ?: ""
        _devnetQuorumUrl.value = prefs[KEY_DEVNET_QUORUM_URL] ?: ""
        // A saved Devnet choice whose settings are missing or refused could
        // never build an SDK: start on the default network instead. Other
        // launch failures offer the same way out (AppRoot).
        if (_currentNetwork.value == Network.DEVNET && !devnetSettingsResolve()) {
            _currentNetwork.value = Network.DEFAULT
        }
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
        sdkLock.withLock { buildAndPublishSdk(network, dapiAddresses, quorumUrl) }
    }

    private suspend fun buildAndPublishSdk(
        network: Network,
        dapiAddresses: String?,
        quorumUrl: String?,
    ) {
        _isLoading.value = true
        try {
            // Devnet has no built-in endpoints: the SDK discovers its DAPI
            // nodes from the quorum service, so take the URL from the devnet
            // settings when the caller passed none. The name and URL are read
            // together here, before the build suspends, and published with
            // the SDK: an edit made during the build waits for the next Apply.
            val explicitQuorumUrl = quorumUrl?.takeIf { it.isNotBlank() }
            val devnet = if (network == Network.DEVNET) {
                val name = devnetNameOrNull()
                val url = resolveDevnetQuorumUrl(name, explicitQuorumUrl ?: _devnetQuorumUrl.value)
                    ?: throw IllegalStateException(DEVNET_SETTINGS_MISSING)
                AppliedDevnet(name, url)
            } else {
                null
            }
            val effectiveQuorumUrl = devnet?.quorumUrl ?: explicitQuorumUrl
            val newSdk = Sdk.create(
                SdkConfig(
                    network = network,
                    dapiAddresses = dapiAddresses,
                    quorumUrl = effectiveQuorumUrl,
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
                _sdk.value.also {
                    _sdk.value = newSdk
                    recordAppliedDevnet(devnet)
                }
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
        sdkLock.withLock {
            if (network == _currentNetwork.value && _sdk.value != null) return
            buildAndPublishSdk(network, dapiAddresses, quorumUrl)
            // The new SDK is published, so the network it serves is now the
            // current one. Saving the pick can still fail; then only the next
            // launch starts on the old network.
            _currentNetwork.value = network
            try {
                dataStore.edit { it[KEY_NETWORK] = network.networkName }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _errorMessage.value =
                    "Switched to ${network.displayName}, but could not save it: ${e.message}"
            }
        }
    }

    /**
     * Rebuild the devnet SDK from the edited devnet settings — the Settings
     * "Apply Devnet Settings" button. Does nothing once the app has left
     * devnet, so an Apply that waited behind a network switch never
     * publishes a devnet SDK under another network.
     */
    suspend fun applyDevnetSettings() {
        sdkLock.withLock {
            if (_currentNetwork.value != Network.DEVNET) return
            buildAndPublishSdk(Network.DEVNET, null, null)
        }
    }

    /**
     * Leave a saved network the app cannot start on — the launch error
     * screen's way out. Saves [Network.DEFAULT] as the network; wallets and
     * every other setting, the devnet ones included, stay as they are.
     */
    suspend fun leaveSavedNetwork() {
        sdkLock.withLock {
            dataStore.edit { it[KEY_NETWORK] = Network.DEFAULT.networkName }
            _currentNetwork.value = Network.DEFAULT
        }
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
     * this refresh, not a mode change. Refreshes overlap, so a read that a
     * later refresh superseded is dropped, as is a read of an SDK a rebuild
     * closed meanwhile, and a failed read: the indicator keeps its last value,
     * and a rebuild refreshes for its replacement.
     */
    suspend fun refreshQuorumSource() {
        val sdk = _sdk.value ?: return
        try {
            quorumSourceReads.publishLatest(read = { sdk.quorumSource() }) { source ->
                if (_sdk.value === sdk) _quorumSource.value = source
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // The indicator keeps its last value.
        }
    }

    fun setUseDockerSetup(enabled: Boolean) {
        _useDockerSetup.value = enabled
        savePreference { it[KEY_DOCKER] = enabled }
    }

    fun setUseLocalhostCore(enabled: Boolean) {
        _useLocalhostCore.value = enabled
        savePreference { it[KEY_USE_LOCAL_CORE] = enabled }
    }

    fun setLocalCorePeers(peers: String) {
        _localCorePeers.value = peers
        savePreference { it[KEY_LOCAL_CORE_PEERS] = peers }
    }

    fun setDevnetName(name: String) {
        _devnetName.value = name
        savePreference { it[KEY_DEVNET_NAME] = name }
    }

    fun setDevnetQuorumUrl(url: String) {
        _devnetQuorumUrl.value = url
        savePreference { it[KEY_DEVNET_QUORUM_URL] = url }
    }

    /**
     * Save a setting off the caller's path. A failed write shows in
     * [errorMessage]; the app scope has no exception handler, so letting it
     * escape would end the process.
     */
    private fun savePreference(update: (MutablePreferences) -> Unit) {
        scope.launch {
            try {
                dataStore.edit { update(it) }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                _errorMessage.value = "Could not save the setting: ${e.message}"
            }
        }
    }

    /**
     * The devnet quorum service URL: [devnetQuorumUrl] when set, else the
     * URL the Rust trusted context provider gives [devnetName]. Null when
     * neither is set.
     *
     * @throws IllegalArgumentException when [devnetQuorumUrl] is not https
     *   (the app allows no cleartext traffic) or the provider refuses the
     *   name, such as a reserved network name like `mainnet`
     */
    fun resolvedDevnetQuorumUrl(): String? =
        resolveDevnetQuorumUrl(devnetNameOrNull(), _devnetQuorumUrl.value)

    /**
     * [url] when set, else the provider's URL for [name]. A set [name] is
     * checked by the provider either way, so a URL never carries a refused
     * name to SPV.
     */
    private fun resolveDevnetQuorumUrl(name: String?, url: String): String? {
        val derived = name?.let { checkedName ->
            try {
                devnetQuorumBaseUrl(checkedName)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                throw IllegalArgumentException(
                    e.message ?: "Devnet name '$checkedName' was refused",
                    e,
                )
            }
        }
        val explicit = url.trim().takeIf { it.isNotEmpty() } ?: return derived
        require(explicit.startsWith("https://", ignoreCase = true)) { DEVNET_QUORUM_URL_NOT_HTTPS }
        return explicit
    }

    private fun devnetSettingsResolve(): Boolean =
        try {
            resolvedDevnetQuorumUrl() != null
        } catch (_: IllegalArgumentException) {
            false
        }

    /**
     * Keep [applied] as the settings the published SDK was built from.
     * Called when an SDK is published; tests call it in place of a native
     * SDK build.
     */
    internal fun recordAppliedDevnet(applied: AppliedDevnet?) {
        _appliedDevnet.value = applied
    }

    /**
     * The devnet name SPV needs on [network], from the applied settings;
     * null off devnet or when the applied settings have no name.
     */
    fun spvDevnetName(network: Network): String? =
        if (network == Network.DEVNET) _appliedDevnet.value?.name else null

    private fun devnetNameOrNull(): String? =
        _devnetName.value.trim().takeIf { it.isNotEmpty() }

    /**
     * The SPV peer override for [network] — port of
     * `CoreSpvLauncher.peerOverride(network:)`: [localCorePeers] verbatim
     * when [useLocalhostCore] is on and the list is not empty; on devnet otherwise the active
     * masternodes' Core P2P addresses from the applied settings'
     * `{quorumUrl}/masternodes` (each reports its own port, so none is
     * guessed); else empty (the FFI falls back to the network's built-in
     * seed nodes). The devnet lookup ([discover]) blocks on HTTP, so it
     * runs on [Dispatchers.IO].
     *
     * @throws IllegalStateException on devnet when no devnet SDK is
     *   applied, when the lookup finds no active masternode (a devnet has
     *   no seed nodes, so SPV would run with no peers), or when the devnet
     *   SDK was rebuilt during the lookup
     */
    suspend fun spvPeerOverride(
        network: Network = _currentNetwork.value,
        discover: (quorumUrl: String) -> List<String>? = { quorumUrl ->
            Sdk.discoverActiveMasternodes(quorumUrl, DEVNET_DAPI_PORT)?.map { it.spvPeer }
        },
    ): List<String> {
        if (_useLocalhostCore.value) {
            val custom = _localCorePeers.value
                .split(",")
                .map { it.trim() }
                .filter { it.isNotEmpty() }
            // An empty custom list on devnet falls through to discovery: a
            // devnet has no seed nodes to fall back on.
            if (custom.isNotEmpty() || network != Network.DEVNET) return custom
        }
        if (network != Network.DEVNET) return emptyList()
        val applied = _appliedDevnet.value ?: throw IllegalStateException(DEVNET_NOT_APPLIED)
        val peers = withContext(Dispatchers.IO) { discover(applied.quorumUrl) }
        check(_appliedDevnet.value === applied) { DEVNET_CHANGED_DURING_LOOKUP }
        if (peers.isNullOrEmpty()) {
            throw IllegalStateException(
                "No active masternodes found at ${applied.quorumUrl.trimEnd('/')}/masternodes. " +
                    "Check the devnet settings, or set custom SPV peers.",
            )
        }
        return peers
    }

    /** Devnet config changed without a network switch — force rebinds. */
    fun bumpRebindTick() {
        _rebindTick.value += 1
    }

    fun clearError() {
        _errorMessage.value = null
    }

    companion object {
        /** Shown when Devnet is picked before its name or quorum URL is set. */
        const val DEVNET_SETTINGS_MISSING =
            "Devnet needs a devnet name or a quorum URL. Enter them under Devnet, then pick Devnet again."

        /** Shown when the devnet quorum URL is not https. */
        const val DEVNET_QUORUM_URL_NOT_HTTPS =
            "The devnet quorum URL must start with https://; the app allows no cleartext traffic."

        /** Shown when SPV is started on devnet before a devnet SDK was built. */
        const val DEVNET_NOT_APPLIED =
            "No devnet settings are applied yet. Pick Devnet or press Apply Devnet Settings first."

        /** Shown when the devnet SDK was rebuilt while SPV peers were looked up. */
        const val DEVNET_CHANGED_DURING_LOOKUP =
            "The devnet settings changed while SPV peers were looked up. Press Start again."

        /** DAPI port for nodes that report no `platformHTTPPort` (non-mainnet). */
        private const val DEVNET_DAPI_PORT = 1443

        private val KEY_NETWORK = stringPreferencesKey("currentNetwork")
        private val KEY_DOCKER = booleanPreferencesKey("useDockerSetup")

        // Verbatim iOS UserDefaults key names (OptionsView.swift).
        private val KEY_USE_LOCAL_CORE = booleanPreferencesKey("useLocalhostCore")
        private val KEY_LOCAL_CORE_PEERS = stringPreferencesKey("localCorePeers")
        private val KEY_QUORUM_MODE = stringPreferencesKey("quorumMode")
        private val KEY_DEVNET_NAME = stringPreferencesKey("platformDevnetName")
        private val KEY_DEVNET_QUORUM_URL = stringPreferencesKey("platformQuorumURL")
    }
}
