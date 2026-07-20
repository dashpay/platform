package org.dashfoundation.dashsdk.identity

/**
 * The canonical DPP-role layout for a freshly-registered identity's keys —
 * the Kotlin source of truth that replaces the retired Rust
 * `role_for_registration_key_id`. Since the registration wire format now
 * carries each key's full role (see [IdentityPubkeyCodec]) instead of
 * reconstructing it positionally on the Rust side, the layout is stamped here
 * and shipped over the boundary.
 *
 * Byte-for-byte identical to the iOS reference (SwiftExampleApp's
 * `CreateIdentityView.defaultKeyCount` role table + `makeDashpayKeyPair`):
 *
 * | keyId | keyType         | purpose        | securityLevel | bounds                    |
 * |-------|-----------------|----------------|---------------|---------------------------|
 * | 0     | ECDSA_SECP256K1 | AUTHENTICATION | MASTER        | —                         |
 * | 1     | ECDSA_SECP256K1 | AUTHENTICATION | CRITICAL      | —                         |
 * | 2     | ECDSA_SECP256K1 | AUTHENTICATION | HIGH          | —                         |
 * | 3     | ECDSA_SECP256K1 | TRANSFER       | CRITICAL      | —                         |
 * | 4     | ECDSA_SECP256K1 | ENCRYPTION     | MEDIUM        | DashPay / contactRequest  |
 * | 5     | ECDSA_SECP256K1 | DECRYPTION     | MEDIUM        | DashPay / contactRequest  |
 *
 * - keyId 0 (MASTER/AUTH) signs the IdentityCreate / IdentityUpdate transition.
 * - keyId 1 (CRITICAL/AUTH) signs token state transitions — without it the
 *   identity can't mint / burn / freeze tokens.
 * - keyId 2 (HIGH/AUTH) signs general document / DPNS / contract transitions.
 * - keyId 3 (TRANSFER/CRITICAL) signs IdentityCreditTransfer / …Withdrawal —
 *   without it those broadcasts are rejected on-chain with "no transfer public
 *   key".
 * - keyId 4 (ENCRYPTION) + keyId 5 (DECRYPTION), bound to DashPay's
 *   `contactRequest` document type, let the new identity send / receive
 *   contact requests. Without them the app's own Add Contact flow rejects the
 *   identity: `select_own_encryption_key` requires an enabled ECDSA
 *   ENCRYPTION key. These two are appended only on fresh-funding registration,
 *   not on asset-lock resume (which never grows past the key set it originally
 *   committed to on-chain).
 */
object RegistrationKeys {

    /** Base auth/transfer key count (keyIds 0..3), every registration path. */
    const val BASE_KEY_COUNT: Int = 4

    /** Extra DashPay ENCRYPTION + DECRYPTION keys (keyIds 4..5). */
    const val DASHPAY_KEY_COUNT: Int = 2

    /** Immutable backing bytes for [DASHPAY_CONTRACT_ID] — never handed out directly. */
    private val DASHPAY_CONTRACT_ID_BYTES: ByteArray = byteArrayOf(
        162.toByte(), 161.toByte(), 180.toByte(), 172.toByte(), 111, 239.toByte(), 34, 234.toByte(),
        42, 26, 104, 232.toByte(), 18, 54, 68, 179.toByte(),
        87, 135.toByte(), 95, 107, 65, 44, 24, 16,
        146.toByte(), 129.toByte(), 193.toByte(), 70, 231.toByte(), 178.toByte(), 113, 188.toByte(),
    )

    /**
     * DashPay data-contract id (32 bytes) — source of truth
     * `packages/dashpay-contract/src/lib.rs::ID_BYTES`. Mirrored here so the
     * contract-bounds payload for the ENCRYPTION / DECRYPTION keys can be built
     * without a contract fetch; pinned against the Rust constant by the
     * cross-language golden fixture in the tests. Network-agnostic.
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

    /** Total keys a registration derives for the given DashPay choice. */
    fun keyCount(includeDashPayKeys: Boolean): Int =
        BASE_KEY_COUNT + if (includeDashPayKeys) DASHPAY_KEY_COUNT else 0

    /**
     * Stamp the canonical DPP roles onto [publicKeys], producing the rich
     * [IdentityPubkey] rows the registration wire format carries. [publicKeys]
     * must be exactly [keyCount] entries in keyId order (index i → keyId i);
     * each is the on-chain public payload for that slot (the derived
     * compressed pubkey, already persisted to the Keystore by the caller). No
     * private material is touched here.
     *
     * @throws IllegalArgumentException if the count doesn't match the policy.
     */
    fun buildRegistrationRows(
        publicKeys: List<ByteArray>,
        includeDashPayKeys: Boolean,
    ): List<IdentityPubkey> {
        val expected = keyCount(includeDashPayKeys)
        require(publicKeys.size == expected) {
            "expected $expected registration public keys, got ${publicKeys.size}"
        }
        return publicKeys.mapIndexed { keyId, pubkey -> rowFor(keyId, pubkey) }
    }

    /** The DashPay contact-request contract-document bounds for the enc/dec keys. */
    private fun dashPayBounds(): ContractBounds =
        ContractBounds.SingleContractDocumentType(
            // The accessor already returns a fresh copy the bounds object owns.
            contractId = DASHPAY_CONTRACT_ID,
            documentTypeName = DASHPAY_CONTACT_REQUEST_DOCUMENT_TYPE,
        )

    private fun rowFor(keyId: Int, pubkey: ByteArray): IdentityPubkey = when (keyId) {
        0 -> IdentityPubkey(
            keyId = 0,
            keyType = KeyType.ECDSA_SECP256K1,
            purpose = KeyPurpose.AUTHENTICATION,
            securityLevel = SecurityLevel.MASTER,
            pubkeyBytes = pubkey,
        )
        1 -> IdentityPubkey(
            keyId = 1,
            keyType = KeyType.ECDSA_SECP256K1,
            purpose = KeyPurpose.AUTHENTICATION,
            securityLevel = SecurityLevel.CRITICAL,
            pubkeyBytes = pubkey,
        )
        2 -> IdentityPubkey(
            keyId = 2,
            keyType = KeyType.ECDSA_SECP256K1,
            purpose = KeyPurpose.AUTHENTICATION,
            securityLevel = SecurityLevel.HIGH,
            pubkeyBytes = pubkey,
        )
        3 -> IdentityPubkey(
            keyId = 3,
            keyType = KeyType.ECDSA_SECP256K1,
            purpose = KeyPurpose.TRANSFER,
            securityLevel = SecurityLevel.CRITICAL,
            pubkeyBytes = pubkey,
        )
        4 -> IdentityPubkey(
            keyId = 4,
            keyType = KeyType.ECDSA_SECP256K1,
            purpose = KeyPurpose.ENCRYPTION,
            securityLevel = SecurityLevel.MEDIUM,
            pubkeyBytes = pubkey,
            contractBounds = dashPayBounds(),
        )
        5 -> IdentityPubkey(
            keyId = 5,
            keyType = KeyType.ECDSA_SECP256K1,
            purpose = KeyPurpose.DECRYPTION,
            securityLevel = SecurityLevel.MEDIUM,
            pubkeyBytes = pubkey,
            contractBounds = dashPayBounds(),
        )
        // Unreachable: buildRegistrationRows caps the count at keyCount(true) = 6.
        else -> error("no registration role defined for keyId $keyId")
    }
}
