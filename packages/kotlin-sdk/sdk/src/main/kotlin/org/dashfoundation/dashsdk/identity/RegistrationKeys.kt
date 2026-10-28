package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.ffi.IdentityNative

/**
 * Rich registration rows tied to the identity HD slot that produced them.
 *
 * The row wire format deliberately contains only consensus key metadata; the
 * derivation index is host-side provenance and must travel beside the rows so
 * asset-lock resume can verify it still matches the tracked lock.
 */
class RegistrationKeySet(
    val identityIndex: Int,
    rows: List<IdentityPubkey>,
) {
    val rows: List<IdentityPubkey> = rows.toList()

    init {
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(rows.isNotEmpty()) { "registration key rows must not be empty" }
    }
}

/**
 * Which key layout an identity-creation flow registers — the Kotlin mirror of
 * Rust's `platform_wallet::IdentityCreationKeyLayout`. [nativeCode] is the
 * ordinal `IdentityNative.identityCreationKeyLayout` and
 * `IdentityNative.previewRegistrationKeySet` take.
 */
enum class IdentityCreationKeyLayout(val nativeCode: Int) {
    /** A new identity without DashPay keys. */
    NEW_IDENTITY(0),

    /** A new identity with the DashPay ENCRYPTION / DECRYPTION keys. */
    NEW_IDENTITY_WITH_DASHPAY_KEYS(1),

    /**
     * Resuming a registration from a tracked asset lock that cannot fund the
     * DIP-11 key set: the four keys every resume used before the MEDIUM key
     * existed. Rust picks it (`IdentityRegistration.resumeKeyLayout`); it is
     * never chosen host-side.
     */
    ASSET_LOCK_RESUME(2);

    companion object {
        /** The layout for a [nativeCode] returned by the native layer. */
        fun fromNativeCode(code: Int): IdentityCreationKeyLayout =
            requireNotNull(entries.firstOrNull { it.nativeCode == code }) {
                "unknown identity creation key layout code $code"
            }

        /** The layout a fresh registration uses. */
        fun forNewIdentity(includeDashPayKeys: Boolean): IdentityCreationKeyLayout =
            if (includeDashPayKeys) NEW_IDENTITY_WITH_DASHPAY_KEYS else NEW_IDENTITY
    }
}

/**
 * One key of an [IdentityCreationKeyLayout], as Rust defines it: the key id,
 * the DIP-9 key index it is derived at, and the role it is registered with.
 */
data class RegistrationKeySlot(
    val keyId: Int,
    val derivationKeyIndex: Int,
    val keyType: KeyType,
    val purpose: KeyPurpose,
    val securityLevel: SecurityLevel,
    val contractBounds: ContractBounds?,
)

/**
 * The identity-creation key layout, read from Rust. Kotlin decides nothing
 * here: the layout (which keys, which ids, which roles) is owned by
 * `platform_wallet::IdentityCreationKeyLayout` and marshalled through
 * `IdentityNative.identityCreationKeyLayout`. This object decodes it and
 * stamps each slot's role onto the derived public keys.
 *
 * The layouts today (all ECDSA_SECP256K1, DIP-9 key index = key id):
 *
 * | keyId | purpose        | securityLevel | layouts                         |
 * |-------|----------------|---------------|---------------------------------|
 * | 0     | AUTHENTICATION | MASTER        | all                             |
 * | 1     | AUTHENTICATION | CRITICAL      | all                             |
 * | 2     | AUTHENTICATION | HIGH          | all                             |
 * | 3     | TRANSFER       | CRITICAL      | all                             |
 * | 4     | ENCRYPTION     | MEDIUM        | NEW_IDENTITY_WITH_DASHPAY_KEYS  |
 * | 5     | DECRYPTION     | MEDIUM        | NEW_IDENTITY_WITH_DASHPAY_KEYS  |
 * | 4 / 6 | AUTHENTICATION | MEDIUM        | NEW_IDENTITY / …WITH_DASHPAY_KEYS |
 *
 * Rust's write path refuses a new identity without an AUTHENTICATION key at
 * each of MASTER, CRITICAL, HIGH and MEDIUM (DIP-11).
 */
object RegistrationKeys {

    /** Immutable backing bytes for [DASHPAY_CONTRACT_ID] — never handed out directly. */
    private val DASHPAY_CONTRACT_ID_BYTES: ByteArray = byteArrayOf(
        162.toByte(), 161.toByte(), 180.toByte(), 172.toByte(), 111, 239.toByte(), 34, 234.toByte(),
        42, 26, 104, 232.toByte(), 18, 54, 68, 179.toByte(),
        87, 135.toByte(), 95, 107, 65, 44, 24, 16,
        146.toByte(), 129.toByte(), 193.toByte(), 70, 231.toByte(), 178.toByte(), 113, 188.toByte(),
    )

    /**
     * DashPay data-contract id (32 bytes) — source of truth
     * `packages/dashpay-contract/src/lib.rs::ID_BYTES`. Used by the manual
     * Add Identity Key flow; registration takes its bounds from the Rust
     * layout instead. Network-agnostic.
     *
     * Returns a fresh copy on every access — the id is process-global, so
     * handing out the backing array would let a stray write corrupt every
     * DashPay-bound key thereafter.
     */
    val DASHPAY_CONTRACT_ID: ByteArray
        get() = DASHPAY_CONTRACT_ID_BYTES.copyOf()

