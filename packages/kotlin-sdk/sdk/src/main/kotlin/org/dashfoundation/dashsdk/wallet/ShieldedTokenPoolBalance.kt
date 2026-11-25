package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.ShieldedTokenPoolBalanceData

/**
 * One account's tokens in one token shielded pool a wallet watches, as the
 * local scan knows them. Mirrors the Rust `ShieldedTokenPoolBalance`
 * (`ShieldedTokenPoolBalanceFFI` across the FFI); every value is Rust's,
 * copied as is. Port of Swift's `ShieldedTokenPoolBalance`
 * (`PlatformWalletManagerShieldedSync.swift`).
 * [PlatformWalletManager.shieldedTokenPoolBalances] lists them, one per
 * watched pool and bound account.
 *
 * @property tokenId the watched token's 32-byte id.
 * @property dataContractId the 32-byte id of the contract that defines it.
 * @property tokenContractPosition the token's position in that contract.
 * @property accountIndex the Orchard (ZIP-32) account the notes belong to.
 * @property balance the value of the account's unspent notes that no spend
 *   in flight holds, in the token's smallest unit: what a token pool spend
 *   can use now. Covers exactly the [scannedNoteCount] notes checked.
 * @property unspentNoteCount how many notes make up [balance].
 * @property reservedBalance the value of the unspent notes a submitted spend
 *   still holds, in the token's smallest unit. They leave when a shielded
 *   sync sees them spent, or return to [balance] if the spend can no longer
 *   execute. Not part of [balance].
 * @property scannedNoteCount how many of the pool's notes, from the first
 *   without a gap, were checked for this account; null before the first
 *   scan reached it.
 * @property poolNoteCount the highest note count a page of the pool's scans
 *   proved; null before any. It does not mean a scan covered it.
 * @property scannedHeight the highest height a page of the pool's scans was
 *   proven at; null before any.
 * @property lastError why the last scan of the pool stopped early, if it
 *   did. The next chunk read clears it.
 */
data class ShieldedTokenPoolBalance(
    val tokenId: ByteArray,
    val dataContractId: ByteArray,
    val tokenContractPosition: Int,
    val accountIndex: Int,
    val balance: ULong,
    val unspentNoteCount: UInt,
    val reservedBalance: ULong,
    val scannedNoteCount: ULong?,
    val poolNoteCount: ULong?,
    val scannedHeight: ULong?,
    val lastError: String?,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is ShieldedTokenPoolBalance) return false
        return tokenId.contentEquals(other.tokenId) &&
            dataContractId.contentEquals(other.dataContractId) &&
            tokenContractPosition == other.tokenContractPosition &&
            accountIndex == other.accountIndex &&
            balance == other.balance &&
            unspentNoteCount == other.unspentNoteCount &&
            reservedBalance == other.reservedBalance &&
            scannedNoteCount == other.scannedNoteCount &&
            poolNoteCount == other.poolNoteCount &&
            scannedHeight == other.scannedHeight &&
            lastError == other.lastError
    }

    override fun hashCode(): Int {
        var result = tokenId.contentHashCode()
        result = 31 * result + dataContractId.contentHashCode()
        result = 31 * result + tokenContractPosition
        result = 31 * result + accountIndex
        result = 31 * result + balance.hashCode()
        result = 31 * result + reservedBalance.hashCode()
        return result
    }

    internal companion object {
        /**
         * Copy a transport row. Each `has*` flag gates its value: a false
         * flag becomes null, never the zero the native row holds. Unsigned
         * values are read back from their bit patterns.
         */
        fun fromNative(row: ShieldedTokenPoolBalanceData): ShieldedTokenPoolBalance =
            ShieldedTokenPoolBalance(
                tokenId = row.tokenId.copyOf(),
                dataContractId = row.dataContractId.copyOf(),
                tokenContractPosition = row.tokenContractPosition,
                accountIndex = row.accountIndex,
                balance = row.balance.toULong(),
                unspentNoteCount = row.unspentNoteCount.toUInt(),
                reservedBalance = row.reservedBalance.toULong(),
                scannedNoteCount = if (row.hasScannedNoteCount) row.scannedNoteCount.toULong() else null,
                poolNoteCount = if (row.hasPoolNoteCount) row.poolNoteCount.toULong() else null,
                scannedHeight = if (row.hasScannedHeight) row.scannedHeight.toULong() else null,
                lastError = row.lastError,
            )
    }
}

/**
 * Which token pool spend [PlatformWalletManager.shieldedTokenSpendPreview]
 * asks about. Mirrors the Rust `TokenPoolSpendKind`; [raw] is the FFI
 * `kind` byte. Port of Swift's `ShieldedTokenPoolSpendKind`.
 */
enum class ShieldedTokenPoolSpendKind(val raw: Byte) {
    /** [PlatformWalletManager.shieldedTokenTransfer] (type 26): the tokens move within the pool. */
    ShieldedTransfer(0),

    /**
     * [PlatformWalletManager.shieldedTokenUnshield] (type 27): the tokens
     * leave the pool into an identity's token balance.
     */
    Unshield(1),
}

/**
 * What a token pool spend would take from an account's notes, as Rust
 * chooses them ([PlatformWalletManager.shieldedTokenSpendPreview]). Mirrors
 * the Rust `ShieldedTokenSpendPreview`; every value is Rust's. Port of
 * Swift's `ShieldedTokenSpendPreview`.
 *
 * @property tokenNotes the token notes the spend would spend.
 * @property feeNotes the credit pool notes that would pay its fee.
 * @property fee the fee, in credits, paid from the credit pool.
 */
data class ShieldedTokenSpendPreview(
    val tokenNotes: UInt,
    val feeNotes: UInt,
    val fee: ULong,
) {
    internal companion object {
        /**
         * Read the native `[fee, tokenNotes, feeNotes]` array. The counts
         * are u32 widened without a sign, the fee is a u64's bit pattern.
         */
        fun fromNative(values: LongArray): ShieldedTokenSpendPreview {
            check(values.size == 3) { "spend preview must have 3 values, got ${values.size}" }
            return ShieldedTokenSpendPreview(
                tokenNotes = values[1].toUInt(),
                feeNotes = values[2].toUInt(),
                fee = values[0].toULong(),
            )
        }
    }
}
