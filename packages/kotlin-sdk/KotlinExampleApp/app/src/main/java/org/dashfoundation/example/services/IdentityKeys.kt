package org.dashfoundation.example.services

import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity

/**
 * Room `public_keys` rows store `purpose` / `securityLevel` / `keyType`
 * as the *stringified DPP rawValue* (the persister writes
 * `(purpose.toInt() and 0xFF).toString()`, mirroring Swift's
 * `String(purpose.rawValue)`), while hand-written code and the iOS UI
 * compare against the enum *names*. These helpers normalize either form
 * to the canonical DPP name so gates and signing-key resolution behave
 * identically for both encodings.
 */
object IdentityKeys {

    /** DPP `Purpose` discriminants, in rawValue order. */
    private val PURPOSE_NAMES = listOf(
        "AUTHENTICATION", "ENCRYPTION", "DECRYPTION", "TRANSFER",
        "SYSTEM", "VOTING", "OWNER",
    )

    /** DPP `SecurityLevel` discriminants, in rawValue order. */
    private val SECURITY_LEVEL_NAMES = listOf("MASTER", "CRITICAL", "HIGH", "MEDIUM")

    /** DPP `KeyType` discriminants, in rawValue order. */
    private val KEY_TYPE_NAMES = listOf(
        "ECDSA_SECP256K1", "BLS12_381", "ECDSA_HASH160",
        "BIP13_SCRIPT_HASH", "EDDSA_25519_HASH160",
    )

    /** Canonical DPP purpose name for a raw Room value ("0" or "AUTHENTICATION"). */
    fun purposeName(raw: String): String = normalize(raw, PURPOSE_NAMES)

    /** Canonical DPP security-level name for a raw Room value. */
    fun securityLevelName(raw: String): String = normalize(raw, SECURITY_LEVEL_NAMES)

    /** Canonical DPP key-type name for a raw Room value. */
    fun keyTypeName(raw: String): String = normalize(raw, KEY_TYPE_NAMES)

    /**
     * The numeric DPP security-level rank for a raw Room value (lower =
     * stronger, matching rs-dpp's `SecurityLevel` ordering: MASTER 0 …
     * MEDIUM 3). Unknown values rank weakest.
     */
    fun securityLevelRank(raw: String): Int {
        val name = securityLevelName(raw)
        val index = SECURITY_LEVEL_NAMES.indexOf(name)
        return if (index >= 0) index else SECURITY_LEVEL_NAMES.size
    }

    private fun normalize(raw: String, names: List<String>): String {
        val trimmed = raw.trim()
        trimmed.toIntOrNull()?.let { ordinal ->
            return names.getOrNull(ordinal) ?: trimmed
        }
        val upper = trimmed.uppercase()
        return names.firstOrNull { it == upper } ?: upper
    }

    /**
     * Resolve the AUTHENTICATION key an identity should sign a
     * document / contract transition with — port of the key-selection
     * half of `DocumentActionRunner.resolveSigning` (DocumentsView.swift):
     * an *enabled* AUTHENTICATION key at or above [minimumSecurityLevel]
     * (numeric DPP rank, e.g. 2 = HIGH), preferring the strongest
     * (lowest-rank, i.e. most critical) qualifying key, filtered to keys
     * whose private half is available per [hasPrivateKey].
     *
     * Returns the chosen keyId, or null when no qualifying key exists.
     */
    fun findAuthenticationSigningKeyId(
        keys: List<PublicKeyEntity>,
        minimumSecurityLevel: Int,
        hasPrivateKey: (PublicKeyEntity) -> Boolean,
    ): Int? = keys
        .asSequence()
        .filter { it.disabledAt == null }
        .filter { purposeName(it.purpose) == "AUTHENTICATION" }
        .filter { securityLevelRank(it.securityLevel) <= minimumSecurityLevel }
        // MASTER keys never sign document transitions (DPP forbids it) —
        // exclude rank 0 unless nothing else qualifies is NOT done here;
        // rs-dpp rejects master-signed document transitions outright.
        .filter { securityLevelRank(it.securityLevel) >= 1 }
        .filter(hasPrivateKey)
        .sortedWith(compareBy({ securityLevelRank(it.securityLevel) }, { it.keyId }))
        .firstOrNull()
        ?.keyId
}
