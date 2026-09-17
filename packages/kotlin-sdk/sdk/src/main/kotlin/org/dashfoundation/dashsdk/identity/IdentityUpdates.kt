package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.wallet.op

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.TransactionsNative

/**
 * DPP identity public-key type — Kotlin mirror of the Swift `KeyType`
 * (`SwiftDashSDK/DPP/DPPIdentity.swift`). [ffiValue] discriminants match the
 * rs-dpp `KeyType` enum.
 */
enum class KeyType(val ffiValue: Int) {
    ECDSA_SECP256K1(0),
    BLS12_381(1),
    ECDSA_HASH160(2),
    BIP13_SCRIPT_HASH(3),
    EDDSA_25519_HASH160(4),
}

/**
 * DPP identity key purpose — Kotlin mirror of the Swift `KeyPurpose`.
 * [ffiValue] discriminants match the rs-dpp `Purpose` enum.
 */
enum class KeyPurpose(val ffiValue: Int) {
    AUTHENTICATION(0),
    ENCRYPTION(1),
    DECRYPTION(2),
    TRANSFER(3),
    SYSTEM(4),
    VOTING(5),
    OWNER(6),
}

/**
 * DPP identity key security level — Kotlin mirror of the Swift
 * `SecurityLevel`. [ffiValue] discriminants match the rs-dpp `SecurityLevel`
 * enum.
 */
enum class SecurityLevel(val ffiValue: Int) {
    MASTER(0),
    CRITICAL(1),
    HIGH(2),
    MEDIUM(3),
}

/**
 * Contract-bounds shape for an ENCRYPTION / DECRYPTION key — Kotlin mirror
 * of Swift's `ManagedPlatformWallet.ContractBounds`. Required by Drive for
 * those purposes; omitted (null) for AUTHENTICATION / TRANSFER.
 */
sealed class ContractBounds {
    /** Bind the key to a single contract (any of its document types). */
    data class SingleContract(val contractId: ByteArray) : ContractBounds() {
        init {
            require(contractId.size == 32) {
                "contractId must be 32 bytes, got ${contractId.size}"
            }
        }

        override fun equals(other: Any?): Boolean =
            other is SingleContract && contractId.contentEquals(other.contractId)

        override fun hashCode(): Int = contractId.contentHashCode()
    }

    /** Bind the key to a single contract + a specific document type. */
    data class SingleContractDocumentType(
        val contractId: ByteArray,
        val documentTypeName: String,
    ) : ContractBounds() {
        init {
            require(contractId.size == 32) {
                "contractId must be 32 bytes, got ${contractId.size}"
            }
        }

        override fun equals(other: Any?): Boolean =
            other is SingleContractDocumentType &&
                contractId.contentEquals(other.contractId) &&
                documentTypeName == other.documentTypeName

        override fun hashCode(): Int =
            31 * contractId.contentHashCode() + documentTypeName.hashCode()
    }
}

/**
 * One public key to add to an identity — Kotlin mirror of Swift's
 * `ManagedPlatformWallet.IdentityPubkey` (built by
 * `AddIdentityKeyView.submit` / `IdentityKeyAddition.prepareKeys`). The
 * private half must already be derived + persisted to the Keystore BEFORE
 * the update is submitted (the signer only signs the update transition with
 * an existing MASTER key).
 *
 * @property pubkeyBytes the on-chain public payload — the 33-byte compressed
 *   pubkey, or the 20-byte HASH160 for an [KeyType.ECDSA_HASH160] key.
 */
data class IdentityPubkey(
    val keyId: Int,
    val keyType: KeyType,
    val purpose: KeyPurpose,
    val securityLevel: SecurityLevel,
    val pubkeyBytes: ByteArray,
    val readOnly: Boolean = false,
    val contractBounds: ContractBounds? = null,
) {
    init {
        require(keyId >= 0) { "keyId must be non-negative, got $keyId" }
    }

    override fun equals(other: Any?): Boolean =
        other is IdentityPubkey &&
            keyId == other.keyId &&
            keyType == other.keyType &&
            purpose == other.purpose &&
            securityLevel == other.securityLevel &&
            pubkeyBytes.contentEquals(other.pubkeyBytes) &&
            readOnly == other.readOnly &&
            contractBounds == other.contractBounds

    override fun hashCode(): Int {
        var result = keyId
        result = 31 * result + keyType.hashCode()
        result = 31 * result + purpose.hashCode()
        result = 31 * result + securityLevel.hashCode()
        result = 31 * result + pubkeyBytes.contentHashCode()
        result = 31 * result + readOnly.hashCode()
        result = 31 * result + (contractBounds?.hashCode() ?: 0)
        return result
    }
}

/**
 * Identity add/disable-keys bridge — port of the identity-update slice of
 * `ManagedPlatformWallet.swift` (`updateIdentity(addPublicKeys:...)`, driven
 * by `AddIdentityKeyView`). Thin wrapper over [TransactionsNative]; no
 * orchestration lives here (see `packages/kotlin-sdk/CLAUDE.md`). Handles are
 * supplied by the caller — `PlatformWalletManager` owns the [signerHandle],
 * `ManagedPlatformWallet` owns the wallet handle.
 */
class IdentityUpdates internal constructor(
    private val gate: org.dashfoundation.dashsdk.wallet.TeardownGate? = null,
) {

    /**
     * Add a single public key to [identityId], signed via [signerHandle].
     * Convenience over [update] for the single-key `AddIdentityKeyView` flow.
     */
    suspend fun addKey(
        walletHandle: Long,
        identityId: ByteArray,
        key: IdentityPubkey,
        signerHandle: Long,
    ) = update(walletHandle, identityId, listOf(key), emptyList(), signerHandle)

    /**
     * Disable existing key ids on [identityId], signed via [signerHandle].
     */
    suspend fun disableKeys(
        walletHandle: Long,
        identityId: ByteArray,
        keyIds: List<Int>,
        signerHandle: Long,
    ) = update(walletHandle, identityId, emptyList(), keyIds, signerHandle)

    /**
     * Add [addPublicKeys] and/or disable [disablePublicKeyIds] on
     * [identityId] in a single `IdentityUpdateTransition`, signed via
     * [signerHandle]. At least one of add / disable must be non-empty.
     */
    suspend fun update(
        walletHandle: Long,
        identityId: ByteArray,
        addPublicKeys: List<IdentityPubkey> = emptyList(),
        disablePublicKeyIds: List<Int> = emptyList(),
        signerHandle: Long,
    ) = gate.op {
        require(identityId.size == 32) {
            "identityId must be 32 bytes, got ${identityId.size}"
        }
        require(addPublicKeys.isNotEmpty() || disablePublicKeyIds.isNotEmpty()) {
            "updateIdentity needs at least one key to add or disable"
        }
        require(disablePublicKeyIds.all { it >= 0 }) {
            "every disabled key id must be non-negative, got $disablePublicKeyIds"
        }
        mapNativeErrors {
            TransactionsNative.updateIdentity(
                walletHandle,
                identityId,
                IdentityPubkeyCodec.encode(addPublicKeys),
                disablePublicKeyIds.toIntArray(),
                signerHandle,
            )
        }
    }
}
