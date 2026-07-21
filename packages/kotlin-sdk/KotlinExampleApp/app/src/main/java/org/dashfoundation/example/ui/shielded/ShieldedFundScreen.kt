package org.dashfoundation.example.ui.shielded

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.ShieldedFundProgress
import org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator.StartFundingResult
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Shield funds from an asset lock — port of `ShieldedFundFromAssetLockView.swift`.
 * Gated on shielded support ([ShieldedGate]). Shows the Halo 2 prover
 * readiness (via [ShieldedProver.isReady], warming it on entry) and the
 * consensus-pinned shield fee ([ShieldedProver.estimateFee]).
 *
 * Submit is wired to the real shield FFI: the recipient defaults to the
 * wallet's own bound shielded address ("shield to self", via
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedDefaultAddress]),
 * overridable with an 86-char hex (= 43 raw bytes). The funding runs through
 * the [org.dashfoundation.example.services.shielded.ShieldedFundFromAssetLockCoordinator]
 * (per-(wallet, recipient) slot + per-wallet serialization), calling
 * `platform_wallet_manager_shielded_fund_from_asset_lock`; the screen then
 * navigates to the dismissal-safe progress view. The Orchard note arrives on
 * the next shielded sync pass, not synchronously.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ShieldedFundScreen(walletIdHex: String, navController: NavHostController) {
    ShieldedGate(navController) {
        val container = LocalAppContainer.current
        val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
        val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()

        var amountText by rememberSaveable { mutableStateOf("") }
        var recipientHex by rememberSaveable { mutableStateOf("") }
        var error by remember { mutableStateOf<String?>(null) }
        var isSubmitting by remember { mutableStateOf(false) }

        // Prover status + fee estimate (bridged, single-note-with-change = 2 actions).
        val proverReady by produceState(initialValue = false) {
            runCatching { ShieldedProver.warmUp() }
            value = runCatching { ShieldedProver.isReady() }.getOrDefault(false)
        }
        val feeEstimate by produceState<Long?>(initialValue = null) {
            value = runCatching {
                ShieldedProver.estimateFee(ShieldedProver.FeeKind.TransferOrShield, 2)
            }.getOrNull()
        }

        // Default "shield to self" recipient — the wallet's bound shielded
        // address (null until the wallet is bound), mirroring the Swift view.
        val defaultRecipient by produceState<ByteArray?>(initialValue = null, manager) {
            value = manager?.let { m ->
                runCatching { m.shieldedDefaultAddress(walletId) }.getOrNull()
            }
        }

        // The effective recipient: user hex override (86 hex chars = 43 bytes)
        // when valid, else the wallet's default.
        val trimmedHex = recipientHex.trim()
        val overrideRecipient = remember(trimmedHex) {
            if (trimmedHex.length == 86) runCatching { trimmedHex.hexToBytes() }
                .getOrNull()?.takeIf { it.size == 43 }
            else null
        }
        val recipient = overrideRecipient ?: defaultRecipient

        val amount = amountText.toLongOrNull()
        val canSubmit =
            manager != null && recipient != null && amount != null && amount > 0 && !isSubmitting

        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text("Shield from Asset Lock") },
                    navigationIcon = {
                        IconButton(onClick = { navController.popBackStack() }) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                        }
                    },
                )
            },
        ) { padding ->
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .imePadding()
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                FormSection(title = "Prover") {
                    LabeledContent(
                        "Halo 2 prover",
                        if (proverReady) "Ready" else "Preparing…",
                    )
                    LabeledContent(
                        "Estimated fee",
                        feeEstimate?.let { "$it credits" } ?: "—",
                    )
                }

                FormSection(title = "Recipient") {
                    OutlinedTextField(
                        value = recipientHex,
                        onValueChange = { recipientHex = it },
                        label = { Text("Orchard address (hex, blank = shield to self)") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth().testTag("shieldedFund.recipient"),
                    )
                    Text(
                        when {
                            overrideRecipient != null -> "Using pasted recipient."
                            defaultRecipient != null -> "Defaulting to this wallet's shielded address."
                            else -> "This wallet has no bound shielded address yet — paste a recipient."
                        },
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }

                FormSection(title = "Amount") {
                    OutlinedTextField(
                        value = amountText,
                        onValueChange = { amountText = it.filter(Char::isDigit) },
                        label = { Text("Amount (duffs)") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier.fillMaxWidth().testTag("shieldedFund.amount"),
                    )
                }

                SubmitButton(
                    text = "Shield",
                    isLoading = isSubmitting,
                    enabled = canSubmit,
                    modifier = Modifier.fillMaxWidth().testTag("shieldedFund.submit"),
                ) {
                    val m = manager ?: return@SubmitButton
                    val recipientBytes = recipient ?: return@SubmitButton
                    val amountDuffs = amount ?: return@SubmitButton
                    isSubmitting = true
                    // Start the funding through the coordinator (dismissal-safe,
                    // per-wallet serialized). The body performs the shield FFI.
                    val result = container.shieldedFundCoordinator.startFunding(
                        walletId = walletId,
                        recipientRaw43 = recipientBytes,
                    ) {
                        m.shieldedFundFromAssetLock(
                            walletId = walletId,
                            recipientRaw43 = recipientBytes,
                            amountDuffs = amountDuffs,
                        )
                    }
                    when (result) {
                        is StartFundingResult.Started -> {
                            navController.navigate(
                                ShieldedFundProgress(walletIdHex, recipientBytes.toHex()),
                            )
                        }
                        is StartFundingResult.BlockedByOtherWalletFunding -> {
                            error = "Another shielded funding is already in flight on this " +
                                "wallet (recipient ${result.blocker.recipientRaw43.toHex().take(16)}…). " +
                                "Shield-class operations are serialized per wallet — wait for it to finish."
                        }
                    }
                    isSubmitting = false
                }
            }
        }

        ErrorAlertDialog(message = error, onDismiss = { error = null })
    }
}
