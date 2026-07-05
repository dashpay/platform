package org.dashfoundation.example.services

import org.dashfoundation.dashsdk.identity.ContractBounds
import org.dashfoundation.dashsdk.identity.IdentityPubkey
import org.dashfoundation.dashsdk.identity.KeyPurpose
import org.dashfoundation.dashsdk.identity.KeyType
import org.dashfoundation.dashsdk.identity.SecurityLevel
import org.dashfoundation.dashsdk.security.WalletStorage

/**
 * Shared "add a key to an existing identity" plumbing — port of
 * `IdentityKeyAddition.swift`. Runs the derive → persist →
 * build-[IdentityPubkey] sequence `AddIdentityKeyView.submit()` performs,
 * so both `AddIdentityKeyScreen` and the generic transition builder drive
 * one implementation.
 *
 * The wallet-side derivation is injected as a [SlotKeyDeriver] because the
 * Rust derive itself is bridged only scalar-shaped today:
 * `IdentityNative.deriveIdentityPrivateKeyWithResolver` wraps
 * `dash_sdk_derive_identity_key_at_slot_with_resolver` but copies out only
 * `IdentityKeyPreviewFFI.private_key_bytes`, discarding the row's
 * `public_key` — and the on-chain [IdentityPubkey] row needs the public
 * half. [UnbridgedSlotDeriver] names that missing marshalling; when the
 * JNI grows a pubkey-returning variant, swapping the deriver completes the
 * flow with no changes here.
 */
object IdentityKeyAdditionFlow {

    /**
     * One key the caller wants to add, described by its DPP attributes —
     * port of Swift's `IdentityKeyAddition.KeySpec`. The keypair itself is
     * derived wallet-side; the caller never supplies private material.
     */
    data class KeySpec(
        val keyType: KeyType,
        val purpose: KeyPurpose,
        val securityLevel: SecurityLevel,
        val contractBounds: ContractBounds? = null,
    )

    /** The derived material for one new key slot. */
    data class DerivedKey(
        /** 33-byte compressed public key. */
        val publicKey: ByteArray,
        /** 32-byte private scalar — persisted then scrubbed by the flow. */
        val privateKey: ByteArray,
    )

    /** Derives the identity keypair at `(identityIndex, keyId)`. */
    fun interface SlotKeyDeriver {
        suspend fun derive(identityIndex: Int, keyId: Int): DerivedKey
    }

    /** A [KeySpec] the flow cannot fulfil, with user-facing copy. */
    class KeyAdditionException(message: String) : Exception(message)

    /**
     * The scalar-only slot-derive gap, named precisely — the Android
     * analog of a `notBridged` dialog body (grep `notBridged` under `ui/`).
     */
    object UnbridgedSlotDeriver : SlotKeyDeriver {
        const val EXPORT_NOTE: String =
            "Deriving the new key's public half is not bridged yet: " +
                "`IdentityNative.deriveIdentityPrivateKeyWithResolver` " +
                "(over `dash_sdk_derive_identity_key_at_slot_with_resolver`) " +
                "returns only the 32-byte private scalar and discards " +
                "`IdentityKeyPreviewFFI.public_key`, but the " +
                "`IdentityUpdateTransition` row requires the public key. " +
                "A pubkey-returning slot-derive JNI export completes this flow."

        override suspend fun derive(identityIndex: Int, keyId: Int): DerivedKey =
            throw KeyAdditionException(EXPORT_NOTE)
    }

    /**
     * Auto-assign slots as `max(existingKeyIds) + 1, +2, …` (non-recyclable
     * — disabled keys leave a hole; new keys always extend past the highest
     * ever used, mirroring `AddIdentityKeyView.nextKeyId`).
     */
    fun nextKeyId(existingKeyIds: Collection<Int>): Int =
        (existingKeyIds.maxOrNull() ?: 0) + 1

    /**
     * Validate [spec] against the combinations Drive accepts — the same
     * gating `AddIdentityKeyView` / `IdentityKeyAddition.prepareKeys`
     * enforce before deriving. Returns the user-facing refusal, or null
     * when the spec is submittable.
     */
    fun validationError(spec: KeySpec): String? {
        if (spec.keyType == KeyType.BLS12_381) {
            return "BLS derivation is not yet wired through the FFI for this flow. " +
                "Use ECDSA secp256k1 or ECDSA Hash160."
        }
        val boundsRequired =
            spec.purpose == KeyPurpose.ENCRYPTION || spec.purpose == KeyPurpose.DECRYPTION
        if (boundsRequired && spec.contractBounds == null) {
            return "Encryption / decryption keys must be bound to a contract — " +
                "Drive rejects unbounded keys for those purposes."
        }
        if (!boundsRequired && spec.contractBounds != null) {
            return "Contract bounds are only valid on encryption / decryption keys."
        }
        return null
    }

    /**
     * Derive each requested key, pre-persist its private scalar to
     * Keystore-backed [walletStorage] (keyed by public-key hex so
     * `KeystoreSigner` can find it), and build the matching
     * [IdentityPubkey] rows — without broadcasting. Port of
     * `IdentityKeyAddition.prepareKeys`; slots are assigned via
     * [nextKeyId] so they never collide.
     *
     * @throws KeyAdditionException when a spec fails [validationError] or
     *   the deriver cannot produce the keypair (see [UnbridgedSlotDeriver]).
     */
    suspend fun prepareKeys(
        specs: List<KeySpec>,
        existingKeyIds: Collection<Int>,
        identityIndex: Int,
        walletStorage: WalletStorage,
        deriver: SlotKeyDeriver = UnbridgedSlotDeriver,
    ): List<IdentityPubkey> {
        var freeKeyId = nextKeyId(existingKeyIds)
        val rows = ArrayList<IdentityPubkey>(specs.size)
        for (spec in specs) {
            validationError(spec)?.let { throw KeyAdditionException(it) }
            val keyId = freeKeyId
            freeKeyId += 1

            val derived = deriver.derive(identityIndex, keyId)
            val pubkeyHex = derived.publicKey.joinToString("") { "%02x".format(it) }
            try {
                // The one allowed Kotlin persist step (kotlin-sdk/CLAUDE.md):
                // Rust derived; we only encrypt the scalar into the Keystore.
                walletStorage.storePrivateKey(pubkeyHex, derived.privateKey)
            } finally {
                derived.privateKey.fill(0)
            }

            rows.add(
                IdentityPubkey(
                    keyId = keyId,
                    keyType = spec.keyType,
                    purpose = spec.purpose,
                    securityLevel = spec.securityLevel,
                    pubkeyBytes = derived.publicKey,
                    contractBounds = spec.contractBounds,
                ),
            )
        }
        return rows
    }
}
