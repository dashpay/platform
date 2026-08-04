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
     * Derive the full identity-registration key SET for a single identity
     * — keyId 0..[count] at the fixed [identityIndex]. This is the call
     * the create-identity flow uses: it fixes the identity index and walks
     * the key index, so a freshly created identity is provisioned with the
     * whole canonical key set (keyId 0 MASTER/AUTH, 1 CRITICAL/AUTH, 2
     * HIGH/AUTH, 3 TRANSFER/CRITICAL) rather than just the MASTER key.
     *
     * Without the full set, all Platform writes fail validation right
     * after creation: document / DPNS / token / contract transitions need
     * a HIGH-or-CRITICAL AUTHENTICATION key, and credit transfers /
     * withdrawals need the TRANSFER key ("no transfer public key").
     *
     * Each row here carries only the derived keypair; the per-key DPP role is
     * stamped Kotlin-side by [RegistrationKeys] when the rich registration rows
     * are built. The caller persists each row's private key to the Keystore
     * before registering.
     *
     * @param count number of keys to derive; < 0 uses the canonical
     *   default set (4 keys). ([previewRegistrationKeys], by contrast,
     *   walks the *identity* index at the MASTER slot for the discovery
     *   preview.)
     */
    suspend fun previewRegistrationKeySet(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        identityIndex: Int,
        count: Int = -1,
    ): List<IdentityKeyPreview> = gate.opWithCleanupOnCancellation(
        cleanup = { previews -> previews.forEach { it.privateKey.fill(0) } },
    ) {
        val blob = mapNativeErrors {
            IdentityNative.previewRegistrationKeySet(
                walletHandle,
                mnemonicResolverHandle,
                identityIndex,
                count,
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
     */
    suspend fun registerDpnsName(
        walletHandle: Long,
        identityId: ByteArray,
        label: String,
        signerHandle: Long,
    ): String = gate.op {
        mapNativeErrors {
            IdentityNative.registerDpnsName(walletHandle, identityId, label, signerHandle)
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
