package org.dashfoundation.example.services

import org.dashfoundation.dashsdk.identity.IdentityKeyPreview
import org.dashfoundation.dashsdk.identity.RegistrationKeySet
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
 * - its policy depends on the existing identity's current key IDs and builds
 *   an update batch, while registration needs a fixed 0..N role table before
 *   the identity exists.
 *
 * Forcing registration through it would fork its semantics; this helper reuses
 * the shared pieces instead — the SDK's [RegistrationKeys] role table for the
 * on-chain rows and [IdentityKeyPersistence] for the store-then-zero secret
 * discipline (shared with [IdentityKeyAdditionFlow]).
 */
object DashpayKeyProvisioning {

    /**
     * Persist each derived private key, scrub the JVM copy, and return the
     * rich rows the registration wire format carries, tied to the identity HD
     * slot that produced them.
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
        persister: IdentityKeyPersistence.PrivateKeyPersister,
    ): RegistrationKeySet {
        val expected = RegistrationKeys.keyCount(includeDashPayKeys)
        try {
            // Inside the try so the catch below scrubs the derived scalars even
            // on a count-mismatch rejection (a wire-skew symptom) — no plaintext
            // key may survive any failure once the previews have been derived.
            require(previews.size == expected) {
                "expected $expected derived registration keys, got ${previews.size}"
            }
            val identityIndex = previews.first().identityIndex
            require(previews.all { it.identityIndex == identityIndex }) {
                "every registration key must use identityIndex $identityIndex"
            }
            for (preview in previews) {
                IdentityKeyPersistence.storeAndScrub(
                    publicKeyHex = preview.publicKeyHex,
                    privateKey = preview.privateKey,
                    walletId = walletId,
                    persister = persister,
                )
            }
        } catch (e: Throwable) {
            // The whole set was derived up front, so on any failure scrub every
            // preview's private half — including keys `storeAndScrub` never
            // reached — so no plaintext scalar survives a partial failure.
            previews.forEach { it.privateKey.fill(0) }
            throw e
        }
        // Only the public keys cross into the rich rows — the privates are gone.
        return RegistrationKeySet(
            identityIndex = previews.first().identityIndex,
            rows = RegistrationKeys.buildRegistrationRows(
                previews.map { it.publicKey },
                includeDashPayKeys,
            ),
        )
    }
}
