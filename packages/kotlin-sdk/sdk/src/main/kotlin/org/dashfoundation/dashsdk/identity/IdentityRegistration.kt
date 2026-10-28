package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.wallet.op
import org.dashfoundation.dashsdk.wallet.opWithCleanupOnCancellation

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.credits.FundingInput
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.ContestedDpnsNamesNativeResult
import org.dashfoundation.dashsdk.ffi.IdentityNative
import org.dashfoundation.dashsdk.ffi.IdentityRegistrationNativeResult
import org.dashfoundation.dashsdk.ffi.TokensNative
import org.dashfoundation.dashsdk.tokens.translateManagedIdentityNotFoundToZero
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock

internal fun interface ResumeIdentityNativeCall {
    fun call(
        walletHandle: Long,
        outpointTxid: ByteArray,
        outpointVout: Int,
        identityIndex: Int,
        pubkeysBlob: ByteArray,
        signerHandle: Long,
        coreSignerHandle: Long,
        consumeInvitationVoucher: Boolean,
    ): IdentityRegistrationNativeResult
}

internal fun interface ClaimInvitationNativeCall {
    fun call(
        walletHandle: Long,
        uri: String,
        identityIndex: Int,
        pubkeysBlob: ByteArray,
        signerHandle: Long,
        nowUnix: Long,
    ): IdentityRegistrationNativeResult
}

internal fun interface SyncContestedDpnsNativeCall {
    fun call(walletHandle: Long, identityId: ByteArray): Int
}

internal fun interface ManagedIdentityLookupNativeCall {
    fun call(walletHandle: Long, identityId: ByteArray): Long
}

internal fun interface CachedContestedDpnsNativeCall {
    fun call(identityHandle: Long): ContestedDpnsNamesNativeResult
}

/**
 * Thin wrapper over the identity JNI surface (`IdentityNative`) — port of
 * the identity-registration slice of `ManagedPlatformWallet.swift`.
 *
 * Every method is a single FFI call marshalled onto [Dispatchers.IO] and
 * routed through [mapNativeErrors]; no orchestration lives here (see
 * `packages/kotlin-sdk/CLAUDE.md`). The handles are supplied by the caller
 * — `PlatformWalletManager` owns the `signerHandle` /
 * `mnemonicResolverHandle`, and `ManagedPlatformWallet` owns the wallet
 * handle.
 */
