package org.dashfoundation.example.ui.shielded

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPoolBalance
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPoolSpendKind
import org.dashfoundation.example.services.tokens.TokenAmounts
import org.dashfoundation.example.ui.wallet.shieldedOperationMayHaveGoneThrough
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.DashAddress
import org.dashfoundation.example.util.DashAddressType
import org.dashfoundation.example.util.toHex

// What the Shielded Tokens screens show and read, apart from Compose: port of
// the helpers in
// `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Core/Views/ShieldedTokenPoolsView.swift`.
//
// Nothing here decides a spend. Every balance, note count, fee and refusal is
// Rust's; these functions only format them and read the user's text into the
// values the wallet calls take (an Orchard address, an identity id, a whole
// number of the token's smallest unit), as the Swift forms do.

/** The rows of one watched token, in the order Rust listed them. */
internal data class ShieldedTokenGroup(
    val tokenId: ByteArray,
    val accounts: List<ShieldedTokenPoolBalance>,
) {
    val tokenIdHex: String get() = tokenId.toHex()
}

/**
 * Group [balances] by token, keeping the order in which Rust listed the
 * tokens and each token's accounts. ← Swift `ShieldedTokenPoolsView.groups`.
 */
internal fun groupShieldedTokenBalances(
    balances: List<ShieldedTokenPoolBalance>,
): List<ShieldedTokenGroup> {
    val byToken = LinkedHashMap<String, MutableList<ShieldedTokenPoolBalance>>()
    for (balance in balances) {
        byToken.getOrPut(balance.tokenId.toHex()) { mutableListOf() }.add(balance)
    }
    return byToken.values.map { rows -> ShieldedTokenGroup(rows.first().tokenId, rows) }
}

/**
 * The app's local record of the row's token, for its name and decimals only,
 * matched by contract id and position. ← Swift `knownToken`.
 */
internal fun knownShieldedToken(
    balance: ShieldedTokenPoolBalance,
    tokens: List<TokenEntity>,
): TokenEntity? = tokens.firstOrNull {
    it.contractId.contentEquals(balance.dataContractId) && it.position == balance.tokenContractPosition
}

/**
 * [amount] with the token's decimals when the app has the token, otherwise
 * the raw count in the token's smallest unit. ← Swift `amountText`.
 */
internal fun shieldedTokenAmountText(amount: ULong, token: TokenEntity?): String =
    if (token == null) "$amount (smallest units)" else TokenAmounts.format(amount, token.decimals)

/** "Scanned X of Y notes (height H)", or "Not scanned yet". ← Swift `scanText`. */
internal fun shieldedTokenScanText(balance: ShieldedTokenPoolBalance): String {
    val scanned = balance.scannedNoteCount ?: return "Not scanned yet"
    val pool = balance.poolNoteCount
    var text = if (pool != null) "Scanned $scanned of $pool notes" else "Scanned $scanned notes"
    balance.scannedHeight?.let { text += " (height $it)" }
    return text
}

/** "1 unspent note" / "N unspent notes". */
internal fun shieldedTokenNoteCountText(count: UInt): String =
    "$count unspent ${if (count == 1u) "note" else "notes"}"

/**
 * The line under Refresh, for the outcome of its sync call. Like Swift, it
 * does not claim this wallet's scan succeeded: each token row shows its own
 * last scan result.
 */
internal fun shieldedTokensRefreshText(status: CheckNowStatus): String? = when (status) {
    CheckNowStatus.Finished -> "Sync requested. Each token shows its own last scan result."
    else -> status.text
}

/**
 * Read [text] as a whole number of the token's smallest unit: ASCII digits
 * only, nothing rounded or converted; null for empty text, a decimal point, a
 * sign, any other character, or a number past `UInt64`.
 * ← Swift `ShieldedTokenSendSheet.smallestUnitAmount`.
 */
internal fun shieldedTokenSmallestUnitAmount(text: String): ULong? {
    if (text.isEmpty() || !text.all { it in '0'..'9' }) return null
    return text.toULongOrNull()
}

