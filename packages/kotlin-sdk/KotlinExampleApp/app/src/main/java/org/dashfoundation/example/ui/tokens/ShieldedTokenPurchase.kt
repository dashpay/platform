package org.dashfoundation.example.ui.tokens

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.wallet.ShieldedTokenPurchaseAvailability
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.wallet.shieldedOperationMayHaveGoneThrough

// Paying for a direct purchase from the shielded pool (transition type 28):
// the pieces `PurchaseForm` adds for it. Port of the shielded half of
// `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/TokenActions/TokenPurchaseActionView.swift`;
// the accessibility identifiers are reused verbatim as test tags.
//
// Nothing here decides whether the purchase can be made or what became of
// it. Whether the shielded payment is offered is Rust's answer
// (`shieldedTokenPurchaseAvailability`), whether the notes cover price plus
// fee is Rust's refusal, and a purchase that returns is only ever reported
// as submitted.

/** Where a direct purchase is paid from. ← Swift `FundingSource`. */
internal enum class PurchaseFundingSource(val label: String) {
    IdentityCredits("Identity credits"),
    ShieldedPool("Shielded pool"),
}

/**
 * Rust's answer to whether the token can be bought from the shielded pool.
 * [Failed] when the question itself could not be asked.
 * ← Swift `ShieldedAvailabilityState`.
 */
internal sealed interface ShieldedPurchaseAvailabilityState {
    data object Loading : ShieldedPurchaseAvailabilityState

    data class Loaded(val availability: ShieldedTokenPurchaseAvailability) :
        ShieldedPurchaseAvailabilityState

    data object Failed : ShieldedPurchaseAvailabilityState

    /** The native library was built without shielded support: nothing to ask. */
    data object Unsupported : ShieldedPurchaseAvailabilityState

    /** The shielded payment is offered on Rust's "available" answer only. */
    val offersShieldedPayment: Boolean
        get() = this == Loaded(ShieldedTokenPurchaseAvailability.Available)
}

/** Why the shielded payment is not offered, and whether asking again may help. */
internal data class ShieldedUnavailableNote(
    val text: String,
    val isError: Boolean,
    val offersRetry: Boolean,
)

/** The note for [state], or null when the shielded payment is offered. */
internal fun shieldedUnavailableNote(state: ShieldedPurchaseAvailabilityState): ShieldedUnavailableNote? =
    when (state) {
        ShieldedPurchaseAvailabilityState.Loading -> ShieldedUnavailableNote(
            "Checking whether this token can be bought from the shielded pool…",
            isError = false,
            offersRetry = false,
        )
        ShieldedPurchaseAvailabilityState.Failed -> UNREADABLE_NOTE
        ShieldedPurchaseAvailabilityState.Unsupported -> ShieldedUnavailableNote(
            "This build has no shielded support, so paying from the shielded pool isn't offered.",
            isError = false,
            offersRetry = false,
        )
        is ShieldedPurchaseAvailabilityState.Loaded -> when (state.availability) {
            ShieldedTokenPurchaseAvailability.Available -> null
            ShieldedTokenPurchaseAvailability.NoShieldedPool -> ShieldedUnavailableNote(
                "This token has no shielded pool, so it can't be bought from the shielded pool.",
                isError = false,
                offersRetry = false,
            )
            ShieldedTokenPurchaseAvailability.ConfigurationUnreadable -> UNREADABLE_NOTE
        }
    }

private val UNREADABLE_NOTE = ShieldedUnavailableNote(
    "Couldn't read this token's configuration, so paying from the shielded pool isn't offered.",
    isError = true,
    offersRetry = true,
)

/**
 * Shown after Platform accepted the purchase. Submitted, never complete:
 * the transition's result proof binds neither the notes spent nor the
 * tokens minted, so only a shielded sync that finds the purchase confirms it.
 */
internal const val SHIELDED_PURCHASE_SUBMITTED =
    "Purchase submitted. Platform accepted it; a shielded sync confirms it once it lands, " +
        "and it shows as pending in Shielded Activity until then. Do not retry. The tokens " +
        "go to this wallet's shielded token pool; Wallet > Shielded Tokens shows them after a " +
        "shielded sync."

/**
 * Shown when the purchase may already be on chain and Rust keeps its notes
 * reserved: reported without inviting a retry, which would sign a second
 * purchase.
 */
internal const val SHIELDED_PURCHASE_MAY_HAVE_GONE_THROUGH =
    "The purchase may have gone through; a shielded sync confirms it once it lands. Do not retry."

/**
 * Run [purchase] and return what to tell the buyer. A purchase that returns
 * is submitted; one that fails with an error Rust marks as possibly sent
 * ([shieldedOperationMayHaveGoneThrough]) is reported the same careful way;
 * any other error means nothing was sent and is rethrown for the form's
 * error dialog. ← Swift `submitShielded`.
 */
