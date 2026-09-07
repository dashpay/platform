package org.dashfoundation.dashsdk.tokens

import android.content.SharedPreferences
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.normalizeDpnsLabel

/**
 * Local record of recipients explicitly confirmed by this wallet's user.
 * Mirrors the Swift tip sheet's persistent recipient-change warning. This
 * record only informs confirmation; Rust still verifies the recipient before sending.
 */
class ShieldedTipRecipientHistory(private val preferences: SharedPreferences) {
    fun hasChanged(
        network: Network, walletId: ByteArray, username: String, recipient: ShieldedTipRecipient,
    ): Boolean {
        val previous = preferences.getString(key(network, walletId, username), null) ?: return false
        return previous != snapshot(recipient)
    }

    /** Call only after explicit confirmation, before submitting the payment. */
    suspend fun confirm(
        network: Network, walletId: ByteArray, username: String, recipient: ShieldedTipRecipient,
    ) = withContext(Dispatchers.IO) {
        check(preferences.edit().putString(key(network, walletId, username), snapshot(recipient)).commit()) {
            "Could not save recipient confirmation"
        }
    }

    private fun key(network: Network, walletId: ByteArray, username: String): String {
        val label = username.trim().lowercase().removeSuffix(".dash")
        return "${network.ffiValue}:${walletId.hex()}:${normalizeDpnsLabel(label)}.dash"
    }

    private fun snapshot(recipient: ShieldedTipRecipient): String =
        "${recipient.identityId.hex()}:${recipient.address.hex()}"

    private fun ByteArray.hex(): String = joinToString("") { "%02x".format(it) }
}
