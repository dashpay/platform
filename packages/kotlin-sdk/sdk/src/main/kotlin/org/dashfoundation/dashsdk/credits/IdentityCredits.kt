package org.dashfoundation.dashsdk.credits

import org.dashfoundation.dashsdk.wallet.op

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.CreditsNative
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock

internal fun interface ResumeTopUpNativeCall {
    fun call(
        walletHandle: Long,
        outpointTxid: ByteArray,
        outpointVout: Int,
        identityId: ByteArray,
        coreSignerHandle: Long,
        consumeInvitationVoucher: Boolean,
    ): Long
}

/**
 * One Platform-address funding input for an identity top-up — port of the
 * `IdentityAddressInput` tuple the Swift `TopUpIdentityView` packs. Encodes
 * to the flat blob [CreditsNative.topUpFromAddresses] consumes.
 *
 * @property addressType 0 = P2PKH, 1 = P2SH.
 * @property hash the 20-byte address hash.
 * @property credits credits to spend from this address.
 */
data class FundingInput(
    val addressType: Int,
    val hash: ByteArray,
    val credits: Long,
) {
    init {
        require(hash.size == 20) { "FundingInput.hash must be 20 bytes, got ${hash.size}" }
        require(credits > 0) { "FundingInput.credits must be positive, got $credits" }
    }

    override fun equals(other: Any?): Boolean =
        other is FundingInput &&
            addressType == other.addressType &&
            hash.contentEquals(other.hash) &&
            credits == other.credits

    override fun hashCode(): Int =
        (31 * addressType + hash.contentHashCode()) * 31 + credits.hashCode()

    companion object {
        /**
         * Serialize [inputs] to the big-endian blob the top-up FFI reads:
         * `u32 rowCount` then per row `u8 addressType, u8[20] hash, u64 credits`.
         */
        fun encode(inputs: List<FundingInput>): ByteArray {
            val out = java.io.ByteArrayOutputStream()
            val dos = java.io.DataOutputStream(out)
            dos.writeInt(inputs.size) // u32 big-endian
            for (input in inputs) {
                dos.writeByte(input.addressType)
                dos.write(input.hash)
                dos.writeLong(input.credits) // u64 big-endian
            }
            return out.toByteArray()
        }
    }
}

/**
 * Thin wrapper over the credits JNI surface (`CreditsNative`) — port of
 * the credit-movement slice of `ManagedPlatformWallet.swift`
 * (`transferCredits`, `withdrawCredits`, `topUpFromAddresses`).
 *
 * Every method is a single FFI call marshalled onto [Dispatchers.IO] and
 * routed through [mapNativeErrors]; no orchestration lives here (see
 * `packages/kotlin-sdk/CLAUDE.md`). The handles are supplied by the caller
 * — `PlatformWalletManager` owns the `signerHandle`, `ManagedPlatformWallet`
 * owns the wallet handle.
 */
