package org.dashfoundation.example.services.tokens

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.map
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
         * The code as a number of its own. Error texts carry amounts and
         * millisecond timestamps, and a claim time such as 1758140722000
         * contains the digits, so a plain substring match would take an
         * unrelated failure for a spent claim and hide the kind for good.
         */
        private val ALREADY_CLAIMED_CODE_PATTERN =
            Regex("(?<![0-9])$ALREADY_CLAIMED_ERROR_CODE(?![0-9])")

        /**
         * True when [error] is the already-claimed rejection. The native
         * layer surfaces consensus errors as text, so match the code and the
         * message rs-dpp renders for it.
         */
        fun isAlreadyClaimed(error: Throwable): Boolean {
            val message = generateSequence(error) { it.cause }
                .mapNotNull { it.message }
                .joinToString(" ")
            return ALREADY_CLAIMED_CODE_PATTERN.containsMatchIn(message) ||
                message.contains("already claimed the once-per-identity distribution")
        }

        private fun preferenceKey(networkRaw: Int, tokenId: ByteArray, identityId: ByteArray) =
            booleanPreferencesKey(
                "once_per_identity_claimed.$networkRaw.${tokenId.toHex()}.${identityId.toHex()}",
            )
    }
}
