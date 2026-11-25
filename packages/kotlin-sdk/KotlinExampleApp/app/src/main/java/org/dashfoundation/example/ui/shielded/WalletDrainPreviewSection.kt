package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.WalletDrainPreview
import org.dashfoundation.dashsdk.wallet.shieldedWalletDrainPreview
import org.dashfoundation.example.ui.components.FormSection
import java.util.Locale

/**
 * A Max estimate together with the scope it was read for. Submit checks the
 * scope against the current inputs, so a quote for one account index (or
 * wallet, or manager) can never gate a transfer of another. Port of
 * `WalletDrainQuote` in
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/WalletDrainPreviewSection.swift`.
 */
class WalletDrainQuote(
    private val manager: PlatformWalletManager,
    walletId: ByteArray,
    val fundingAccountIndex: Int,
    val preview: WalletDrainPreview,
) {
    private val walletId = walletId.copyOf()

    fun matches(manager: PlatformWalletManager, walletId: ByteArray, fundingAccountIndex: Int): Boolean =
        this.manager === manager &&
            this.walletId.contentEquals(walletId) &&
            this.fundingAccountIndex == fundingAccountIndex
}

/**
 * Read-only estimate for a Core to shielded Max transfer, shown in place of
 * the amount field. Port of `WalletDrainPreviewSection` (same Swift file);
 * the accessibility identifiers are reused verbatim as test tags.
 *
 * Every figure is Rust's
 * ([org.dashfoundation.dashsdk.wallet.shieldedWalletDrainPreview]). The
 * loaded estimate is handed to [onQuote] with the scope it was read for, and
 * null whenever the inputs change or a read is in progress.
 */
@Composable
fun WalletDrainPreviewSection(
    manager: PlatformWalletManager,
    walletId: ByteArray,
    fundingAccountIndex: Int,
    quote: WalletDrainQuote?,
    onQuote: (WalletDrainQuote?) -> Unit,
) {
    var isLoading by remember { mutableStateOf(false) }
    var loadError by remember { mutableStateOf<String?>(null) }
    var refreshTick by remember { mutableIntStateOf(0) }

    LaunchedEffect(manager, walletId.contentHashCode(), fundingAccountIndex, refreshTick) {
        // Read for the inputs captured here and stored under that scope, so a
        // read that finishes after they changed cannot pass for the new ones.
        isLoading = true
        loadError = null
        onQuote(null)
        try {
            val loaded = manager.shieldedWalletDrainPreview(walletId, fundingAccountIndex)
            onQuote(WalletDrainQuote(manager, walletId, fundingAccountIndex, loaded))
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            loadError = "Could not estimate: ${e.message ?: e}"
        }
        isLoading = false
    }

    FormSection(title = "Max (estimates)") {
        val current = quote?.takeIf { it.matches(manager, walletId, fundingAccountIndex) }
        when {
            current != null -> PreviewRows(current.preview)
            loadError != null -> Text(
                loadError.orEmpty(),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.testTag("shieldFromCore.maxPreview.status"),
            )
            isLoading -> Text(
                "Estimating…",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        TextButton(
            onClick = { refreshTick++ },
            enabled = !isLoading,
            modifier = Modifier.testTag("shieldFromCore.maxPreview.refreshButton"),
        ) { Text("Refresh") }
        Text(
            "No change output is created. Coins that are not final yet, locked coins " +
                "and the CoinJoin account are not swept. The figures can differ when the " +
                "transfer runs, because inputs reserved by other operations are counted here.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun PreviewRows(preview: WalletDrainPreview) {
    ValueRow(
        "Swept from Core (est.)",
        "${formatDuffs(preview.candidateBalanceDuffs)}, ${preview.candidateInputCount} inputs",
        "shieldFromCore.maxPreview.swept",
    )
    ValueRow(
        "Core network fee (est.)",
        preview.estimatedL1FeeDuffs?.let(::formatDuffs) ?: "not available",
        "shieldFromCore.maxPreview.l1Fee",
    )
    ValueRow(
        "Shielded pool fee (est.)",
        formatCredits(preview.poolFeeCredits),
        "shieldFromCore.maxPreview.poolFee",
    )
    ValueRow(
        "You receive, shielded (est.)",
        preview.estimatedRecipientCredits?.let(::formatCredits) ?: "not available",
        "shieldFromCore.maxPreview.receive",
    )
    val (text, color) = when (preview.status) {
        WalletDrainPreview.Status.Ready ->
            "Ready" to MaterialTheme.colorScheme.onSurfaceVariant
        WalletDrainPreview.Status.InsufficientFunds ->
            "The balance does not cover the fees, so there is nothing to shield." to
                MaterialTheme.colorScheme.error
        WalletDrainPreview.Status.InputLimitMayBeExceeded ->
            "This wallet has more inputs than one asset-lock transaction can take. " +
                "The transfer may be refused." to WARNING
    }
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = color,
        modifier = Modifier.testTag("shieldFromCore.maxPreview.status"),
    )
}

@Composable
private fun ValueRow(label: String, value: String, tag: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            // One node per row, as the Swift row combines its children.
            .semantics(mergeDescendants = true) {}
            .testTag(tag),
    ) {
        Text(label, style = MaterialTheme.typography.bodyMedium)
        Text(
            value,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.End,
            modifier = Modifier.weight(1f).padding(start = 8.dp),
        )
    }
}

private val WARNING = Color(0xFFB26A00)

private fun formatDuffs(duffs: Long): String =
    String.format(Locale.US, "%.8f DASH", duffs / 100_000_000.0)

private fun formatCredits(credits: Long): String =
    String.format(Locale.US, "%.8f DASH (credits)", credits / 100_000_000_000.0)
