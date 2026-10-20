package org.dashfoundation.example.ui.dashpay

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.onStart
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex

sealed interface DashPayActiveIdentityPreference {
    data object Loading : DashPayActiveIdentityPreference

    data class Ready(
        val identityIdBase58: String?,
    ) : DashPayActiveIdentityPreference

    data class Failed(
        val error: Throwable,
    ) : DashPayActiveIdentityPreference
}

/**
 * Persists the network-scoped counterpart of the active DashPay identity selection in
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/DashPay/DashPayTabView.swift`.
 */
class DashPayActiveIdentityStore(
    private val dataStore: DataStore<Preferences>,
) {
    fun observe(network: Network): Flow<DashPayActiveIdentityPreference> {
        val key = preferenceKey(network)
        return dataStore.data
            .map<Preferences, DashPayActiveIdentityPreference> { preferences ->
                DashPayActiveIdentityPreference.Ready(preferences[key])
            }
            .onStart { emit(DashPayActiveIdentityPreference.Loading) }
            .catch { error -> emit(DashPayActiveIdentityPreference.Failed(error)) }
    }

    suspend fun select(
        network: Network,
        identityIdBase58: String,
    ) {
        dataStore.edit { preferences ->
            preferences[preferenceKey(network)] = identityIdBase58
        }
    }

    suspend fun clearIfIneligible(
        network: Network,
        eligibleIdentityIdsBase58: Set<String>,
    ) {
        dataStore.edit { preferences ->
            val key = preferenceKey(network)
            val selectedIdentityId = preferences[key]
            if (
                selectedIdentityId != null &&
                selectedIdentityId !in eligibleIdentityIdsBase58
            ) {
                preferences.remove(key)
            }
        }
    }

    private fun preferenceKey(network: Network): Preferences.Key<String> =
        stringPreferencesKey("dashpay.activeIdentityId.${network.networkName}")
}

internal fun eligibleDashPayIdentities(
    walletOwnedIdentities: List<IdentityEntity>,
    loadedWalletIdsHex: Set<String>,
): List<IdentityEntity> =
    walletOwnedIdentities
        .filter { it.hasLoadedWallet(loadedWalletIdsHex) }
        .sortedBy { it.createdAt.time }

private fun IdentityEntity.hasLoadedWallet(loadedWalletIdsHex: Set<String>): Boolean =
    walletId?.toHex()?.let(loadedWalletIdsHex::contains) == true

internal fun resolveActiveDashPayIdentity(
    eligibleIdentities: List<IdentityEntity>,
    selectedIdentityIdBase58: String?,
): IdentityEntity? =
    eligibleIdentities.firstOrNull { identity ->
        Base58.encode(identity.identityId) == selectedIdentityIdBase58
    } ?: eligibleIdentities.firstOrNull()

data class DashPayWalletLoad<out T>(
    val value: T,
    val loadedWalletIdsHex: Set<String>,
)

sealed interface DashPayActiveIdentityRestorationState {
    data object Idle : DashPayActiveIdentityRestorationState

    data class Loading(
        val network: Network,
    ) : DashPayActiveIdentityRestorationState

    data class Ready(
        val network: Network,
    ) : DashPayActiveIdentityRestorationState

    data class Failed(
        val network: Network,
        val error: Throwable,
    ) : DashPayActiveIdentityRestorationState
}

/**
 * Restores the active-identity behavior ported from
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/DashPay/DashPayTabView.swift`
 * after loaded-wallet eligibility is known.
 */
class DashPayActiveIdentityRestorationCoordinator(
    private val store: DashPayActiveIdentityStore,
) {
    private val restorationMutex = Mutex()
    private val _state = MutableStateFlow<DashPayActiveIdentityRestorationState>(
        DashPayActiveIdentityRestorationState.Idle,
    )
    val state: StateFlow<DashPayActiveIdentityRestorationState> = _state.asStateFlow()

    suspend fun <T> restore(
        network: Network,
        loadWallets: suspend () -> DashPayWalletLoad<T>,
        loadWalletOwnedIdentities: suspend () -> List<IdentityEntity>,
    ): T = restorationMutex.withLock {
        val previousState = _state.value
        _state.value = DashPayActiveIdentityRestorationState.Loading(network)
        try {
            val walletLoad = loadWallets()
            val walletOwnedIdentities = loadWalletOwnedIdentities()
            val eligibleIdentityIds = walletOwnedIdentities
                .asSequence()
                .filter { it.hasLoadedWallet(walletLoad.loadedWalletIdsHex) }
                .mapTo(mutableSetOf()) { identity -> Base58.encode(identity.identityId) }
            store.clearIfIneligible(network, eligibleIdentityIds)
            _state.value = DashPayActiveIdentityRestorationState.Ready(network)
            walletLoad.value
        } catch (error: CancellationException) {
            _state.value = previousState
            throw error
        } catch (error: Throwable) {
            _state.value = DashPayActiveIdentityRestorationState.Failed(network, error)
            throw error
        }
    }
}