/**
 * The 43 raw bytes of [text] when it is an Orchard address on [network];
 * null otherwise. ← Swift `ShieldedTokenSendSheet.recipientRaw`.
 */
internal fun shieldedTokenOrchardRecipient(text: String, network: Network): ByteArray? {
    val trimmed = text.trim()
    // Longer text cannot be an Orchard address, and the generic parser's
    // Base58 fallback is costly on long input: refuse it before decoding.
    if (trimmed.isEmpty() || trimmed.length > MAX_ORCHARD_ADDRESS_LENGTH) return null
    return (DashAddress.parse(trimmed, network) as? DashAddressType.Orchard)?.raw43
}

/**
 * The longest bech32m text an Orchard address can be: an Orchard address is
 * 83 characters on testnet (`tdash1`, 71 data characters, 6 checksum), and
 * BIP-173 caps bech32 strings at 90.
 */
internal const val MAX_ORCHARD_ADDRESS_LENGTH = 90

/** At most 44 Base58 characters encode 32 bytes; longer text is refused before decoding. */
private const val MAX_BASE58_IDENTIFIER_LENGTH = 44

/**
 * The 32-byte identity id [text] decodes to as Base58; null otherwise.
 * ← Swift `ShieldedTokenUnshieldSheet.recipientId`.
 */
internal fun shieldedTokenIdentityRecipient(text: String): ByteArray? {
    val trimmed = text.trim()
    if (trimmed.isEmpty() || trimmed.length > MAX_BASE58_IDENTIFIER_LENGTH) return null
    return Base58.decode(trimmed)?.takeIf { it.size == 32 }
}

/** What a token pool spend form says once its spend call returned or failed as possibly sent. */
internal data class ShieldedTokenSpendResult(val message: String, val mayHaveGoneThrough: Boolean)

/**
 * Shown when Rust accepted the spend: submitted, never complete, since its
 * result proof does not show that it executed.
 */
internal const val SHIELDED_TOKEN_SPEND_SUBMITTED =
    "Submitted, pending until the next shielded sync. Do not send it again."

/**
 * Run [spend] and say what became of it. One that returns is submitted; one
 * that fails with an error Rust marks as possibly sent
 * ([shieldedOperationMayHaveGoneThrough]) is reported without inviting a
 * retry, which would sign a second spend; any other error means nothing was
 * sent and is rethrown for the form's error line. ← Swift `submit()` of
 * `ShieldedTokenSendSheet` / `ShieldedTokenUnshieldSheet`.
 */
internal suspend fun shieldedTokenSpendResult(
    kind: ShieldedTokenPoolSpendKind,
    spend: suspend () -> Unit,
): ShieldedTokenSpendResult = try {
    spend()
    ShieldedTokenSpendResult(SHIELDED_TOKEN_SPEND_SUBMITTED, mayHaveGoneThrough = false)
} catch (e: CancellationException) {
    throw e
} catch (e: Exception) {
    if (!shieldedOperationMayHaveGoneThrough(e)) throw e
    val noun = when (kind) {
        ShieldedTokenPoolSpendKind.ShieldedTransfer -> "transfer"
        ShieldedTokenPoolSpendKind.Unshield -> "unshield"
    }
    ShieldedTokenSpendResult(
        "The $noun may have gone through; a shielded sync confirms it once it lands. " +
            "Do not retry. (${e.message ?: e.toString()})",
        mayHaveGoneThrough = true,
    )
}

/** "≈ X tokens if the token has N decimals (from this app's token cache)." Display only. */
internal fun shieldedTokenDecimalsHint(amount: ULong, token: TokenEntity): String =
    "≈ ${TokenAmounts.format(amount, token.decimals)} tokens if the token has ${token.decimals} " +
        "decimals (from this app's token cache)."

/** The fee line's note counts. ← Swift `ShieldedTokenSpendFeeLine` quoted case. */
internal fun shieldedTokenFeeNotesText(tokenNotes: UInt, feeNotes: UInt): String =
    "Spends $tokenNotes token ${if (tokenNotes == 1u) "note" else "notes"}; " +
        "$feeNotes credit ${if (feeNotes == 1u) "note pays" else "notes pay"} the fee."
