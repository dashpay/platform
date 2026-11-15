package org.dashfoundation.dashsdk.credits

import org.dashfoundation.dashsdk.wallet.op
import org.dashfoundation.dashsdk.wallet.opReturningCompletedResult

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.CreditsNative
import org.dashfoundation.dashsdk.ffi.IdentityTopUpNativeResult
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock

/**
 * The JNI calls behind [IdentityCredits]' top-ups that resume or pay through
 * an asset lock, as one seam so unit tests can stand in for the native
 * library, which Robolectric cannot load. Production uses [Jni].
 */
internal interface TopUpNative {
    fun topUpIdentityWithExistingAssetLock(
        walletHandle: Long,
        outpointTxid: ByteArray,
        outpointVout: Int,
        identityId: ByteArray,
        coreSignerHandle: Long,
        consumeInvitationVoucher: Boolean,
    ): Long

    fun topUpExternalIdentityFromCore(
        walletHandle: Long,
        identityId: ByteArray,
        amountDuffs: Long,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): Long

    fun resumeTopUpToRecordedRecipient(
        walletHandle: Long,
        outpointTxid: ByteArray,
        outpointVout: Int,
        coreSignerHandle: Long,
    ): IdentityTopUpNativeResult

    fun assignTopUpRecipient(
        walletHandle: Long,
        outpointTxid: ByteArray,
        outpointVout: Int,
        identityId: ByteArray,
    )

    /** [CreditsNative]'s exports. */
    object Jni : TopUpNative {
        override fun topUpIdentityWithExistingAssetLock(
            walletHandle: Long,
            outpointTxid: ByteArray,
            outpointVout: Int,
            identityId: ByteArray,
            coreSignerHandle: Long,
            consumeInvitationVoucher: Boolean,
        ): Long = CreditsNative.topUpIdentityWithExistingAssetLock(
            walletHandle,
            outpointTxid,
            outpointVout,
            identityId,
            coreSignerHandle,
            consumeInvitationVoucher,
        )

        override fun topUpExternalIdentityFromCore(
            walletHandle: Long,
            identityId: ByteArray,
            amountDuffs: Long,
            accountIndex: Int,
            coreSignerHandle: Long,
        ): Long = CreditsNative.topUpExternalIdentityFromCore(
            walletHandle,
            identityId,
            amountDuffs,
            accountIndex,
            coreSignerHandle,
        )

        override fun resumeTopUpToRecordedRecipient(
            walletHandle: Long,
            outpointTxid: ByteArray,
            outpointVout: Int,
            coreSignerHandle: Long,
        ): IdentityTopUpNativeResult = CreditsNative.resumeTopUpToRecordedRecipient(
            walletHandle,
            outpointTxid,
            outpointVout,
            coreSignerHandle,
        )

        override fun assignTopUpRecipient(
            walletHandle: Long,
            outpointTxid: ByteArray,
            outpointVout: Int,
            identityId: ByteArray,
        ) = CreditsNative.assignTopUpRecipient(walletHandle, outpointTxid, outpointVout, identityId)
    }
}

/**
 * Outcome of [IdentityCredits.resumeTopUpToRecordedRecipient]: the identity the
 * lock paid (32 bytes; in this wallet or not) and its proved new balance.
 */
