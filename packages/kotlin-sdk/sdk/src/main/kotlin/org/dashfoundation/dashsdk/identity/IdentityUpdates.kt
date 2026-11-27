package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.wallet.op
import org.dashfoundation.dashsdk.wallet.opReturningCompletedResult

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.IdentityKeyLimitsNative
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
 * Contract bounds of an identity public key: Kotlin mirror of the rs-dpp
 * `ContractBounds` enum (and of Swift's `ManagedPlatformWallet.ContractBounds`).
 * Drive requires them on ENCRYPTION / DECRYPTION keys. AUTHENTICATION keys
 * may carry them since protocol version 14, where they limit what the key
 * can sign. Null means unbounded.
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

    /**
     * Bind an AUTHENTICATION key to a contract group: the key signs only
     * within the group's members. [contractGroupId] is the 32-byte contract
     * group id; there is never a document type.
     */
    data class ContractGroup(val contractGroupId: ByteArray) : ContractBounds() {
        init {
            require(contractGroupId.size == 32) {
                "contractGroupId must be 32 bytes, got ${contractGroupId.size}"
            }
        }

        override fun equals(other: Any?): Boolean =
            other is ContractGroup && contractGroupId.contentEquals(other.contractGroupId)

        override fun hashCode(): Int = contractGroupId.contentHashCode()
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
 * @property totalBudget usage limit (protocol version 14): the credits the key may
 *   take from the identity over its lifetime. Only AUTHENTICATION keys below
 *   MASTER may carry limits; either limit registers a version 1 key.
 * @property expiresAt usage limit (protocol version 14): the block time in
 *   milliseconds from which the key can no longer sign.
 */
data class IdentityPubkey(
    val keyId: Int,
    val keyType: KeyType,
    val purpose: KeyPurpose,
    val securityLevel: SecurityLevel,
    val pubkeyBytes: ByteArray,
    val readOnly: Boolean = false,
    val contractBounds: ContractBounds? = null,
    val totalBudget: Long? = null,
    val expiresAt: Long? = null,
) {
    init {
        require(keyId >= 0) { "keyId must be non-negative, got $keyId" }
        require(totalBudget == null || totalBudget >= 0) {
            "totalBudget must be non-negative, got $totalBudget"
        }
        require(expiresAt == null || expiresAt >= 0) {
            "expiresAt must be non-negative, got $expiresAt"
        }
    }

    /** Whether the key carries a budget or an expiry (a version 1 key). */
    val hasLimits: Boolean get() = totalBudget != null || expiresAt != null

    override fun equals(other: Any?): Boolean =
        other is IdentityPubkey &&
            keyId == other.keyId &&
            keyType == other.keyType &&
            purpose == other.purpose &&
            securityLevel == other.securityLevel &&
            pubkeyBytes.contentEquals(other.pubkeyBytes) &&
            readOnly == other.readOnly &&
            contractBounds == other.contractBounds &&
            totalBudget == other.totalBudget &&
            expiresAt == other.expiresAt

    override fun hashCode(): Int {
        var result = keyId
        result = 31 * result + keyType.hashCode()
        result = 31 * result + purpose.hashCode()
        result = 31 * result + securityLevel.hashCode()
        result = 31 * result + pubkeyBytes.contentHashCode()
        result = 31 * result + readOnly.hashCode()
        result = 31 * result + (contractBounds?.hashCode() ?: 0)
        result = 31 * result + (totalBudget?.hashCode() ?: 0)
        result = 31 * result + (expiresAt?.hashCode() ?: 0)
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
    /**
     * The owning manager's signer handle, read inside [gate] by the key limits calls: the
     * manager closes that signer only after the gate has drained, so a call holding the gate
     * can never borrow a freed signer. Null for a facade without a manager.
     */
    private val managerSignerHandle: (() -> Long)? = null,
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

    /**
     * Raise the limits of key [keyId] of [identityId] (protocol version 14):
     * add [addBudget] credits to its total budget (and to what is left of
     * it), and/or move its expiry to [expiresAt] (block time in
     * milliseconds). At least one must be given. Bridges
     * `platform_wallet_update_identity_key_limits_with_signer`, signed via
     * [signerHandle] with the identity's MASTER key or a CRITICAL
     * authentication key without limits and without contract bounds. A
     * limit the key does not have, a zero top-up and an expiry that is not
     * later are refused before anything is signed. Room learns of the raised
     * limits through the persistence changeset, as it does for an added key.
     */
    suspend fun updateKeyLimits(
        walletHandle: Long,
        identityId: ByteArray,
        keyId: Int,
        addBudget: Long? = null,
        expiresAt: Long? = null,
        signerHandle: Long,
    ) = gate.op {
        require(identityId.size == 32) {
            "identityId must be 32 bytes, got ${identityId.size}"
        }
        require(keyId >= 0) { "keyId must be non-negative, got $keyId" }
        require(addBudget != null || expiresAt != null) {
            "updateKeyLimits needs a budget to add or a new expiry"
        }
        require(addBudget == null || addBudget > 0) {
            "addBudget must be positive, got $addBudget"
        }
        require(expiresAt == null || expiresAt >= 0) {
            "expiresAt must be non-negative, got $expiresAt"
        }
        mapNativeErrors {
            TransactionsNative.updateIdentityKeyLimits(
                walletHandle,
                identityId,
                keyId,
                addBudget != null,
                addBudget ?: 0L,
                expiresAt != null,
                expiresAt ?: 0L,
                signerHandle,
            )
        }
    }

    /**
     * What raising key [keyId] of [identityId] would do, signing and sending nothing: port
     * of Swift's `ManagedPlatformWallet.previewIdentityKeyLimitsUpdate(identityId:keyId:
     * addBudget:expiresAt:signer:)`, bridging
     * `platform_wallet_preview_identity_key_limits_update`.
     *
     * With neither [addBudget] (credits) nor [expiresAt] (block time in milliseconds), Rust
     * answers whether the key can be raised at all and by which key; with a change it also
     * returns the absolute limits the update would set. Every rule is Rust's: a missing,
     * disabled or unlimited key, a change that is not a raise (a zero top-up, an expiry that
     * is not later), a key the change would leave expired, and a manager signer holding no
     * MASTER key and no CRITICAL authentication key without limits or contract bounds are
     * refused with Rust's reason as a [org.dashfoundation.dashsdk.errors.DashSdkError].
     * The answer comes from the identity as the wallet at [walletHandle] holds it; call
     * [refreshIdentity] first to start from Platform.
     *
     * Signed by the owning manager's signer, borrowed inside the teardown gate (not a handle
     * the caller passes, which another owner could close mid-call). [identityId] is copied
     * once before the call, so the returned preview names exactly the identity Rust answered
     * for even if the caller reuses its buffer.
     */
    suspend fun previewKeyLimitsUpdate(
        walletHandle: Long,
        identityId: ByteArray,
        keyId: Int,
        addBudget: ULong? = null,
        expiresAt: ULong? = null,
    ): IdentityKeyLimitsUpdatePreview {
        val id = identityId.copyOf()
        require(id.size == 32) { "identityId must be 32 bytes, got ${id.size}" }
        return gate.op {
            val slots = mapNativeErrors {
                IdentityKeyLimitsNative.previewKeyLimitsUpdate(
                    walletHandle,
                    id,
                    keyId,
                    addBudget != null,
                    (addBudget ?: 0uL).toLong(),
                    expiresAt != null,
                    (expiresAt ?: 0uL).toLong(),
                    ownSignerHandle(),
                )
            }
            IdentityKeyLimitsUpdatePreview.fromSlots(id, keyId, slots)
        }
    }

    /**
     * Raise the limits of the key [reviewed] names to exactly what it shows: port of Swift's
     * `ManagedPlatformWallet.updateIdentityKeyLimits(asReviewed:signer:)`, bridging
     * `platform_wallet_update_identity_key_limits_as_reviewed_with_signer`.
     *
     * Rust signs the reviewed total, expiry and signing key, or refuses before a nonce is
     * taken when the wallet's copy moved since the review. A
     * [org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed]
     * means the update may have landed; do not send it again. Room learns of the raised
     * limits through the persistence changeset.
     *
     * Signed by the owning manager's signer, borrowed inside the teardown gate. If the caller
     * is cancelled after the native call completed, the call still returns normally instead
     * of throwing (see `opReturningCompletedResult`), so a landed update is never reported as
     * a cancellation that a caller might send again.
     */
    suspend fun updateKeyLimitsAsReviewed(
        walletHandle: Long,
        reviewed: IdentityKeyLimitsUpdatePreview,
    ) {
        val identityId = reviewed.identityId
        val slots = reviewed.toSlots()
        gate.opReturningCompletedResult {
            mapNativeErrors {
                IdentityKeyLimitsNative.updateKeyLimitsAsReviewed(
                    walletHandle,
                    identityId,
                    reviewed.keyId,
                    slots,
                    ownSignerHandle(),
                )
            }
        }
    }

    /**
     * Fetch [identityId] from Platform with a proof and fold it into the wallet at
     * [walletHandle]: port of Swift's `ManagedPlatformWallet.refreshIdentity(identityId:)`,
     * bridging `platform_wallet_refresh_identity`. Keys added or raised on another client
     * are taken in, and a node behind the wallet's copy cannot roll it back. Changed key rows
     * reach Room through the persister. A failed store is thrown and leaves the wallet's
     * copy as it was, so a retry still includes those keys: success means the wallet's copy
     * is current and the rows were stored.
     */
    suspend fun refreshIdentity(walletHandle: Long, identityId: ByteArray) {
        val id = identityId.copyOf()
        require(id.size == 32) { "identityId must be 32 bytes, got ${id.size}" }
        gate.op { mapNativeErrors { IdentityKeyLimitsNative.refreshIdentity(walletHandle, id) } }
    }

    /** The manager's signer handle; call only inside [gate]. */
    private fun ownSignerHandle(): Long =
        checkNotNull(managerSignerHandle) { "this IdentityUpdates has no manager signer" }()
}
