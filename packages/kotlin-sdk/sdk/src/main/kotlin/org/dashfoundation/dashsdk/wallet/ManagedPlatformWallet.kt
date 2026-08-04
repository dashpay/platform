package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.NativeCleaner
import org.dashfoundation.dashsdk.ffi.TokensNative
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.dashfoundation.dashsdk.tokens.translateManagedIdentityNotFoundToZero
import java.util.concurrent.atomic.AtomicLong

/**
 * Thin per-wallet wrapper over a native `PlatformWallet` handle — port of
 * `ManagedPlatformWallet.swift`.
 *
 * Like the Swift type, this **owns the wallet handle** and destroys it
 * (`platform_wallet_destroy`, which drops the manager's `Arc` clone) via
 * [close] or the [NativeCleaner] backstop. The Rust `PlatformWalletManager`
 * keeps its own registration; destroying this wrapper's handle only drops
 * one `Arc` clone, so it is safe even while the manager still holds the
 * wallet.
 *
 * All native reads are cheap in-memory lookups but are still confined to
 * [Dispatchers.IO] at the suspend boundary and go through `mapNativeErrors`
 * so callers only ever see the public error hierarchy.
 *
 * @property walletId the 32-byte wallet id (hex-keyed in the manager map).
 */
class ManagedPlatformWallet internal constructor(
    handle: Long,
    val walletId: ByteArray,
    // Owning manager's teardown fence: ops that borrow the manager's raw
    // signer/resolver handles run on the CALLER's scope, so manager
    // teardown must await them through this gate before freeing the boxes.
    // Null only for bare test construction (no fence, old behavior).
    private val gate: TeardownGate? = null,
) : AutoCloseable {

    private val handleRef = AtomicLong(handle)
    private val cleanable = NativeCleaner.register(this, HandleCleanup(handleRef))

    /** Raw native `PlatformWallet` handle; throws if the wrapper was closed. */
    val handle: Long
        get() = handleRef.get().also {
            check(it != 0L) { "ManagedPlatformWallet has been closed" }
        }

    val isClosed: Boolean get() = handleRef.get() == 0L

    /** Hex form of [walletId] — the manager-map key and nav-arg format. */
    val walletIdHex: String = walletId.joinToString("") { "%02x".format(it) }

    /**
     * Token state-transition surface bound to this wallet's handle — the
     * Kotlin analog of the Swift `ManagedPlatformWallet` token-action
     * extension (`TokenActions.swift`). Stateless: each call reads
     * [handle] afresh, so a closed wrapper fails fast.
     */
    val tokens: org.dashfoundation.dashsdk.tokens.Tokens
        get() = org.dashfoundation.dashsdk.tokens.Tokens(handle, gate)

    /**
     * Group-action discovery bound to this wallet's handle — the Kotlin
     * analog of the Swift `TokenGroupActionQueries.swift` extension.
     */
    val groups: org.dashfoundation.dashsdk.tokens.Groups
        // Ungated on purpose: Groups' queries are Arc-based wallet-handle
        // reads (no signer/resolver borrow), safe concurrent with teardown.
        get() = org.dashfoundation.dashsdk.tokens.Groups(handle)

    /**
     * DashPay contact/payment surface bound to this wallet's handle —
     * the Kotlin analog of the Swift `ManagedPlatformWallet` DashPay
     * extension.
     */
    val dashpay: org.dashfoundation.dashsdk.tokens.Dashpay
        get() = org.dashfoundation.dashsdk.tokens.Dashpay(handle, gate)

    /**
     * Data-contract create surface bound to this wallet's handle — the
     * Kotlin analog of Swift's `ManagedPlatformWallet.createDataContract`.
     */
    val dataContracts: org.dashfoundation.dashsdk.identity.DataContracts
        get() = org.dashfoundation.dashsdk.identity.DataContracts(handle, gate)

    /** Lock-free balance snapshot from Rust's in-memory state. */
    data class Balance(
        val confirmed: Long,
        val unconfirmed: Long,
        val immature: Long,
        val locked: Long,
    )

    /**
     * Read the wallet's lock-free balance — port of Swift's `balance()`.
     * Calls `platform_wallet_get_balance` (atomic reads, no disk I/O).
     */
    suspend fun balance(): Balance = withContext(Dispatchers.IO) {
        val b = mapNativeErrors { WalletManagerNative.walletGetBalance(handle) }
        Balance(
            confirmed = b.getOrElse(0) { 0L },
            unconfirmed = b.getOrElse(1) { 0L },
            immature = b.getOrElse(2) { 0L },
            locked = b.getOrElse(3) { 0L },
        )
    }

    /** Standard account derivation shape for [sendToAddresses]. */
    enum class AccountType(val ffiValue: Int) {
        BIP44(0),
        BIP32(1),
    }

    /**
     * The transient core-wallet handle for UTXO management, addresses, and
     * transaction broadcasting — port of Swift's
     * `ManagedPlatformWallet.coreWallet()`. The returned [ManagedCoreWallet]
     * owns the handle and destroys it on `close()` / GC.
     */
    fun coreWallet(): ManagedCoreWallet =
        ManagedCoreWallet(mapNativeErrors { WalletManagerNative.platformWalletGetCore(handle) })

    /**
     * Build, sign, and broadcast a Core payment to [recipients], returning
     * the broadcast txid as a lowercase hex string.
     *
     * Mirrors the `.coreToCore` flow in Swift's `SendViewModel.executeSend`
     * (SendViewModel.swift:515-533): drive a [CoreTransactionBuilder] step by
     * step (`new → addOutput* → finalizeAtomic`), then consume the finalized
     * handle through broadcast. Rust atomically selects and reserves inputs
     * before signing; Kotlin only marshals outputs and owns the handle lifetime.
     *
     * @param network the wallet network — output/change addresses are
     *   validated against it Rust-side (Swift's `SendViewModel` likewise hands
     *   the app network to the builder). The atomic finalizer re-checks it
     *   against the wallet's own network.
     * @param coreSignerHandle the manager's `MnemonicResolverHandle`
     *   (`PlatformWalletManager.mnemonicResolverHandle`) — used for the
     *   Core ECDSA signatures. No private key crosses the boundary.
     */
    suspend fun sendToAddresses(
        recipients: List<Pair<String, Long>>,
        network: org.dashfoundation.dashsdk.Network,
        coreSignerHandle: Long,
        accountType: AccountType = AccountType.BIP44,
        accountIndex: Int = 0,
    ): String = gate.op {
        require(accountIndex >= 0) { "accountIndex must be non-negative, got $accountIndex" }
        require(recipients.isNotEmpty()) { "recipients must not be empty" }
        require(recipients.all { it.second > 0 }) {
            "every recipient amount must be positive"
        }
        val builderAccountType = when (accountType) {
            AccountType.BIP44 -> CoreTransactionBuilder.AccountType.BIP44
            AccountType.BIP32 -> CoreTransactionBuilder.AccountType.BIP32
        }
        mapNativeErrors {
            val builder = CoreTransactionBuilder(network)
            // `buildSigned` consumes the builder; `use` still safely destroys
            // it on the pre-build failure paths (addOutput / setFunding throw).
            val signedTx = builder.use {
                for ((address, amount) in recipients) {
                    it.addOutput(address, amount)
                }
                it.finalizeAtomic(
                    this@ManagedPlatformWallet,
                    builderAccountType,
                    accountIndex,
                    coreSignerHandle,
                )
            }
            signedTx.use { tx -> coreWallet().use { core -> core.broadcastTransaction(tx) } }
        }
    }

    /**
     * The wallet's Platform-payment addresses that currently hold credits,
     * each as a [FundingInput] whose `credits` is the full cached balance —
     * the funding-input candidates for an identity top-up. Port of the
     * `account.platformAddresses.filter { balance > 0 }` enumeration Swift's
     * `TopUpIdentityView` performs. Addresses with a zero balance are
     * filtered out (they can't fund anything). One composite Rust call
     * (get-platform → enumerate → free → destroy-handle).
     */
    suspend fun addressesWithBalances(): List<org.dashfoundation.dashsdk.credits.FundingInput> =
        withContext(Dispatchers.IO) {
            val blob = mapNativeErrors {
                WalletManagerNative.walletAddressesWithBalances(handle)
            }
            decodeAddressBalances(blob)
        }

    /**
     * One recipient of an asset-lock funding — port of Swift's
     * `FundFromAssetLockRecipient`. Exactly one recipient in a request must
     * have `credits = null` (the fee-absorbing remainder recipient).
     *
     * @property addressType 0 = P2PKH, 1 = P2SH.
     * @property hash the 20-byte recipient address hash.
     * @property credits credits to route to this address, or null for the
     *   single remainder recipient.
     */
    data class FundRecipient(
        val addressType: Int,
        val hash: ByteArray,
        val credits: Long?,
    ) {
        init {
            require(hash.size == 20) { "FundRecipient.hash must be 20 bytes, got ${hash.size}" }
            require(credits == null || credits > 0) {
                "FundRecipient.credits must be positive (or null for the remainder recipient), got $credits"
            }
        }

        override fun equals(other: Any?): Boolean =
            other is FundRecipient &&
                addressType == other.addressType &&
                hash.contentEquals(other.hash) &&
                credits == other.credits

        override fun hashCode(): Int =
            (31 * addressType + hash.contentHashCode()) * 31 + (credits?.hashCode() ?: 0)
    }

    /** One updated address balance from a funding changeset. */
    data class UpdatedBalance(val addressType: Int, val hash: ByteArray, val balance: Long) {
        override fun equals(other: Any?): Boolean =
            other is UpdatedBalance &&
                addressType == other.addressType &&
                hash.contentEquals(other.hash) &&
                balance == other.balance

        override fun hashCode(): Int =
            (31 * addressType + hash.contentHashCode()) * 31 + balance.hashCode()
    }

    /**
     * Fund Platform addresses from a Core L1 asset lock built from this
     * wallet's balance — port of `ManagedPlatformAddressWallet.fundFromAssetLock`.
     * Returns the resulting per-address updated balances (the funding
     * changeset). Exactly one [recipients] entry must have `credits = null`.
     *
     * @param signerHandle the platform-address per-input `SignerHandle`
     *   (`PlatformWalletManager.signerHandle`).
     * @param coreSignerHandle the manager's `MnemonicResolverHandle`
     *   (`PlatformWalletManager.mnemonicResolverHandle`) for the asset-lock's
     *   outer state-transition signature.
     */
    suspend fun fundFromAssetLock(
        amountDuffs: Long,
        fundingAccountIndex: Int,
        platformAccountIndex: Int,
        recipients: List<FundRecipient>,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): List<UpdatedBalance> = gate.op {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        require(fundingAccountIndex >= 0) {
            "fundingAccountIndex must be non-negative, got $fundingAccountIndex"
        }
        require(platformAccountIndex >= 0) {
            "platformAccountIndex must be non-negative, got $platformAccountIndex"
        }
        val blob = mapNativeErrors {
            WalletManagerNative.walletFundFromAssetLock(
                walletHandle = handle,
                amountDuffs = amountDuffs,
                accountIndex = fundingAccountIndex,
                platformAccountIndex = platformAccountIndex,
                recipientsBlob = encodeRecipients(recipients),
                signerHandle = signerHandle,
                coreSignerHandle = coreSignerHandle,
            )
        }
        decodeChangeset(blob)
    }

    /**
     * Resume a stuck Platform-address asset-lock funding from an already-
     * tracked lock — port of `ManagedPlatformAddressWallet.resumeFundFromAssetLock`.
     *
     * @param outPointTxid 32-byte raw txid (little-endian wire order).
     */
    suspend fun resumeFundFromAssetLock(
        outPointTxid: ByteArray,
        outPointVout: Int,
        platformAccountIndex: Int,
        recipients: List<FundRecipient>,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): List<UpdatedBalance> = gate.op {
        require(outPointTxid.size == 32) {
            "outPointTxid must be exactly 32 bytes, got ${outPointTxid.size}"
        }
        require(outPointVout >= 0) { "outPointVout must be non-negative, got $outPointVout" }
        require(platformAccountIndex >= 0) {
            "platformAccountIndex must be non-negative, got $platformAccountIndex"
        }
        val blob = mapNativeErrors {
            WalletManagerNative.walletResumeFundFromAssetLock(
                walletHandle = handle,
                outPointTxid = outPointTxid,
                outPointVout = outPointVout,
                platformAccountIndex = platformAccountIndex,
                recipientsBlob = encodeRecipients(recipients),
                signerHandle = signerHandle,
                coreSignerHandle = coreSignerHandle,
            )
        }
        decodeChangeset(blob)
    }

    // ── Wallet-signed Platform-address credit movement (ADDR-02/04) ───

    /** One recipient of a wallet-signed platform-address transfer. */
    data class CreditOutput(val addressType: Int, val hash: ByteArray, val credits: Long) {
        init {
            require(hash.size == 20) { "CreditOutput.hash must be 20 bytes, got ${hash.size}" }
            require(credits > 0) { "CreditOutput.credits must be positive" }
        }

        override fun equals(other: Any?): Boolean =
            other is CreditOutput &&
                addressType == other.addressType &&
                hash.contentEquals(other.hash) &&
                credits == other.credits

        override fun hashCode(): Int =
            (31 * addressType + hash.contentHashCode()) * 31 + credits.hashCode()
    }

    /** Version-locked minimum transfer/withdraw amounts (credits). */
    data class MinAmounts(val minInput: Long, val minOutput: Long)

    /** Result of a withdrawal preflight (mirror of `WithdrawalPreflightFFI`). */
    data class WithdrawalPreflight(
        val canWithdraw: Boolean,
        val netWithdrawable: Long,
        val estimatedFee: Long,
    )

    /**
     * Transfer platform-address credits to [outputs], signed by the wallet's
     * platform-address signer — port of Swift's rewritten
     * `ManagedPlatformAddressWallet.transfer`. AUTO input selection: Rust
     * owns selection / balancing / nonces / signing; Kotlin only marshals the
     * recipient list. Returns the resulting per-address updated balances.
     *
     * @param signerHandle the platform-address per-input `SignerHandle`
     *   (`PlatformWalletManager.signerHandle`).
     */
    suspend fun transferCredits(
        outputs: List<CreditOutput>,
        signerHandle: Long,
        accountIndex: Int = 0,
    ): List<UpdatedBalance> = gate.op {
        require(accountIndex >= 0) { "accountIndex must be non-negative, got $accountIndex" }
        val blob = mapNativeErrors {
            WalletManagerNative.walletPlatformAddressTransfer(
                walletHandle = handle,
                accountIndex = accountIndex,
                outputsBlob = encodeCreditOutputs(outputs),
                signerHandle = signerHandle,
            )
        }
        decodeChangeset(blob)
    }

    /**
     * Withdraw the account's full platform-address credit balance to a Core
     * L1 address, signed by the wallet's platform-address signer — port of
     * Swift's `ManagedPlatformAddressWallet.withdraw`. The address is
     * network-checked Rust-side. Returns the resulting per-address updated
     * balances.
     *
     * @param coreFeePerByte a Fibonacci-sequence core fee rate (DPP rejects
     *   non-Fibonacci values).
     * @param signerHandle the platform-address per-input `SignerHandle`.
     */
    suspend fun withdrawCredits(
        coreAddress: String,
        coreFeePerByte: Int,
        signerHandle: Long,
        accountIndex: Int = 0,
    ): List<UpdatedBalance> = gate.op {
        require(accountIndex >= 0) { "accountIndex must be non-negative, got $accountIndex" }
        val blob = mapNativeErrors {
            WalletManagerNative.walletPlatformAddressWithdraw(
                walletHandle = handle,
                accountIndex = accountIndex,
                coreAddress = coreAddress,
                coreFeePerByte = coreFeePerByte,
                signerHandle = signerHandle,
            )
        }
        decodeChangeset(blob)
    }

    /**
     * Preflight an AUTO withdrawal WITHOUT signing / broadcasting / consuming
     * a Core address — port of Swift's `preflightWithdrawal`. Reads the
     * account's on-chain balances and sizes the plan the spend would use, so
     * gating a submit button on [WithdrawalPreflight.canWithdraw] keeps it in
     * lockstep with what the spend accepts. Runs on [Dispatchers.IO] (the
     * Rust side polls a proof query on an 8 MB-stack worker).
     */
    suspend fun preflightWithdrawal(
        accountIndex: Int = 0,
        coreFeePerByte: Int = 0,
    ): WithdrawalPreflight = withContext(Dispatchers.IO) {
        val triple = mapNativeErrors {
            WalletManagerNative.walletPlatformAddressPreflightWithdrawal(
                walletHandle = handle,
                accountIndex = accountIndex,
                coreFeePerByte = coreFeePerByte,
            )
        }
        WithdrawalPreflight(
            canWithdraw = triple.getOrElse(0) { 0L } != 0L,
            netWithdrawable = triple.getOrElse(1) { 0L },
            estimatedFee = triple.getOrElse(2) { 0L },
        )
    }

    /**
     * The version-locked minimum input / output amounts (credits) that gate
     * the transfer/withdraw UI — port of Swift's `minInputAmount()` /
     * `minOutputAmount()`, folded into one composite call.
     */
    suspend fun minAmounts(): MinAmounts = withContext(Dispatchers.IO) {
        val pair = mapNativeErrors { WalletManagerNative.walletPlatformAddressMinAmounts(handle) }
        MinAmounts(
            minInput = pair.getOrElse(0) { 0L },
            minOutput = pair.getOrElse(1) { 0L },
        )
    }

    // ── Wallet-memory snapshots (Wave-1B) ─────────────────────────────
    //
    // Read-only in-memory-state accessors backing the Kotlin
    // `WalletMemoryExplorerView` port — ports of Swift's
    // `wallet.inMemorySummary()` / `inMemoryIdentityIds()` /
    // `inMemoryWatchedIdentityIds()`.

    /** In-memory summary of the wallet's Rust-side state. */
    data class InMemorySummary(
        val identitiesCount: Long,
        val watchedCount: Long,
        val lastScannedIndex: Long,
        val trackedAssetLocksCount: Long,
    )

    /**
     * A wallet's in-memory summary — port of Swift's `inMemorySummary()`.
     * Bridges `platform_wallet_get_in_memory_summary`.
     */
    suspend fun inMemorySummary(): InMemorySummary = withContext(Dispatchers.IO) {
        val quad = mapNativeErrors { WalletManagerNative.walletInMemorySummary(handle) }
        InMemorySummary(
            identitiesCount = quad.getOrElse(0) { 0L },
            watchedCount = quad.getOrElse(1) { 0L },
            lastScannedIndex = quad.getOrElse(2) { 0L },
            trackedAssetLocksCount = quad.getOrElse(3) { 0L },
        )
    }

    /**
     * The 32-byte ids of every identity the wallet manages — port of Swift's
     * `inMemoryIdentityIds()`. Bridges
     * `platform_wallet_list_in_memory_identity_ids`.
     */
    suspend fun inMemoryIdentityIds(): List<ByteArray> = withContext(Dispatchers.IO) {
        val flat = mapNativeErrors { WalletManagerNative.walletInMemoryIdentityIds(handle) }
        splitIds(flat)
    }

    /**
     * The 32-byte ids of every out-of-wallet / observed identity — port of
     * Swift's `inMemoryWatchedIdentityIds()`. Bridges
     * `platform_wallet_list_in_memory_watched_identity_ids`.
     */
    suspend fun inMemoryWatchedIdentityIds(): List<ByteArray> = withContext(Dispatchers.IO) {
        val flat = mapNativeErrors { WalletManagerNative.walletInMemoryWatchedIdentityIds(handle) }
        splitIds(flat)
    }

    /** Lifecycle status of a managed identity, mirroring `IdentityStatusFFI`. */
    enum class IdentityStatus {
        UNKNOWN,
        PENDING_CREATION,
        ACTIVE,
        FAILED_CREATION,
        NOT_FOUND,
        ;

        internal companion object {
            fun fromDiscriminant(value: Int): IdentityStatus = when (value) {
                1 -> PENDING_CREATION
                2 -> ACTIVE
                3 -> FAILED_CREATION
                4 -> NOT_FOUND
                else -> UNKNOWN
            }
        }
    }

    /**
     * The in-memory index + lifecycle status of one managed identity — port
     * of the per-identity drill-down in Swift's `WalletMemoryExplorerView`.
     * [index] is the BIP-9 identity index, or `-1` for a watched
     * (out-of-wallet) identity; [watched] reflects which list the id came from.
     */
    data class IdentityState(
        val identityId: ByteArray,
        val index: Long,
        val watched: Boolean,
        val status: IdentityStatus,
    ) {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (other !is IdentityState) return false
            return identityId.contentEquals(other.identityId) &&
                index == other.index &&
                watched == other.watched &&
                status == other.status
        }

        override fun hashCode(): Int {
            var result = identityId.contentHashCode()
            result = 31 * result + index.hashCode()
            result = 31 * result + watched.hashCode()
            result = 31 * result + status.hashCode()
            return result
        }
    }

    /**
     * Snapshot the in-memory index + status of every managed and watched
     * identity — the drill-down backing the Kotlin `WalletMemoryExplorerView`
     * port. Opens a `ManagedIdentity` handle per id
     * (`TokensNative.getManagedIdentity`), reads its index/status, and frees
     * the handle before moving on. Ids that can't be snapshotted are skipped
     * (they may have just been removed by a concurrent sync). No network I/O.
     */
    suspend fun inMemoryIdentityStates(): List<IdentityState> = withContext(Dispatchers.IO) {
        val managed = inMemoryIdentityIds().map { it to false }
        val watched = inMemoryWatchedIdentityIds().map { it to true }
        (managed + watched).mapNotNull { (id, isWatched) ->
            mapNativeErrors {
                // The native side reports an unmanaged / just-removed identity
                // as a platform-wallet NotFound error, not a zero handle, so
                // the raw code is translated back to the zero-handle "skip"
                // signal here — otherwise one removed id would throw through
                // the whole listing (dashpay/platform#4060).
                val identityHandle = translateManagedIdentityNotFoundToZero {
                    TokensNative.getManagedIdentity(handle, id)
                }
                if (identityHandle == 0L) return@mapNativeErrors null
                try {
                    val index = WalletManagerNative.managedIdentityGetIdentityIndex(identityHandle)
                    val status = WalletManagerNative.managedIdentityGetStatus(identityHandle)
                    IdentityState(
                        identityId = id,
                        index = index,
                        watched = isWatched || index < 0,
                        status = IdentityStatus.fromDiscriminant(status),
                    )
                } finally {
                    TokensNative.managedIdentityDestroy(identityHandle)
                }
            }
        }
    }

    /**
     * The advisory "why not" reason a withdrawal preflight records when the
     * account can't fund one — port of the `success_with_message` reason the
     * Swift `WithdrawalPreflightFFI` message carries. Returns null when the
     * withdrawal can proceed (no reason to show) or no message was recorded.
     * Complements [preflightWithdrawal], whose `canWithdraw` flag remains the
     * authoritative gate.
     */
    suspend fun preflightWithdrawalReason(
        accountIndex: Int = 0,
        coreFeePerByte: Int = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            WalletManagerNative.walletPlatformAddressPreflightWithdrawalReason(
                walletHandle = handle,
                accountIndex = accountIndex,
                coreFeePerByte = coreFeePerByte,
            )
        }
    }

    override fun close() {
        cleanable.clean()
    }

    /** Split a flat `byte[]` of concatenated 32-byte ids into per-id rows. */
    private fun splitIds(flat: ByteArray): List<ByteArray> {
        if (flat.size < 32) return emptyList()
        val out = ArrayList<ByteArray>(flat.size / 32)
        var offset = 0
        while (offset + 32 <= flat.size) {
            out.add(flat.copyOfRange(offset, offset + 32))
            offset += 32
        }
        return out
    }

    /**
     * Encode [outputs] to the credit-outputs blob the transfer FFI reads:
     * `u32 rowCount` then per row `u8 addressType, u8[20] hash, u64 credits`.
     */
    private fun encodeCreditOutputs(outputs: List<CreditOutput>): ByteArray {
        val out = java.io.ByteArrayOutputStream()
        val dos = java.io.DataOutputStream(out)
        dos.writeInt(outputs.size)
        for (o in outputs) {
            dos.writeByte(o.addressType)
            dos.write(o.hash)
            dos.writeLong(o.credits)
        }
        return out.toByteArray()
    }

    /**
     * Encode [recipients] to the funding-recipients blob the FFI reads:
     * `u32 rowCount` then per row `u8 addressType, u8[20] hash,
     * u8 hasBalance (0/1), u64 balance`.
     */
    private fun encodeRecipients(recipients: List<FundRecipient>): ByteArray {
        val out = java.io.ByteArrayOutputStream()
        val dos = java.io.DataOutputStream(out)
        dos.writeInt(recipients.size)
        for (r in recipients) {
            dos.writeByte(r.addressType)
            dos.write(r.hash)
            dos.writeByte(if (r.credits != null) 1 else 0)
            dos.writeLong(r.credits ?: 0L)
        }
        return out.toByteArray()
    }

    /**
     * Decode a funding changeset blob (`u32 rowCount` then per row
     * `u8 addressType, u8[20] hash, u64 balance`) into [UpdatedBalance]s.
     */
    private fun decodeChangeset(blob: ByteArray): List<UpdatedBalance> {
        if (blob.size < 4) return emptyList()
        val buffer = java.nio.ByteBuffer.wrap(blob)
        val count = buffer.int
        val out = ArrayList<UpdatedBalance>(count.coerceAtLeast(0))
        repeat(count) {
            if (buffer.remaining() < 1 + 20 + 8) return out
            val addressType = buffer.get().toInt() and 0xFF
            val hash = ByteArray(20)
            buffer.get(hash)
            out.add(UpdatedBalance(addressType, hash, buffer.long))
        }
        return out
    }

    /**
     * Decode the address-balances blob (`u32 rowCount` then per row
     * `u8 addressType, u8[20] hash, u64 balance`) into balance-carrying
     * [FundingInput]s. Malformed / truncated blobs yield the rows decoded so
     * far; a zero-length blob yields an empty list.
     */
    private fun decodeAddressBalances(
        blob: ByteArray,
    ): List<org.dashfoundation.dashsdk.credits.FundingInput> {
        if (blob.size < 4) return emptyList()
        val buffer = java.nio.ByteBuffer.wrap(blob) // big-endian by default
        val count = buffer.int
        val out = ArrayList<org.dashfoundation.dashsdk.credits.FundingInput>(count.coerceAtLeast(0))
        repeat(count) {
            if (buffer.remaining() < 1 + 20 + 8) return out
            val addressType = buffer.get().toInt() and 0xFF
            val hash = ByteArray(20)
            buffer.get(hash)
            val balance = buffer.long
            if (balance > 0) {
                out.add(
                    org.dashfoundation.dashsdk.credits.FundingInput(
                        addressType = addressType,
                        hash = hash,
                        credits = balance,
                    ),
                )
            }
        }
        return out
    }

    /** Runs on [NativeCleaner] or [close]; destroys the handle exactly once. */
    private class HandleCleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val handle = handleRef.getAndSet(0)
            if (handle != 0L) {
                WalletManagerNative.walletDestroy(handle)
            }
        }
    }
}
