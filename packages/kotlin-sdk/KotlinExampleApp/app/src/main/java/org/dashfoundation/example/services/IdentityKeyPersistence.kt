package org.dashfoundation.example.services

/**
 * The single place a derived identity private key is persisted and then
 * scrubbed. Both registration-key provisioning ([DashpayKeyProvisioning]) and
 * the add-key flow ([IdentityKeyAdditionFlow]) route their one allowed Kotlin
 * persist step through here, so the secret-handling discipline — store the
 * scalar, then zero the JVM copy on both the success and the failure path —
 * lives in exactly one tested place rather than being re-implemented per flow.
 */
object IdentityKeyPersistence {

    /**
     * Persists one derived identity private key (public-key hex → scalar),
     * owner-scoped so wallet deletion can reach it before the identity's
     * `public_keys` row commits. Injected so callers stay unit-testable
     * without an Android Keystore.
     */
    fun interface PrivateKeyPersister {
        suspend fun persist(publicKeyHex: String, privateKey: ByteArray, ownerWalletId: ByteArray)
    }

    /**
     * Persist [privateKey] under [publicKeyHex] (scoped to [walletId]), then
     * scrub the JVM copy — whether the persist succeeds or throws.
     *
     * The Keystore is authoritative once this returns; the caller must not
     * read [privateKey] afterward (it is zeroed). Callers deriving a batch of
     * keys up front are still responsible for scrubbing the keys they have
     * NOT yet handed to this function if an earlier one fails — this only owns
     * the one scalar it was given.
     */
    suspend fun storeAndScrub(
        publicKeyHex: String,
        privateKey: ByteArray,
        walletId: ByteArray,
        persister: PrivateKeyPersister,
    ) {
        try {
            // The one allowed Kotlin persist step (kotlin-sdk/CLAUDE.md): Rust
            // derived the scalar, we only encrypt it into the Keystore keyed by
            // the public bytes the signer looks up.
            persister.persist(publicKeyHex, privateKey, walletId)
        } finally {
            privateKey.fill(0)
        }
    }
}
