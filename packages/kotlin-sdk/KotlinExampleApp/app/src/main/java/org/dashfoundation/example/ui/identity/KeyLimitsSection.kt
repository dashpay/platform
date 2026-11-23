package org.dashfoundation.example.ui.identity

import android.util.Log
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.UpdateKeyLimits
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.KeyLimitsFormatting

/** Whether the key carries a protocol 14 budget or expiry. A fact of the row, not a rule. */
val PublicKeyEntity.hasLimits: Boolean get() = totalBudget != null || expiresAt != null

/** Rust's answer to "can this key's limits be raised at all". */
private sealed interface UpdateLimitsGate {
    data object Allowed : UpdateLimitsGate
    data class Refused(val reason: String) : UpdateLimitsGate
}

/**
 * The key detail screen's **Limits** section: port of `KeyDetailView.limitsSection` in
 * `KeyDetailView.swift`. Shows the key's total budget as the key row last synced it, what
 * Platform says is left of it (a separate read, fetched again when the total changes, so it
 * is labelled apart: the two can come from different blocks until #6064 reads both from one
 * proof), and its expiry. The
 * **Update Limits** button (`keyLimits.update.open`) is enabled only when Rust's no-change
 * preview answers, and otherwise disabled with Rust's reason
 * (`keyLimits.update.unavailableReason`). The first time, the wallet's copy of the identity
 * is refreshed from Platform with a proof (best effort), so a key added or raised on another
 * client is not refused as unknown. A raise reaches this screen through the key row the
 * wallet persists, which re-keys both reads.
 */
@Composable
fun KeyLimitsSection(
    identityIdHex: String,
    identityIdBase58: String,
    identityId: ByteArray,
    key: PublicKeyEntity,
    manager: PlatformWalletManager?,
    wallet: ManagedPlatformWallet?,
    navController: NavHostController,
) {
    val appState = LocalAppState.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val total = key.totalBudget?.toULong()
    val expiresAt = key.expiresAt?.toULong()

    // What is left of the budget, only next to the total it was read for.
    var remaining by remember { mutableStateOf<Pair<ULong, ULong?>?>(null) }
    var remainingError by remember { mutableStateOf<String?>(null) }
    var loadingRemaining by remember { mutableStateOf(false) }
    var gate by remember { mutableStateOf<UpdateLimitsGate?>(null) }
    var walletRefreshed by rememberSaveable { mutableStateOf(false) }

    LaunchedEffect(key.keyId, total, sdk) {
        val activeSdk = sdk
        remaining = null
        remainingError = null
        if (total == null || activeSdk == null) return@LaunchedEffect
        loadingRemaining = true
        try {
            val budgets = KeyLimitsFormatting.remainingBudgets(
                activeSdk.identities.fetchKeysRemainingBudgets(identityIdBase58, listOf(key.keyId)),
            )
            remaining = total to budgets[key.keyId]
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            remainingError = e.message ?: e.toString()
        } finally {
            loadingRemaining = false
        }
    }

    LaunchedEffect(key.keyId, total, expiresAt, key.disabledAt, manager, wallet) {
        if (!key.hasLimits) return@LaunchedEffect
        val mgr = manager
        val w = wallet
        if (mgr == null || w == null) {
            gate = UpdateLimitsGate.Refused("Wallet not loaded; this identity's keys cannot sign here.")
            return@LaunchedEffect
        }
        gate = null
        val calls = keyLimitsWallet(mgr, w, identityId, key.keyId).getOrElse {
            // The wallet was closed while this screen opened.
            gate = UpdateLimitsGate.Refused("Wallet not loaded; this identity's keys cannot sign here.")
            return@LaunchedEffect
        }
        if (!walletRefreshed) {
            walletRefreshed = true
            try {
                calls.refresh()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.w(TAG, "wallet identity refresh failed: ${e.message}")
            }
        }
        gate = try {
            calls.preview(null, null)
            UpdateLimitsGate.Allowed
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            UpdateLimitsGate.Refused(e.message ?: e.toString())
        }
    }

    FormSection(title = "Limits") {
        if (total != null) {
            LabeledContent("Last-synced budget", KeyLimitsFormatting.dash(total))
            val reading = remaining?.takeIf { it.first == total }
            when {
                loadingRemaining -> Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Remaining on Platform", modifier = Modifier.weight(1f))
                    CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp)
                }
                reading?.second != null -> LabeledContent(
                    "Remaining on Platform",
                    KeyLimitsFormatting.dash(reading.second!!),
                    valueColor = if (reading.second == 0uL) MaterialTheme.colorScheme.error else null,
                )
                remainingError != null -> LabeledContent("Remaining on Platform", "Unavailable")
                else -> LabeledContent("Remaining on Platform", "Unknown")
            }
            remainingError?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            }
        }
        if (expiresAt != null) {
            val now = System.currentTimeMillis()
            val expired = KeyLimitsFormatting.isExpired(expiresAt, now)
            LabeledContent("Expires", KeyLimitsFormatting.expiryDate(expiresAt))
            Text(
                KeyLimitsFormatting.expiryRelative(expiresAt, now),
                style = MaterialTheme.typography.bodySmall,
                color = if (expired) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        when (val current = gate) {
            null -> Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                OutlinedButton(onClick = {}, enabled = false, modifier = Modifier.weight(1f).testTag("keyLimits.update.open")) {
                    Text("Update Limits")
                }
                Spacer(Modifier.width(8.dp))
                CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp)
            }
            UpdateLimitsGate.Allowed -> OutlinedButton(
                onClick = { navController.navigate(UpdateKeyLimits(identityIdHex, key.keyId)) },
                modifier = Modifier.fillMaxWidth().testTag("keyLimits.update.open"),
            ) { Text("Update Limits") }
            is UpdateLimitsGate.Refused -> {
                OutlinedButton(
                    onClick = {},
                    enabled = false,
                    modifier = Modifier.fillMaxWidth().testTag("keyLimits.update.open"),
                ) { Text("Update Limits") }
                Text(
                    current.reason,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.testTag("keyLimits.update.unavailableReason"),
                )
            }
        }
        Text(
            "Platform refuses anything this key signs once its budget is spent or its expiry has " +
                "passed. The limits cannot be lowered. The budget can be raised and the expiry " +
                "extended by the identity's master key, or by a critical authentication key that " +
                "has no limits and no contract bounds.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

private const val TAG = "KeyLimitsSection"
