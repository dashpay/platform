package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import java.lang.ref.Cleaner
import java.util.concurrent.atomic.AtomicLong

/**
 * Thin per-wallet wrapper over a native `PlatformWallet` handle — port of
 * `ManagedPlatformWallet.swift`.
 *
 * Like the Swift type, this **owns the wallet handle** and destroys it
 * (`platform_wallet_destroy`, which drops the manager's `Arc` clone) via
 * [close] or the [Cleaner] backstop. The Rust `PlatformWalletManager`
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
) : AutoCloseable {

    private val handleRef = AtomicLong(handle)
    private val cleanable = CLEANER.register(this, HandleCleanup(handleRef))

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
        get() = org.dashfoundation.dashsdk.tokens.Tokens(handle)

    /**
     * Group-action discovery bound to this wallet's handle — the Kotlin
     * analog of the Swift `TokenGroupActionQueries.swift` extension.
     */
    val groups: org.dashfoundation.dashsdk.tokens.Groups
        get() = org.dashfoundation.dashsdk.tokens.Groups(handle)

    /**
     * DashPay contact/payment surface bound to this wallet's handle —
     * the Kotlin analog of the Swift `ManagedPlatformWallet` DashPay
     * extension.
     */
    val dashpay: org.dashfoundation.dashsdk.tokens.Dashpay
        get() = org.dashfoundation.dashsdk.tokens.Dashpay(handle)

    /**
     * Data-contract create surface bound to this wallet's handle — the
     * Kotlin analog of Swift's `ManagedPlatformWallet.createDataContract`.
     */
    val dataContracts: org.dashfoundation.dashsdk.identity.DataContracts
        get() = org.dashfoundation.dashsdk.identity.DataContracts(handle)

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
     * Build, sign, and broadcast a Core payment to [recipients] — port of
     * Swift's `ManagedCoreWallet.sendToAddresses`.
     *
     * One Rust call does the whole thing (acquire core handle → build +
     * sign via the resolver-backed core signer → broadcast → release
     * handle); Kotlin only marshals the recipient list. Returns the
     * serialized signed transaction bytes.
     *
     * @param coreSignerHandle the manager's `MnemonicResolverHandle`
     *   (`PlatformWalletManager.mnemonicResolverHandle`) — used for the
     *   Core ECDSA signatures. No private key crosses the boundary.
     */
    suspend fun sendToAddresses(
        recipients: List<Pair<String, Long>>,
        coreSignerHandle: Long,
        accountType: AccountType = AccountType.BIP44,
        accountIndex: Int = 0,
    ): ByteArray = withContext(Dispatchers.IO) {
        val addresses = recipients.map { it.first }.toTypedArray()
        val amounts = recipients.map { it.second }.toLongArray()
        mapNativeErrors {
            WalletManagerNative.walletCoreSendToAddresses(
                walletHandle = handle,
                accountType = accountType.ffiValue,
                accountIndex = accountIndex,
                addresses = addresses,
                amounts = amounts,
                coreSignerHandle = coreSignerHandle,
            )
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
    ): List<UpdatedBalance> = withContext(Dispatchers.IO) {
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
    ): List<UpdatedBalance> = withContext(Dispatchers.IO) {
        require(outPointTxid.size == 32) {
            "outPointTxid must be exactly 32 bytes, got ${outPointTxid.size}"
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

    override fun close() {
        cleanable.clean()
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

    /** Runs on [Cleaner] or [close]; destroys the handle exactly once. */
    private class HandleCleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val handle = handleRef.getAndSet(0)
            if (handle != 0L) {
                WalletManagerNative.walletDestroy(handle)
            }
        }
    }

    private companion object {
        val CLEANER: Cleaner = Cleaner.create()
    }
}
