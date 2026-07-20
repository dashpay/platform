package org.dashfoundation.example.services

import org.dashfoundation.dashsdk.identity.IdentityKeyPreview
import org.dashfoundation.dashsdk.identity.IdentityPubkey
import org.dashfoundation.dashsdk.identity.RegistrationKeys

/**
 * Registration key provisioning — the derive-set → persist → zero →
 * build-rich-rows sequence the create-identity flow runs before it broadcasts.
 * App-level analog of Swift's `CreateIdentityView` submit path
 * (`prePersistIdentityKeysForRegistration` + `makeDashpayKeyPair`).
 *
 * ## Why a dedicated helper rather than reusing `IdentityKeyAdditionFlow`
 *
 * `IdentityKeyAdditionFlow.prepareKeys` looks similar (it also derives →
 * persists → zeroes → builds [IdentityPubkey] rows), but it is built for
 * adding keys to an *existing* identity and does not fit registration:
 * - it assigns key IDs as `max(existing) + 1`, so it can never produce the
 *   MASTER slot at key ID 0 that registration requires;
 * - it derives one slot at a time through an injected `SlotKeyDeriver`,
 *   whereas registration derives the whole key set in a **single**
 *   `previewRegistrationKeySet` pass — the secret-lifecycle mandate is one
 *   derivation, not N overlapping ones;
 * - its lifecycle assumes post-registration persistence (an existing
 *   identity id), not the pre-broadcast persistence registration needs.
 *
 * Forcing registration through it would fork its semantics; this helper reuses
 * the shared pieces instead — the SDK's [RegistrationKeys] role table for the
 * on-chain rows and the same store-then-zero discipline for the secrets.
 */
object DashpayKeyProvisioning {

    /**
     * Persists one derived identity private key (public-key hex → scalar),
     * owner-scoped so wallet deletion can reach it before the identity's
     * `public_keys` row commits. Injected so the provisioning logic is
     * unit-testable without an Android Keystore.
     */
    fun interface PrivateKeyPersister {
        suspend fun persist(publicKeyHex: String, privateKey: ByteArray, ownerWalletId: ByteArray)
    }

    /**
     * Persist each derived private key, scrub the JVM copy, and return the
     * rich [IdentityPubkey] rows the registration wire format carries.
     *
     * [previews] must be exactly `RegistrationKeys.keyCount(includeDashPayKeys)`
     * rows in key-ID order — the single [IdentityKeyPreview] derivation pass
     * (`previewRegistrationKeySet`). Every preview's private array is zeroed on
     * both the success and every failure path (including the count-mismatch
     * rejection); a partial persist failure never leaves a plaintext scalar
     * behind. Only the public halves flow into the returned rows.
     *
     * The derived scalar and its public key come from the same Rust preview
     * row, so they are paired by construction — this deliberately does not
     * re-run Swift's `validatePrivateKeyForPublicKey` cross-check (there is no
     * private-key math on the Kotlin side to check against; a pub/priv mismatch
     * would require a Rust derivation bug, which for key 0 fails the create
     * signature immediately).
     */
    suspend fun provision(
        previews: List<IdentityKeyPreview>,
        includeDashPayKeys: Boolean,
        walletId: ByteArray,
        persister: PrivateKeyPersister,
    ): List<IdentityPubkey> {
        val expected = RegistrationKeys.keyCount(includeDashPayKeys)
        try {
            // Inside the try so the catch below scrubs the derived scalars even
            // on a count-mismatch rejection (a wire-skew symptom) — no plaintext
            // key may survive any failure once the previews have been derived.
            require(previews.size == expected) {
                "expected $expected derived registration keys, got ${previews.size}"
            }
            for (preview in previews) {
                try {
                    // The one allowed Kotlin persist step (kotlin-sdk/CLAUDE.md):
                    // Rust derived the scalar, we only encrypt it into the
                    // Keystore keyed by the public bytes the signer looks up.
                    persister.persist(preview.publicKeyHex, preview.privateKey, walletId)
                } finally {
                    // Keystore is authoritative from here; the JVM copy must not
                    // outlive the store (IdentityKeyPreview's retention rule).
                    preview.privateKey.fill(0)
                }
            }
        } catch (e: Throwable) {
            // A persist threw after its own key was zeroed by the finally
            // above; scrub every remaining preview's private half too so no
            // plaintext scalar survives a partial failure.
            previews.forEach { it.privateKey.fill(0) }
            throw e
        }
        // Only the public keys cross into the rich rows — the privates are gone.
        return RegistrationKeys.buildRegistrationRows(
            previews.map { it.publicKey },
            includeDashPayKeys,
        )
    }
}