internal suspend fun shieldedPurchaseOutcomeMessage(purchase: suspend () -> Unit): String =
    try {
        purchase()
        SHIELDED_PURCHASE_SUBMITTED
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        if (shieldedOperationMayHaveGoneThrough(e)) SHIELDED_PURCHASE_MAY_HAVE_GONE_THROUGH else throw e
    }

/**
 * Whether Buy is enabled for the shielded payment, beyond a valid amount
 * and price: Rust offers it for this token, the identity's wallet is known,
 * and its shielded balance is known and not zero, as the send screen's
 * shielded flows require. Whether the balance covers price plus fee is
 * Rust's call (an insufficient-balance error). ← Swift `canSubmit`.
 */
internal fun canSubmitShieldedPurchase(
    availability: ShieldedPurchaseAvailabilityState,
    hasWallet: Boolean,
    shieldedBalance: Long?,
): Boolean = availability.offersShieldedPayment && hasWallet && shieldedBalance != null && shieldedBalance > 0

/** The agreed total plus [fee], for display; null without either, or on overflow. */
internal fun priceWithFee(totalCost: ULong?, fee: Long?): ULong? {
    if (totalCost == null || fee == null || fee < 0) return null
    val total = totalCost + fee.toULong()
    return if (total < totalCost) null else total
}

/**
 * The "Pay with" section: a choice between the two funding sources when
 * Rust offers the shielded payment, otherwise identity credits with the
 * reason the shielded payment is not offered.
 */
@Composable
internal fun PurchaseFundingSection(
    availability: ShieldedPurchaseAvailabilityState,
    selected: PurchaseFundingSource,
    enabled: Boolean,
    onSelect: (PurchaseFundingSource) -> Unit,
    onRetry: () -> Unit,
) {
    FormSection(title = "Pay with") {
        val note = shieldedUnavailableNote(availability)
        if (note == null) {
            val sources = PurchaseFundingSource.entries
            SingleChoiceSegmentedButtonRow(
                modifier = Modifier.fillMaxWidth().testTag("tokenPurchase.fundingSource"),
            ) {
                sources.forEachIndexed { index, source ->
                    SegmentedButton(
                        selected = selected == source,
                        onClick = { onSelect(source) },
                        enabled = enabled,
                        shape = SegmentedButtonDefaults.itemShape(index = index, count = sources.size),
                        modifier = Modifier.testTag("tokenPurchase.fundingSource.${source.name}"),
                    ) { Text(source.label) }
                }
            }
        } else {
            LabeledContent("Pay with", PurchaseFundingSource.IdentityCredits.label)
            Text(
                note.text,
                style = MaterialTheme.typography.bodySmall,
                color = if (note.isError) {
                    MaterialTheme.colorScheme.error
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
                // The loading line carries no id on iOS either.
                modifier = if (availability == ShieldedPurchaseAvailabilityState.Loading) {
                    Modifier
                } else {
                    Modifier.testTag("tokenPurchase.shieldedUnavailable")
                },
            )
            if (note.offersRetry) {
                TextButton(
                    onClick = onRetry,
                    modifier = Modifier.testTag("tokenPurchase.shieldedUnavailable.retry"),
                ) { Text("Retry") }
            }
        }
        Text(
            when (selected) {
                PurchaseFundingSource.IdentityCredits ->
                    "Direct purchase debits credits from this identity in exchange for the " +
                        "requested tokens. The buyer pays the configured price set by the " +
                        "token's pricing controller."
                PurchaseFundingSource.ShieldedPool ->
                    "The price and a pool fee are paid from this wallet's shielded balance; no " +
                        "identity is involved. The tokens are minted to this wallet's own " +
                        "address in the token's shielded pool."
            },
            style = MaterialTheme.typography.bodyMedium,
        )
    }
}

/**
 * What the shielded payment draws on: the wallet's account 0 shielded
 * balance as Rust pushes it, Rust's fee estimate for a purchase paid from
 * one or two notes, and where the tokens go.
 */
@Composable
internal fun ShieldedPoolSection(shieldedBalance: Long?, feeEstimate: Long?, totalCost: ULong?) {
    FormSection(title = "Shielded pool") {
        LabeledContent(
            "Shielded balance",
            shieldedBalance?.let { "$it credits" } ?: "Not known yet",
            modifier = Modifier.testTag("tokenPurchase.shieldedBalance"),
        )
        if (feeEstimate != null) {
            LabeledContent("Pool fee (estimate)", "$feeEstimate credits")
            priceWithFee(totalCost, feeEstimate)?.let { total ->
                LabeledContent("Price + fee", "$total credits")
            }
            Text(
                "The estimate is for a purchase paid from one or two notes; more notes raise the fee.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Text(
            "The purchased tokens go to this wallet's shielded token pool. They appear in " +
                "Wallet > Shielded Tokens after a shielded sync, not in the token list.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.testTag("tokenPurchase.shieldedTokensNote"),
        )
    }
}
