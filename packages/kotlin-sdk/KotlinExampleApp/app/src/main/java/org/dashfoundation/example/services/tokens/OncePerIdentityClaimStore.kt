package org.dashfoundation.example.services.tokens

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.map
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.example.util.toHex

/**
 * Device-local memory of the once-per-identity distributions an identity
 * already claimed (protocol version 14).
 *
 * Platform has no "has this identity claimed" query yet, and a second claim
 * is a paid rejection (`TokenOncePerIdentityDistributionAlreadyClaimedError`,
 * code 40722), so the app remembers what it learned: a claim it submitted
 * that succeeded, or one that came back as already claimed. A fresh install
 * starts empty and learns the state from the first rejection.
 */
class OncePerIdentityClaimStore(
    private val dataStore: DataStore<Preferences>,
) {
    fun observe(networkRaw: Int, tokenId: ByteArray, identityId: ByteArray): Flow<Boolean> {
        val key = preferenceKey(networkRaw, tokenId, identityId)
        return dataStore.data
            .map { preferences -> preferences[key] == true }
            .catch { emit(false) }
    }

    suspend fun markClaimed(networkRaw: Int, tokenId: ByteArray, identityId: ByteArray) {
        dataStore.edit { preferences ->
            preferences[preferenceKey(networkRaw, tokenId, identityId)] = true
        }
    }

    companion object {
        /** Consensus code of `TokenOncePerIdentityDistributionAlreadyClaimedError`. */
        const val ALREADY_CLAIMED_ERROR_CODE = 40722

        /**
         * True when [error] is the already-claimed rejection: the SDK error,
         * or one in its cause chain, carries the consensus code the native
         * layer read off Platform's verdict. The message is never consulted.
         * Claim errors quote amounts and millisecond timestamps, so text that
         * happens to contain the digits says nothing, and a false positive is
         * permanent because the record is never cleared.
         */
        fun isAlreadyClaimed(error: Throwable): Boolean =
            generateSequence(error) { it.cause }
                .filterIsInstance<DashSdkError>()
                .any { it.consensusError?.code == ALREADY_CLAIMED_ERROR_CODE }

        private fun preferenceKey(networkRaw: Int, tokenId: ByteArray, identityId: ByteArray) =
            booleanPreferencesKey(
                "once_per_identity_claimed.$networkRaw.${tokenId.toHex()}.${identityId.toHex()}",
            )
    }
}