class IdentityRegistration internal constructor(
    private val gate: org.dashfoundation.dashsdk.wallet.TeardownGate? = null,
    private val resumeNative: ResumeIdentityNativeCall =
        ResumeIdentityNativeCall(IdentityNative::resumeIdentityWithExistingAssetLock),
    private val claimInvitationNative: ClaimInvitationNativeCall =
        ClaimInvitationNativeCall(
            org.dashfoundation.dashsdk.ffi.DashpayNative::claimInvitation,
        ),
    private val destroyManagedIdentity: (Long) -> Unit = TokensNative::managedIdentityDestroy,
    private val syncContestedDpnsNative: SyncContestedDpnsNativeCall =
        SyncContestedDpnsNativeCall(IdentityNative::syncContestedDpnsNames),
    private val managedIdentityLookupNative: ManagedIdentityLookupNativeCall =
        ManagedIdentityLookupNativeCall(TokensNative::getManagedIdentity),
    private val cachedContestedDpnsNative: CachedContestedDpnsNativeCall =
        CachedContestedDpnsNativeCall(TokensNative::managedIdentityContestedDpnsNames),
) {

    /** One canonical network sync followed by one fresh cached snapshot. */
    data class ContestedDpnsSnapshot(
        val labels: List<String>,
        val refreshedCount: Int,
    )

    /**
     * Fetch every unresolved contested DPNS label for [identityId] in one
     * shared Rust operation, then read the full-replacement cache through a
     * newly-owned managed-identity handle. Resolved contests disappear; no
     * local-label probing or host-side limit is applied.
     *
     * Android persistence does not yet store contested labels: this snapshot
     * is process-local and callers must sync again after restart. The existing
     * persistence schema version is reserved for invitation work, so this API
     * deliberately does not overload it with a conflicting field.
     */
    suspend fun contestedDpnsNames(
        walletHandle: Long,
        identityId: ByteArray,
    ): ContestedDpnsSnapshot = gate.op {
        require(identityId.size == 32) {
            "identityId must be exactly 32 bytes, got ${identityId.size}"
        }
        val refreshedCount = mapNativeErrors {
            syncContestedDpnsNative.call(walletHandle, identityId)
        }
        // The native side reports an unmanaged identity as a platform-wallet
        // NotFound error, not a zero handle; translate the raw code back to
        // the zero-handle signal so the intended, caller-facing message below
        // is what actually surfaces (dashpay/platform#4060).
        val identityHandle = mapNativeErrors {
            translateManagedIdentityNotFoundToZero {
                managedIdentityLookupNative.call(walletHandle, identityId)
            }
        }
        if (identityHandle == 0L) {
            throw DashSdkError.NotFound("identity is not managed by this wallet")
        }
        try {
            val cached = mapNativeErrors {
                cachedContestedDpnsNative.call(identityHandle)
            }
            ContestedDpnsSnapshot(
                labels = cached.labels.toList(),
                refreshedCount = refreshedCount,
            )
        } finally {
            destroyManagedIdentity(identityHandle)
        }
    }

    /**
     * Resume identity registration from [lock]'s exact persisted outpoint.
     * Rust owns rebroadcast/proof/status decisions; Kotlin never builds a
     * replacement funding transaction. Generic recovery cannot consume an
     * invitation voucher.
     *
     * [keys] carries the rich registration rows (built via
     * [RegistrationKeys.buildRegistrationRows]) and the HD slot that derived
     * them. Resume carries the SAME key
     * set the interrupted registration originally committed to on-chain — the
     * base four auth/transfer keys, **without** the DashPay pair: an
     * already-spent asset lock funds a fixed key count, so retroactively
     * growing the transition it funds risks a resume that fails after the
     * user's DASH is already irreversibly locked (matching iOS, which excludes
     * DashPay provisioning from the resume path). A user who resumes and wants
     * DashPay capability adds those keys afterward via the Add Identity Key
     * flow.
     */
    /**
     * The creation key layout Rust decides for resuming [lock]: the four
     * pre-DIP-11 keys when its amount cannot fund the DIP-11 key set (a lock
     * funded before the MEDIUM key existed), else the new-identity layout.
     * Derive exactly this layout for [resumeWithExistingAssetLock].
     */
    suspend fun resumeKeyLayout(
        walletHandle: Long,
        lock: TrackedAssetLock,
    ): IdentityCreationKeyLayout = gate.op {
        val code = mapNativeErrors {
            IdentityNative.resumeIdentityKeyLayout(walletHandle, lock.outpointTxid, lock.outpointVout)
        }
        IdentityCreationKeyLayout.fromNativeCode(code)
    }

    suspend fun resumeWithExistingAssetLock(
        walletHandle: Long,
        lock: TrackedAssetLock,
        identityIndex: Int,
        keys: RegistrationKeySet,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ByteArray = gate.op {
        require(lock.fundingType == TrackedAssetLock.FundingType.IDENTITY_REGISTRATION) {
            "registration recovery requires funding type 0, got ${lock.fundingType.raw}"
        }
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(identityIndex == lock.registrationIndex) {
            "identityIndex $identityIndex does not match tracked lock registrationIndex ${lock.registrationIndex}"
        }
        require(keys.identityIndex == lock.registrationIndex) {
            "registration keys use identityIndex ${keys.identityIndex}, expected tracked lock " +
                "registrationIndex ${lock.registrationIndex}"
        }
        val native = mapNativeErrors {
            resumeNative.call(
                walletHandle = walletHandle,
                outpointTxid = lock.outpointTxid,
                outpointVout = lock.outpointVout,
                identityIndex = identityIndex,
                pubkeysBlob = IdentityPubkeyCodec.encode(keys.rows),
                signerHandle = signerHandle,
                coreSignerHandle = coreSignerHandle,
                consumeInvitationVoucher = false,
            )
        }
        // Adopt immediately. The standalone handle is only an FFI result;
        // the identity itself is already folded into Rust's manager and Room.
        val managed = ManagedIdentityResultHandle(
            native.managedIdentityHandle,
            destroyManagedIdentity,
        )
        managed.use {
            check(native.managedIdentityHandle != 0L) {
                "native registration returned a null managed-identity handle"
            }
            check(native.identityId.size == 32) {
                "native registration returned ${native.identityId.size}-byte identity id"
            }
            native.identityId.copyOf()
        }
    }

    /**
     * Reclaim an unclaimed DIP-13 invitation voucher by registering a NEW
     * identity funded by it — the "register" reclaim target (the sibling
     * of `IdentityCredits.reclaimInvitationAsTopUp`). The voucher's DASH
     * was OP_RETURN-burned at create; the value returns as the new
     * identity's credits. ← Swift
     * `ManagedPlatformWallet.resumeIdentityWithAssetLock(consumeInvitationVoucher: true)`
     * (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/ManagedPlatformWallet.swift).
     *
     * Takes the raw outpoint (from the invitation row's `rawOutPoint`),
     * not a [TrackedAssetLock] — invitation locks are deliberately
     * excluded from the generic tracked-lock recovery model, and no
     * funding-type or registration-index gate applies: the identity slot
     * is freely chosen (next unused). [keys] carries the
     * [IdentityCreationKeyLayout.NEW_IDENTITY] set (no DashPay keys: a
     * reclaim sends no contact request). A reclaim creates a new identity,
     * so Rust applies the DIP-11 check to it.
     * `consumeInvitationVoucher = true` is the explicit authorization
     * Rust core requires; this is one of the two call sites that pass it.
     *
     * @return the 32-byte identity id of the newly-registered identity.
     */
    suspend fun reclaimInvitationAsNewIdentity(
        walletHandle: Long,
        outPointTxid: ByteArray,
        outPointVout: Int,
        identityIndex: Int,
        keys: RegistrationKeySet,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ByteArray = gate.op {
        require(outPointTxid.size == 32) {
            "outPointTxid must be exactly 32 bytes, got ${outPointTxid.size}"
        }
        require(outPointVout >= 0) { "outPointVout must be non-negative, got $outPointVout" }
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(keys.identityIndex == identityIndex) {
            "reclaim keys use identityIndex ${keys.identityIndex}, expected $identityIndex"
        }
        val native = mapNativeErrors {
            resumeNative.call(
                walletHandle = walletHandle,
                outpointTxid = outPointTxid,
                outpointVout = outPointVout,
                identityIndex = identityIndex,
                pubkeysBlob = IdentityPubkeyCodec.encode(keys.rows),
                signerHandle = signerHandle,
                coreSignerHandle = coreSignerHandle,
                consumeInvitationVoucher = true,
            )
        }
        val managed = ManagedIdentityResultHandle(
            native.managedIdentityHandle,
            destroyManagedIdentity,
        )
        managed.use {
            check(native.managedIdentityHandle != 0L) {
                "native reclaim returned a null managed-identity handle"
            }
            check(native.identityId.size == 32) {
                "native reclaim returned ${native.identityId.size}-byte identity id"
            }
            native.identityId.copyOf()
        }
    }

    /**
     * Claim a `dashpay://invite` link: register a NEW identity for the
     * invitee funded by the imported voucher (DIP-13). Blocking (the SDK
     * refetches the funding tx by txid, then waits for the Platform
     * response). Works with no pre-existing identity — the fresh-invitee
     * onboarding path. ← Swift `ManagedPlatformWallet.claimInvitation`
     * (packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/ManagedPlatformWallet.swift).
     *
     * [keys] carries the full fresh-registration set, every key of
     * [IdentityCreationKeyLayout.NEW_IDENTITY_WITH_DASHPAY_KEYS] including
     * the DashPay ENCRYPTION/DECRYPTION pair
     * — unlike asset-lock resume, a claim funds a brand-new transition, so
     * the invitee gets DashPay capability from the start. Each row's
     * private key must already be persisted (see
     * [previewRegistrationKeySet]). No core signer: the asset-lock's outer
     * signature uses the link's raw voucher key.
     *
     * The [uri] is a bearer credential (it embeds the one-time voucher
     * key) — this wrapper never logs it and callers must not either.
     *
     * @return the 32-byte identity id of the newly-registered invitee
     *   identity (already folded into Rust's manager and Room).
     */
    suspend fun claimInvitation(
        walletHandle: Long,
        uri: String,
        identityIndex: Int,
        keys: RegistrationKeySet,
        signerHandle: Long,
        // Long for the same 2038 reason as Dashpay.createInvitation.
        nowUnix: Long = System.currentTimeMillis() / 1000L,
    ): ByteArray = gate.op {
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(keys.identityIndex == identityIndex) {
            "claim keys use identityIndex ${keys.identityIndex}, expected $identityIndex"
        }
        val native = mapNativeErrors {
            claimInvitationNative.call(
                walletHandle = walletHandle,
                uri = uri,
                identityIndex = identityIndex,
                pubkeysBlob = IdentityPubkeyCodec.encode(keys.rows),
                signerHandle = signerHandle,
                nowUnix = nowUnix,
            )
        }
        // Adopt immediately. The standalone handle is only an FFI result;
        // the identity itself is already folded into Rust's manager and Room.
        val managed = ManagedIdentityResultHandle(
            native.managedIdentityHandle,
            destroyManagedIdentity,
        )
        managed.use {
            check(native.managedIdentityHandle != 0L) {
                "native claim returned a null managed-identity handle"
            }
            check(native.identityId.size == 32) {
                "native claim returned ${native.identityId.size}-byte identity id"
            }
            native.identityId.copyOf()
        }
    }

    /**
     * Derive the identity-registration keys for a wallet slot (path +
     * public key + private scalar). Pure compute — no Platform RPCs. The
     * caller persists each row's private key to the Keystore before
     * registering; see `IdentityKeyPreview`.
     *
     * @param count number of consecutive slots to derive; < 0 uses the
     *   Rust gap-limit default.
     */
    suspend fun previewRegistrationKeys(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        startIndex: Int,
        count: Int = -1,
    ): List<IdentityKeyPreview> = gate.opWithCleanupOnCancellation(
        cleanup = { previews -> previews.forEach { it.privateKey.fill(0) } },
    ) {
        val blob = mapNativeErrors {
            IdentityNative.previewRegistrationKeys(
                walletHandle,
                mnemonicResolverHandle,
                startIndex,
                count,
            )
        }
        IdentityKeyPreview.decodeAll(blob)
    }

    /**
     * Derive the full identity-registration key SET of [layout] for a single
     * identity at the fixed [identityIndex]: one keypair per slot of the Rust
     * layout, in key-id order. This is the call the create-identity flow uses:
     * it fixes the identity index and walks the key index, so a new identity
     * is provisioned with every key of its layout rather than just the MASTER
     * key. ([previewRegistrationKeys], by contrast, walks the *identity* index
     * at the MASTER slot for the discovery preview.)
     *
     * Each row here carries only the derived keypair; the per-key DPP role
     * comes from [RegistrationKeys.layout] for the same [layout]. The caller
     * persists each row's private key to the Keystore before registering.
     */
    suspend fun previewRegistrationKeySet(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        identityIndex: Int,
        layout: IdentityCreationKeyLayout,
    ): List<IdentityKeyPreview> = gate.opWithCleanupOnCancellation(
        cleanup = { previews -> previews.forEach { it.privateKey.fill(0) } },
    ) {
        val blob = mapNativeErrors {
            IdentityNative.previewRegistrationKeySet(
                walletHandle,
                mnemonicResolverHandle,
                identityIndex,
                layout.nativeCode,
            )
        }
        IdentityKeyPreview.decodeAll(blob)
    }

    /**
     * Register a new identity funded from the wallet's Core balance. The
     * single entry point the registration coordinator's body calls. [keys] are
     * the rich registration rows (built via
     * [RegistrationKeys.buildRegistrationRows]); each row's private half must
     * already be derived + persisted (see [previewRegistrationKeySet]).
     *
     * @return the 32-byte identity id.
     */
    suspend fun registerWithWalletFunding(
        walletHandle: Long,
        amountDuffs: Long,
        accountIndex: Int,
        identityIndex: Int,
        keys: List<IdentityPubkey>,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ByteArray = gate.op {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        require(accountIndex >= 0) { "accountIndex must be non-negative, got $accountIndex" }
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        mapNativeErrors {
            IdentityNative.registerIdentityWithFunding(
                walletHandle,
                amountDuffs,
                accountIndex,
                identityIndex,
                IdentityPubkeyCodec.encode(keys),
                signerHandle,
                coreSignerHandle,
            )
        }
    }

    /**
     * Register a new identity funded by the wallet's already-committed
     * Platform-payment (DIP-17) address balances — the ID-08 path, distinct
     * from [registerWithWalletFunding] (ID-01) which builds a new Core asset
     * lock. Keys must already be derived + persisted (see
     * [previewRegistrationKeySet]); [inputs] are the funding addresses (the
     * greedily-packed balance-carrying Platform-payment addresses). [keys] are
     * the rich registration rows (built via
     * [RegistrationKeys.buildRegistrationRows]). The same [signerHandle] drives
     * both the identity-key and platform-address signing roles. Nonces are
     * auto-fetched Rust-side.
     *
     * @return the 32-byte identity id.
     */
    suspend fun registerFromAddresses(
        walletHandle: Long,
        identityIndex: Int,
        keys: List<IdentityPubkey>,
        signerHandle: Long,
        inputs: List<FundingInput>,
    ): ByteArray = gate.op {
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(inputs.isNotEmpty()) { "inputs must not be empty" }
        mapNativeErrors {
            IdentityNative.registerIdentityFromAddresses(
                walletHandle,
                identityIndex,
                IdentityPubkeyCodec.encode(keys),
                signerHandle,
                FundingInput.encode(inputs),
            )
        }
    }

    /**
     * Scan the wallet for registered identities (gap-limit walk). Returns
     * the discovered 32-byte identity ids.
     *
     * @param startIndex first slot to probe; < 0 uses the Rust default.
     */
    suspend fun discoverIdentities(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        startIndex: Int = -1,
        gapLimit: Int = 5,
    ): List<ByteArray> = gate.op {
        val flat = mapNativeErrors {
            IdentityNative.discoverIdentities(
                walletHandle,
                mnemonicResolverHandle,
                startIndex,
                gapLimit,
            )
        }
        flat.asList().chunked(32) { it.toByteArray() }.filter { it.size == 32 }
    }

    /**
     * Register a DPNS name for [identityId] (32 bytes), signed via
     * [signerHandle]. Returns the full domain name (e.g. `"alice.dash"`).
     *
     * @param maxContestFund the most, in credits, the identity pays into
     *   the contest a contested name joins, and the identity must hold it;
     *   `null` states the current fund to join, read just before signing. A name that joins no contest
     *   ignores it. Mirrors Swift `ManagedPlatformWallet.registerDpnsName`.
     */
    suspend fun registerDpnsName(
        walletHandle: Long,
        identityId: ByteArray,
        label: String,
        signerHandle: Long,
        maxContestFund: Long? = null,
    ): String = gate.op {
        require(maxContestFund == null || maxContestFund >= 0) {
            "maxContestFund must be non-negative, got $maxContestFund"
        }
        mapNativeErrors {
            IdentityNative.registerDpnsName(
                walletHandle,
                identityId,
                label,
                maxContestFund ?: 0L,
                signerHandle,
            )
        }
    }
}

private class ManagedIdentityResultHandle(
    handle: Long,
    private val destroy: (Long) -> Unit,
) : AutoCloseable {
    private val handleRef = java.util.concurrent.atomic.AtomicLong(handle)

    override fun close() {
        val handle = handleRef.getAndSet(0L)
        if (handle != 0L) destroy(handle)
    }
}