class IdentityCredits internal constructor(
    private val gate: org.dashfoundation.dashsdk.wallet.TeardownGate? = null,
    private val resumeTopUpNative: ResumeTopUpNativeCall =
        ResumeTopUpNativeCall(CreditsNative::topUpIdentityWithExistingAssetLock),
) {

    /**
     * Resume a top-up from [lock]'s exact persisted outpoint. Rust remains
     * authoritative for rebroadcast, proof acquisition, and consumption;
     * Kotlin never creates a second funding transaction. Invitation locks
     * are rejected and generic recovery always passes `false` for voucher
     * consumption.
     */
    suspend fun resumeTopUpWithExistingAssetLock(
        walletHandle: Long,
        identityId: ByteArray,
        lock: TrackedAssetLock,
        coreSignerHandle: Long,
    ): Long = gate.op {
        require(identityId.size == 32) {
            "identityId must be exactly 32 bytes, got ${identityId.size}"
        }
        require(
            lock.fundingType == TrackedAssetLock.FundingType.IDENTITY_TOP_UP ||
                lock.fundingType == TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND,
        ) {
            "top-up recovery requires funding type 1 or 2, got ${lock.fundingType.raw}"
        }
        mapNativeErrors {
            resumeTopUpNative.call(
                walletHandle = walletHandle,
                outpointTxid = lock.outpointTxid,
                outpointVout = lock.outpointVout,
                identityId = identityId,
                coreSignerHandle = coreSignerHandle,
                consumeInvitationVoucher = false,
            )
        }
    }

    /**
     * Reclaim an unclaimed DIP-13 invitation voucher as credits on an
     * existing identity — the "top-up" reclaim target. Consumes the
     * voucher's asset lock (the DASH was OP_RETURN-burned at create; the
     * value returns as identity **credits**, never L1 Dash). The invitee
     * can no longer claim afterwards. ← Swift
     * `ManagedPlatformWallet.resumeTopUpWithAssetLock(consumeInvitationVoucher: true)`.
     *
     * Takes the raw outpoint (from the invitation row's `rawOutPoint` —
     * txid wire order + vout), not a [TrackedAssetLock]: invitation locks
     * are deliberately excluded from the generic tracked-lock recovery
     * model. `consumeInvitationVoucher = true` is the explicit
     * authorization Rust core requires to consume an invitation-typed
     * lock; this is one of the two call sites in the SDK that pass it.
     *
     * A voucher already claimed (or already reclaimed) is rejected
     * deterministically by Platform — the caller classifies that failure
     * (see the reclaim sheet's outcome classifier).
     *
     * @return the identity's new credit balance.
     */
    suspend fun reclaimInvitationAsTopUp(
        walletHandle: Long,
        identityId: ByteArray,
        outPointTxid: ByteArray,
        outPointVout: Int,
        coreSignerHandle: Long,
    ): Long = gate.op {
        require(identityId.size == 32) {
            "identityId must be exactly 32 bytes, got ${identityId.size}"
        }
        require(outPointTxid.size == 32) {
            "outPointTxid must be exactly 32 bytes, got ${outPointTxid.size}"
        }
        require(outPointVout >= 0) { "outPointVout must be non-negative, got $outPointVout" }
        mapNativeErrors {
            resumeTopUpNative.call(
                walletHandle = walletHandle,
                outpointTxid = outPointTxid,
                outpointVout = outPointVout,
                identityId = identityId,
                coreSignerHandle = coreSignerHandle,
                consumeInvitationVoucher = true,
            )
        }
    }

    /**
     * Transfer [amount] credits from [fromIdentityId] to [toIdentityId]
     * (both 32 bytes), signed via [signerHandle].
     */
    suspend fun transfer(
        walletHandle: Long,
        fromIdentityId: ByteArray,
        toIdentityId: ByteArray,
        amount: Long,
        signerHandle: Long,
    ) = gate.op {
        require(amount > 0) { "amount must be positive, got $amount" }
        mapNativeErrors {
            CreditsNative.transferCredits(
                walletHandle,
                fromIdentityId,
                toIdentityId,
                amount,
                signerHandle,
            )
        }
    }

    /**
     * Withdraw [amount] credits from [identityId] (32 bytes) to the
     * Base58Check Dash address [toAddress], signed via [signerHandle].
     */
    suspend fun withdraw(
        walletHandle: Long,
        identityId: ByteArray,
        amount: Long,
        toAddress: String,
        signerHandle: Long,
    ) = gate.op {
        require(amount > 0) { "amount must be positive, got $amount" }
        mapNativeErrors {
            CreditsNative.withdrawCredits(
                walletHandle,
                identityId,
                amount,
                toAddress,
                signerHandle,
            )
        }
    }

    /**
     * Top up [identityId] (32 bytes) from Platform-address [inputs],
     * signed via [signerHandle]. Returns the post-transition balance.
     */
    suspend fun topUpFromAddresses(
        walletHandle: Long,
        identityId: ByteArray,
        inputs: List<FundingInput>,
        signerHandle: Long,
    ): Long = gate.op {
        mapNativeErrors {
            CreditsNative.topUpFromAddresses(
                walletHandle,
                identityId,
                FundingInput.encode(inputs),
                signerHandle,
            )
        }
    }

    /**
     * Transfer credits from [fromIdentityId] (32 bytes) to one or more
     * Platform-address recipients ([outputs]), signed by the identity's
     * transfer key via [signerHandle] — the ID-11 path. Each output's
     * `credits` is the amount routed to that recipient address (the
     * [FundingInput] row shape is reused since it is the identical
     * `addressType / hash / credits` triple the FFI marshals). Returns the
     * sender's post-transfer credit balance.
     */
    suspend fun transferToAddresses(
        walletHandle: Long,
        fromIdentityId: ByteArray,
        outputs: List<FundingInput>,
        signerHandle: Long,
    ): Long = gate.op {
        require(outputs.isNotEmpty()) { "outputs must not be empty" }
        mapNativeErrors {
            CreditsNative.transferCreditsToAddresses(
                walletHandle,
                fromIdentityId,
                FundingInput.encode(outputs),
                signerHandle,
            )
        }
    }

    /**
     * Top up [identityId] (32 bytes) by building + broadcasting a **new
     * Core asset lock** — the ID-05 funding path (same mechanism as
     * identity registration), distinct from [topUpFromAddresses] (ID-06).
     * [amountDuffs] is the Dash amount in duffs to lock; [accountIndex]
     * selects the BIP44 standard account; [coreSignerHandle] is the
     * manager's `MnemonicResolverHandle`. Returns the post-transition
     * balance. Port of the create-identity funding path applied to an
     * existing identity.
     */
    suspend fun topUpFromCore(
        walletHandle: Long,
        identityId: ByteArray,
        amountDuffs: Long,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): Long = gate.op {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        require(accountIndex >= 0) { "accountIndex must be non-negative, got $accountIndex" }
        mapNativeErrors {
            CreditsNative.topUpIdentityFromCore(
                walletHandle,
                identityId,
                amountDuffs,
                accountIndex,
                coreSignerHandle,
            )
        }
    }
}
