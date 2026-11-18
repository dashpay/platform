package org.dashfoundation.dashsdk.wallet

import java.util.concurrent.atomic.AtomicLong

/**
 * Availability of a wallet's local shielded ledger, read without contacting
 * the network. Port of Swift `ShieldedLocalBalanceState`
 * (`PlatformWalletManagerShieldedLocalBalance.swift`).
 */
sealed interface ShieldedLocalBalanceState {
    /** No shielded coordinator or bound account exists for this wallet. */
    data object Unbound : ShieldedLocalBalanceState

    /**
     * Accounts are bound, but neither restoration nor a completed scan in
     * this session establishes the ledger of every account. There is no
     * number to show.
     */
    data object RestoreIncomplete : ShieldedLocalBalanceState

    data class Ready(val snapshot: ShieldedLocalBalanceSnapshot) : ShieldedLocalBalanceState
}

/**
 * One coherent read of a wallet's local shielded balances.
 *
 * @param ordinarySpendableCredits the wallet's shielded balance as Rust
 *   computes it: spendable credits of its ordinary accounts. Identity
 *   accounts are in [accounts] but not in this total, because an ordinary
 *   send cannot spend their notes. Show this number; do not add up
 *   [accounts] or stored notes.
 * @param accounts every bound account by its ZIP-32 index, identity accounts
 *   and accounts holding nothing included.
 */
data class ShieldedLocalBalanceSnapshot(
    val ordinarySpendableCredits: Long,
    val accounts: Map<Int, ShieldedLocalAccountBalance>,
)

/**
 * @param spendableCredits credits a send from this account can select now:
 *   unspent notes, leaving out the ones a pending spend has reserved. A new
 *   network scan may change it.
 * @param lastScannedIndex the account's scan coverage; null when no completed
 *   scan is known, which is distinct from a scan that ended at index zero.
 */
data class ShieldedLocalAccountBalance(
    val spendableCredits: Long,
    val lastScannedIndex: Long?,
    val source: ShieldedBalanceSource,
)

/** What backs an account's local balance. */
enum class ShieldedBalanceSource {
    /** Keys are bound, but no stored ledger or completed scan is known. Zero is not a verdict. */
    NO_HISTORY,

    /** Notes or a scan watermark were restored from local persistence. */
    RESTORED,

    /** A complete scan succeeded in this manager session. */
    SCANNED_THIS_SESSION,
}

/**
 * A local balance read finished, but a shielded bind or clear of the same
 * manager ran while it was in flight, so its accounts may belong to the
 * binding that was replaced. Nothing was changed; read again.
 */
class ShieldedLocalBalanceBindingChanged : Exception(
    "Shielded binding changed during the local balance snapshot; request a fresh read",
)

/**
 * Run [read] and return its value only if [generation] did not move while
 * it ran; otherwise throw [ShieldedLocalBalanceBindingChanged].
 */
internal inline fun <T> fencedByShieldedBinding(generation: AtomicLong, read: () -> T): T {
    val before = generation.get()
    val value = read()
    if (before != generation.get()) throw ShieldedLocalBalanceBindingChanged()
    return value
}

private const val HEADER_LONGS = 3
private const val ACCOUNT_LONGS = 5

/**
 * Decode the `long[]` that `FundingNative.localShieldedBalanceSnapshot`
 * returns (its layout is documented there). Refuses anything it cannot read
 * exactly, so a layout mismatch never turns into a balance.
 */
internal fun decodeLocalShieldedBalance(values: LongArray): ShieldedLocalBalanceState {
    check(values.size >= HEADER_LONGS) { "Local shielded balance snapshot is truncated" }
    val status = values[0]
    val count = values[2]
    check(count >= 0 && values.size.toLong() == HEADER_LONGS + count * ACCOUNT_LONGS) {
        "Local shielded balance snapshot has ${values.size} values for $count accounts"
    }
    when (status) {
        0L -> {
            check(count == 0L) { "Unbound shielded snapshot contains accounts" }
            return ShieldedLocalBalanceState.Unbound
        }
        1L -> {
            check(count == 0L) { "Incomplete shielded snapshot contains accounts" }
            return ShieldedLocalBalanceState.RestoreIncomplete
        }
        2L -> Unit
        else -> error("Unknown local shielded balance status: $status")
    }
    check(count > 0) { "Ready shielded snapshot has no accounts" }
    val accounts = LinkedHashMap<Int, ShieldedLocalAccountBalance>(count.toInt())
    for (row in 0 until count.toInt()) {
        val at = HEADER_LONGS + row * ACCOUNT_LONGS
        val index = values[at]
        check(index in 0..Int.MAX_VALUE) { "Shielded account index out of range: $index" }
        val hasLastScannedIndex = when (values[at + 2]) {
            0L -> false
            1L -> true
            else -> error("Unknown last scanned index flag: ${values[at + 2]}")
        }
        val source = when (values[at + 4]) {
            0L -> ShieldedBalanceSource.NO_HISTORY
            1L -> ShieldedBalanceSource.RESTORED
            2L -> ShieldedBalanceSource.SCANNED_THIS_SESSION
            else -> error("Unknown local shielded balance source: ${values[at + 4]}")
        }
        val previous = accounts.put(
            index.toInt(),
            ShieldedLocalAccountBalance(
                spendableCredits = values[at + 1],
                lastScannedIndex = if (hasLastScannedIndex) values[at + 3] else null,
                source = source,
            ),
        )
        check(previous == null) { "Duplicate shielded account in local snapshot: $index" }
    }
    return ShieldedLocalBalanceState.Ready(
        ShieldedLocalBalanceSnapshot(
            ordinarySpendableCredits = values[1],
            accounts = accounts,
        ),
    )
}