class IdentityTopUpResult(
    val identityId: ByteArray,
    val newBalance: Long,
) {
    override fun equals(other: Any?): Boolean =
        other is IdentityTopUpResult &&
            identityId.contentEquals(other.identityId) &&
            newBalance == other.newBalance

    override fun hashCode(): Int = identityId.contentHashCode() * 31 + newBalance.hashCode()

    override fun toString(): String = "IdentityTopUpResult(newBalance=$newBalance)"
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
    private val native: TopUpNative = TopUpNative.Jni,
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
            native.topUpIdentityWithExistingAssetLock(
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
     * `ManagedPlatformWallet.resumeTopUpWithAssetLock(consumeInvitationVoucher: true)`
     * (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/ManagedPlatformWallet.swift).
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
            native.topUpIdentityWithExistingAssetLock(
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

    /**
     * Top up someone else's identity — [identityId] (32 bytes), one this
     * wallet does not manage — from a new Core asset lock funded by the
     * wallet's balance. Port of
     * `platform_wallet_top_up_external_identity_with_funding_signer`
     * (packages/rs-platform-wallet-ffi/src/identity_top_up.rs): Rust fetches
     * the identity from Platform, records it on the lock's row before the
     * broadcast and adds nothing to the wallet. Returns the identity's proved
     * post-transition balance.
     *
     * Refused before anything is built: an amount below Platform's top-up
     * minimum (native `ErrorInvalidParameter`), an identity Platform does not
     * know
     * ([DashSdkError.NotFound][org.dashfoundation.dashsdk.errors.DashSdkError.NotFound]),
     * and a store that does not keep identity recipients. Once the lock is
     * built a failure may leave it tracked, recording [identityId]: never
     * retry with a new lock; resume that one with
     * [resumeTopUpToRecordedRecipient], or the identity may be paid twice.
     *
     * The native call cannot observe cancellation, so a top-up can complete
     * after the caller was cancelled; its balance is then still returned
     * ([opReturningCompletedResult][org.dashfoundation.dashsdk.wallet.opReturningCompletedResult]),
     * so a completed payment is never hidden. Handle it before suspending
     * again.
     */
    suspend fun topUpExternalIdentityFromCore(
        walletHandle: Long,
        identityId: ByteArray,
        amountDuffs: Long,
        accountIndex: Int,
        coreSignerHandle: Long,
    ): Long = gate.opReturningCompletedResult {
        require(identityId.size == 32) {
            "identityId must be exactly 32 bytes, got ${identityId.size}"
        }
        mapNativeErrors {
            native.topUpExternalIdentityFromCore(
                walletHandle,
                identityId,
                amountDuffs,
                accountIndex,
                coreSignerHandle,
            )
        }
    }

    /**
     * Resume the interrupted identity top-up [lock] to the identity it pays —
     * the caller names none, so a recovery cannot pay someone else's top-up
     * into one of this wallet's identities. Port of
     * `platform_wallet_resume_identity_top_up_to_recorded_recipient_signer`
     * (packages/rs-platform-wallet-ffi/src/identity_registration_funded_with_signer.rs):
     * a bound top-up lock pays the wallet's identity at its registration
     * index, an unbound one the identity its row recorded
     * ([TrackedAssetLock.identityRecipient]). Returns the identity paid and
     * its proved new balance.
     *
     * A lock recording no identity ([TrackedAssetLock.identityRecipientUnassigned])
     * is refused with
     * [AssetLockRecipientUnknown][org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.AssetLockRecipientUnknown]:
     * assign it one with [assignTopUpRecipient] first. Rust decides which
     * locks it resumes; nothing is filtered here.
     *
     * Like [topUpExternalIdentityFromCore], a resume that completes after the
     * caller was cancelled still returns its result.
     */
    suspend fun resumeTopUpToRecordedRecipient(
        walletHandle: Long,
        lock: TrackedAssetLock,
        coreSignerHandle: Long,
    ): IdentityTopUpResult = gate.opReturningCompletedResult {
        val paid = mapNativeErrors {
            native.resumeTopUpToRecordedRecipient(
                walletHandle,
                lock.outpointTxid,
                lock.outpointVout,
                coreSignerHandle,
            )
        }
        IdentityTopUpResult(paid.identityId, paid.newBalance)
    }

    /**
     * Assign [identityId] (32 bytes) as the identity the unassigned unbound
     * top-up [lock] pays, as the user chose it — then resume it with
     * [resumeTopUpToRecordedRecipient]. Port of
     * `platform_wallet_assign_identity_top_up_recipient`
     * (packages/rs-platform-wallet-ffi/src/identity_registration_funded_with_signer.rs).
     *
     * The assignment is permanent. Rust looks the identity up on Platform
     * first: one Platform does not know is
     * [DashSdkError.NotFound][org.dashfoundation.dashsdk.errors.DashSdkError.NotFound]
     * and nothing is assigned. A lock already recording another identity is
     * [AssetLockRecipientMismatch][org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.AssetLockRecipientMismatch];
     * the same identity again is a no-op, which also retries a failed store.
     */
    suspend fun assignTopUpRecipient(
        walletHandle: Long,
        lock: TrackedAssetLock,
        identityId: ByteArray,
    ): Unit = gate.op {
        require(identityId.size == 32) {
            "identityId must be exactly 32 bytes, got ${identityId.size}"
        }
        mapNativeErrors {
            native.assignTopUpRecipient(
                walletHandle,
                lock.outpointTxid,
                lock.outpointVout,
                identityId,
            )
        }
    }
}
