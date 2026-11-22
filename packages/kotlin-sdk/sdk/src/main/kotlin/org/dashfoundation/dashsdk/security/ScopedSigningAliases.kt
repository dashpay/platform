package org.dashfoundation.dashsdk.security

/**
 * The lookups a [KeystoreSigner] resolves to another `WalletStorage` entry only while a
 * scope is open: the Android counterpart of the scoped registry behind Swift's
 * `KeychainSigner.withAdditionalSigningKeys` (`FFI/KeychainSigner.swift`), holding entry
 * names instead of key bytes.
 *
 * A masternode vote asks the signer for the 20-byte voting address, while the voting key is
 * stored under its own entry name (`MasternodeVotingKeyStore`). [open] maps the address hex
 * to that name until the returned [Scope] is closed, so outside a vote nothing resolves the
 * address to the voting key.
 *
 * Scopes may nest and overlap. Two scopes for the same lookup can name different entries
 * (two masternodes that share one voting key each store it under their own name); every
 * entry for one lookup holds the same key, because the lookup is derived from the key. While
 * several are open the most recently opened one still open resolves, and closing them in any
 * order leaves the others in place. Thread safe: the signer resolves on its IO scope while a
 * vote opens and closes scopes on another thread.
 */
internal class ScopedSigningAliases {

    /** One open mapping; identity-compared so closing removes exactly this one. */
    inner class Scope internal constructor(
        internal val lookupHex: String,
        internal val storageKey: String,
    ) : AutoCloseable {
        override fun close() = remove(this)
    }

    private val lock = Any()

    /** Lookup hex to its open scopes, oldest first. */
    private val open = HashMap<String, MutableList<Scope>>()

    /**
     * Resolve [lookupHex] (any case) to [storageKey] until the returned scope is closed.
     * Closing it twice is harmless.
     */
    fun open(lookupHex: String, storageKey: String): Scope {
        val scope = Scope(lookupHex.lowercase(), storageKey)
        synchronized(lock) { open.getOrPut(scope.lookupHex) { mutableListOf() }.add(scope) }
        return scope
    }

    /** The entry [lookupHex] resolves to now, or null when no scope for it is open. */
    fun resolve(lookupHex: String): String? =
        synchronized(lock) { open[lookupHex.lowercase()]?.lastOrNull()?.storageKey }

    private fun remove(scope: Scope) {
        synchronized(lock) {
            val scopes = open[scope.lookupHex] ?: return
            scopes.removeAll { it === scope }
            if (scopes.isEmpty()) open.remove(scope.lookupHex)
        }
    }
}
