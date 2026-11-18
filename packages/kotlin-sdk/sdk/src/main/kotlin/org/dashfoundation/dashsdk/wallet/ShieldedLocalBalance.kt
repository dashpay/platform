package org.dashfoundation.dashsdk.wallet

/**
 * One wallet's shielded balance as Rust last announced it
 * (`on_shielded_balance_changed_fn`). Rust sends an update whenever the
 * wallet's balance state changes: a scan ends (also when it fails), a spend
 * reserves or releases notes, a bind changes the accounts, a Clear empties
 * the ledger. A host keeps the latest update per wallet and shows it; it
 * never reads or adds up a balance itself. An update is the latest state
 * Rust read, not one per change: notes reserved and released again before
 * Rust reads produce none, and a revision names a delivered state, not a
 * ledger change.
 *
 * @param revision counts this wallet's updates, starting at 1. An update
 *   with a lower revision than the one held is older and is dropped.
 */
data class ShieldedBalanceUpdate(
    val walletId: ByteArray,
    val revision: Long,
    val state: ShieldedLocalBalanceState,
) {
    override fun equals(other: Any?): Boolean =
        other is ShieldedBalanceUpdate &&
            walletId.contentEquals(other.walletId) &&
            revision == other.revision &&
            state == other.state

    override fun hashCode(): Int =
        31 * (31 * walletId.contentHashCode() + revision.hashCode()) + state.hashCode()
}

/**
 * Availability of a wallet's local shielded ledger. Port of Swift
 * `ShieldedLocalBalanceState` (`PlatformWalletManagerShieldedLocalBalance.swift`).
 */
sealed interface ShieldedLocalBalanceState {
    /**
     * No shielded account is registered for this wallet. Rust did not read a
     * ledger, so this says nothing about what the wallet holds: it is not a
     * known zero.
     */
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
 * One coherent view of a wallet's local shielded balances.
 *
 * @param ordinarySpendableCredits spendable credits of the wallet's ordinary
 *   accounts, as Rust computes them. Identity accounts are in [accounts] but
 *   not in this total, because an ordinary send cannot spend their notes.
 *   Do not add up [accounts] or stored notes instead.
 * @param ordinaryBalanceEstablished whether every ordinary account has
 *   evidence behind its amount (restored notes or a completed scan). When
 *   false, [ordinarySpendableCredits] is only what is selectable locally
 *   right now: a wallet restored without history reads zero here before its
 *   first scan, and that zero is not the wallet's balance. Use
 *   [ordinaryBalance] to show a balance.
 * @param accounts every bound account by its ZIP-32 index, identity accounts
 *   and accounts holding nothing included.
 */
data class ShieldedLocalBalanceSnapshot(
    val ordinarySpendableCredits: Long,
    val ordinaryBalanceEstablished: Boolean,
    val accounts: Map<Int, ShieldedLocalAccountBalance>,
) {
    /** The wallet's shielded balance, or null while it is not established. */
    val ordinaryBalance: Long?
        get() = ordinarySpendableCredits.takeIf { ordinaryBalanceEstablished }

    /**
     * What a send from [account] can spend, or null when the account is not
     * bound or its amount has no evidence behind it yet.
     */
    fun accountBalance(account: Int): Long? =
        accounts[account]
            ?.takeIf { it.source != ShieldedBalanceSource.NO_HISTORY }
            ?.spendableCredits
}

/**
 * @param spendableCredits credits a send from this account can select now:
 *   unspent notes, leaving out the ones a pending spend has reserved.
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

/** Number of `long` values per account row in a balance update. */
private const val ACCOUNT_LONGS = 5

/**
 * Build a [ShieldedBalanceUpdate] from the arguments of
 * `NativeWalletEventBridge.onShieldedBalanceChanged` (their layout is
 * documented there). Refuses anything it cannot read exactly, so a layout
 * mismatch never turns into a balance.
 */
internal fun decodeShieldedBalanceUpdate(
    walletId: ByteArray,
    revision: Long,
    status: Int,
    ordinarySpendableCredits: Long,
    ordinaryBalanceEstablished: Boolean,
    accounts: LongArray,
): ShieldedBalanceUpdate {
    check(walletId.size == 32) { "Shielded balance update has a ${walletId.size}-byte wallet id" }
    check(revision > 0) { "Shielded balance update has revision $revision" }
    check(accounts.size % ACCOUNT_LONGS == 0) {
        "Shielded balance update has ${accounts.size} account values"
    }
    val state = when (status) {
        0 -> {
            check(accounts.isEmpty()) { "Unbound shielded balance update contains accounts" }
            ShieldedLocalBalanceState.Unbound
        }
        1 -> {
            check(accounts.isEmpty()) { "Incomplete shielded balance update contains accounts" }
            ShieldedLocalBalanceState.RestoreIncomplete
        }
        2 -> ShieldedLocalBalanceState.Ready(
            ShieldedLocalBalanceSnapshot(
                ordinarySpendableCredits = ordinarySpendableCredits,
                ordinaryBalanceEstablished = ordinaryBalanceEstablished,
                accounts = decodeAccounts(accounts),
            ),
        )
        else -> error("Unknown shielded balance status: $status")
    }
    return ShieldedBalanceUpdate(walletId.copyOf(), revision, state)
}

private fun decodeAccounts(values: LongArray): Map<Int, ShieldedLocalAccountBalance> {
    check(values.isNotEmpty()) { "Ready shielded balance update has no accounts" }
    val accounts = LinkedHashMap<Int, ShieldedLocalAccountBalance>(values.size / ACCOUNT_LONGS)
    for (at in values.indices step ACCOUNT_LONGS) {
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
            else -> error("Unknown shielded balance source: ${values[at + 4]}")
        }
        val previous = accounts.put(
            index.toInt(),
            ShieldedLocalAccountBalance(
                spendableCredits = values[at + 1],
                lastScannedIndex = if (hasLastScannedIndex) values[at + 3] else null,
                source = source,
            ),
        )
        check(previous == null) { "Duplicate shielded account in balance update: $index" }
    }
    // Shared by every collector of the published update: read-only.
    return java.util.Collections.unmodifiableMap(accounts)
}

/** The key a wallet's update is held under: its id in lowercase hex. */
internal fun shieldedBalanceKey(walletId: ByteArray): String =
    walletId.joinToString("") { "%02x".format(it) }

/**
 * The latest update per wallet after [update] arrived: it replaces the one
 * held unless that one has the same or a higher revision, so an update
 * delivered late never overwrites a newer one.
 */
internal fun latestShieldedBalances(
    held: Map<String, ShieldedBalanceUpdate>,
    update: ShieldedBalanceUpdate,
): Map<String, ShieldedBalanceUpdate> {
    val key = shieldedBalanceKey(update.walletId)
    val current = held[key]
    return if (current != null && current.revision >= update.revision) {
        held
    } else {
        held + (key to update)
    }
}
