package org.dashfoundation.example.services

/**
 * Room `public_keys` rows store `purpose` / `securityLevel` / `keyType`
 * as the *stringified DPP rawValue* (the persister writes
 * `(purpose.toInt() and 0xFF).toString()`, mirroring Swift's
 * `String(purpose.rawValue)`), while hand-written code and the iOS UI
 * compare against the enum *names*. These helpers normalize either form
 * to the canonical DPP name so gates and labels behave identically for
 * both encodings. Document signing keys are not picked here: the Rust
 * wallet selects them through the signer.
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

    /** Display names in rawValue order, ← Swift `KeyPurpose.name`. */
    private val PURPOSE_LABELS = listOf(
        "Authentication", "Encryption", "Decryption", "Transfer", "System", "Voting", "Owner",
    )

    /** Display names in rawValue order, ← Swift `SecurityLevel.name`. */
    private val SECURITY_LEVEL_LABELS = listOf("Master", "Critical", "High", "Medium")

    /** Display names in rawValue order, ← Swift `KeyType.name`. */
    private val KEY_TYPE_LABELS = listOf(
        "ECDSA secp256k1", "BLS12-381", "ECDSA Hash160", "BIP13 Script Hash", "EdDSA 25519 Hash160",
    )

    /**
     * Display name of a raw Room purpose ("0" or "AUTHENTICATION" →
     * "Authentication"); an unknown value reads "Purpose N", as Swift's
     * `WalletKeyHealthSheet` shows it.
     */
    fun purposeLabel(raw: String): String = label(raw, PURPOSE_NAMES, PURPOSE_LABELS, "Purpose")

    /** Display name of a raw Room security level; an unknown value reads "Level N". */
    fun securityLevelLabel(raw: String): String =
        label(raw, SECURITY_LEVEL_NAMES, SECURITY_LEVEL_LABELS, "Level")

    /** Display name of a raw Room key type; an unknown value reads "Type N". */
    fun keyTypeLabel(raw: String): String = label(raw, KEY_TYPE_NAMES, KEY_TYPE_LABELS, "Type")

    private fun label(raw: String, names: List<String>, labels: List<String>, unknown: String): String {
        val index = names.indexOf(normalize(raw, names))
        return labels.getOrNull(index) ?: "$unknown ${raw.trim()}"
    }

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
}
