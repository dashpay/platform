package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.dashsdk.security.WalletStorage

/**
 * Per-network manager caching semantics — the testable core behind
 * [WalletManagerStore], generic over the manager type `M` so unit tests
 * can drive it with a fake factory (no native library required).
 *
 * Keyed by [Network]. Each cache entry remembers the SDK handle it was
 * built against so a rebuilt SDK (handle change) invalidates the stale
 * manager. All mutation is serialized by a [Mutex].
 *
 * **Close-after-publish**: on a genuine switch the fresh manager is stored
 * and published to [activeManager] BEFORE any superseded manager is
 * closed, so no observer ever sees a closed manager as active.
 *
 * @param isClosedOf probe: has this manager been closed?
 * @param closeOf tear down a manager.
 * @param factory build a manager for `(network, sdkHandle)`.
 */
internal class WalletManagerCache<M>(
    private val isClosedOf: (M) -> Boolean,
    private val closeOf: (M) -> Unit,
    private val factory: (network: Network, sdkHandle: Long) -> M,
) {
    private val lock = Mutex()
    private val managers = HashMap<Network, M>()
    private val managerSdkHandles = HashMap<Network, Long>()

    private val _activeManager = MutableStateFlow<M?>(null)
    val activeManager: StateFlow<M?> = _activeManager.asStateFlow()

    suspend fun activate(network: Network, sdkHandle: Long, makeActive: Boolean): M =
        lock.withLock {
            val cached = managers[network]
            val cachedHandle = managerSdkHandles[network]
            if (cached != null && !isClosedOf(cached) && cachedHandle == sdkHandle) {
                if (makeActive && _activeManager.value !== cached) _activeManager.value = cached
                return@withLock cached
            }

            val stale = cached
            val manager = factory(network, sdkHandle)
            managers[network] = manager
            managerSdkHandles[network] = sdkHandle

            val previousActive = if (makeActive) {
                val prev = _activeManager.value
                _activeManager.value = manager // publish FIRST
                prev
            } else {
                null
            }

            // Retire the stale per-network entry AFTER publishing.
            if (stale != null && stale !== manager) {
                runCatching { closeOf(stale) }
            }
            // Retire the previously-active manager only on a genuine
            // cross-instance switch.
            if (previousActive != null &&
                previousActive !== manager &&
                previousActive !== stale
            ) {
                runCatching { closeOf(previousActive) }
            }
            manager
        }

    fun manager(forNetwork: Network): M? =
        managers[forNetwork]?.takeUnless { isClosedOf(it) }

    suspend fun closeAll() = lock.withLock {
        _activeManager.value = null
        val all = managers.values.toList()
        managers.clear()
        managerSdkHandles.clear()
        all.forEach { runCatching { closeOf(it) } }
    }
}

/**
 * Per-network cache of [PlatformWalletManager]s and the currently-active
 * one — port of `WalletManagerStore.swift`.
 *
 * A [PlatformWalletManager] is network-locked at construction, so
 * switching networks means building a fresh manager for the target
 * network and tearing down the previous one. This is a thin binding over
 * [WalletManagerCache]:
 *
 * - [activate] get-or-builds the manager for a network and (by default)
 *   publishes it as [activeManager].
 * - [manager] returns a cached manager without building.
 * - [backgroundManager] get-or-builds a manager WITHOUT changing the
 *   active one (orphan-recovery routing to a wallet's original network).
 *
 * The store is not a singleton; hold one instance per app session.
 *
 * @param database shared Room database handed to every manager.
 * @param walletStorage shared Keystore-wrapped secret store.
 * @param biometricGate optional auth gate forwarded to each manager.
 */
class WalletManagerStore(
    private val database: DashDatabase,
    private val walletStorage: WalletStorage,
    private val biometricGate: BiometricGate? = null,
) {
    // Managers are built lazily from the SDK captured at activate() time.
    private var pendingSdk: Sdk? = null

    private val cache = WalletManagerCache<PlatformWalletManager>(
        isClosedOf = { it.isClosed },
        closeOf = { it.close() },
        factory = { network, _ ->
            val sdk = pendingSdk
                ?: error("WalletManagerStore: no SDK captured for $network activation")
            PlatformWalletManager(sdk, network, database, walletStorage, biometricGate)
        },
    )

    /** The currently-active manager, or null before the first [activate]. */
    val activeManager: StateFlow<PlatformWalletManager?> = cache.activeManager

    /**
     * Get-or-build the manager for [network] against [sdk], optionally
     * publishing it as [activeManager]. `require`s `sdk.network == network`
     * (the manager is network-locked). Idempotent when the cached manager
     * was built against the same live SDK handle; a rebuilt SDK replaces
     * the stale manager. Close-after-publish ordering is guaranteed by the
     * underlying cache.
     */
    suspend fun activate(
        network: Network,
        sdk: Sdk,
        makeActive: Boolean = true,
    ): PlatformWalletManager {
        require(sdk.network == network) {
            "activate: sdk.network=${sdk.network} does not match requested network=$network"
        }
        pendingSdk = sdk
        return cache.activate(network, sdk.handle, makeActive)
    }

    /** The cached manager for [network], or null if none is activated. */
    fun manager(forNetwork: Network): PlatformWalletManager? = cache.manager(forNetwork)

    /**
     * Get-or-build the manager for [network] WITHOUT changing the active
     * one — port of Swift `backgroundManager(for:)`.
     */
    suspend fun backgroundManager(network: Network, sdk: Sdk): PlatformWalletManager =
        activate(network, sdk, makeActive = false)

    /** Close every cached manager and clear the store. */
    suspend fun closeAll() = cache.closeAll()
}