    /**
     * The DashPay document type the ENCRYPTION / DECRYPTION keys are bound to —
     * the only one in the contract that declares
     * `requiresIdentityEncryptionBoundedKey`.
     */
    const val DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE: String = "contactRequest"

    /** The slots of [layout], from Rust, in key-id order. */
    fun layout(layout: IdentityCreationKeyLayout): List<RegistrationKeySlot> =
        decodeLayout(IdentityNative.identityCreationKeyLayout(layout.nativeCode))

    /** Total keys [layout] registers, for sizing the creation fee. */
    fun keyCount(layout: IdentityCreationKeyLayout): Int = layout(layout).size

    /**
     * Stamp each slot's role onto [publicKeys], producing the rich
     * [IdentityPubkey] rows the registration wire format carries.
     * [publicKeys] must be exactly one per slot, in slot order; each is the
     * on-chain public payload for that slot (the derived compressed pubkey,
     * already persisted to the Keystore by the caller). No private material
     * is touched here.
     *
     * @throws IllegalArgumentException if the count doesn't match the layout.
     */
    fun buildRegistrationRows(
        publicKeys: List<ByteArray>,
        slots: List<RegistrationKeySlot>,
    ): List<IdentityPubkey> {
        require(publicKeys.size == slots.size) {
            "expected ${slots.size} registration public keys, got ${publicKeys.size}"
        }
        return slots.zip(publicKeys) { slot, pubkey ->
            IdentityPubkey(
                keyId = slot.keyId,
                keyType = slot.keyType,
                purpose = slot.purpose,
                securityLevel = slot.securityLevel,
                pubkeyBytes = pubkey,
                contractBounds = slot.contractBounds,
            )
        }
    }

    /**
     * Decode the layout BLOB `IdentityNative.identityCreationKeyLayout`
     * returns (all integers big-endian):
     *
     * ```text
     * u32  row_count
     * repeat row_count times:
     *   u32  key_id
     *   u32  derivation_key_index
     *   u8   key_type, u8 purpose, u8 security_level   (DPP discriminants)
     *   u8   contract_bounds_kind  (0 none, 1 SingleContract, 2 SingleContractDocumentType)
     *   if contract_bounds_kind != 0: u8[32] contract_bounds_id
     *   if contract_bounds_kind == 2: u16 doc_type_len, u8[doc_type_len] doc_type (UTF-8)
     * ```
     *
     * Strict: truncation, trailing bytes and unknown discriminants throw, so
     * a Kotlin artifact paired with a mismatched native library fails loudly.
     *
     * @throws IllegalArgumentException on a malformed BLOB.
     */
    fun decodeLayout(blob: ByteArray): List<RegistrationKeySlot> {
        val buf = java.nio.ByteBuffer.wrap(blob)
        fun need(n: Int, what: String) {
            require(buf.remaining() >= n) { "layout blob truncated at $what" }
        }
        need(4, "row count")
        val count = buf.int
        require(count >= 0) { "layout blob row count must be non-negative, got $count" }
        val slots = ArrayList<RegistrationKeySlot>(minOf(count, 16))
        repeat(count) { i ->
            need(12, "row $i header")
            val keyId = buf.int
            val derivationKeyIndex = buf.int
            val keyTypeByte = buf.get().toInt() and 0xFF
            val purposeByte = buf.get().toInt() and 0xFF
            val levelByte = buf.get().toInt() and 0xFF
            val boundsKind = buf.get().toInt() and 0xFF
            val keyType = requireNotNull(KeyType.entries.firstOrNull { it.ffiValue == keyTypeByte }) {
                "layout blob row $i has unknown key type $keyTypeByte"
            }
            val purpose = requireNotNull(KeyPurpose.entries.firstOrNull { it.ffiValue == purposeByte }) {
                "layout blob row $i has unknown purpose $purposeByte"
            }
            val securityLevel =
                requireNotNull(SecurityLevel.entries.firstOrNull { it.ffiValue == levelByte }) {
                    "layout blob row $i has unknown security level $levelByte"
                }
            val bounds = when (boundsKind) {
                0 -> null
                1 -> {
                    need(32, "row $i contract id")
                    ContractBounds.SingleContract(ByteArray(32).also { buf.get(it) })
                }
                2 -> {
                    need(32 + 2, "row $i contract id")
                    val id = ByteArray(32).also { buf.get(it) }
                    val nameLen = buf.short.toInt() and 0xFFFF
                    need(nameLen, "row $i document type")
                    val name = ByteArray(nameLen).also { buf.get(it) }
                    ContractBounds.SingleContractDocumentType(id, String(name, Charsets.UTF_8))
                }
                else -> throw IllegalArgumentException(
                    "layout blob row $i has unknown contract bounds kind $boundsKind",
                )
            }
            slots += RegistrationKeySlot(
                keyId = keyId,
                derivationKeyIndex = derivationKeyIndex,
                keyType = keyType,
                purpose = purpose,
                securityLevel = securityLevel,
                contractBounds = bounds,
            )
        }
        require(!buf.hasRemaining()) { "layout blob has ${buf.remaining()} trailing bytes" }
        return slots
    }
}
