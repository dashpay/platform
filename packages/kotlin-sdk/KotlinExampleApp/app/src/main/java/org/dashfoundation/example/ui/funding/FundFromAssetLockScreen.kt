package org.dashfoundation.example.ui.funding

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
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.AddressFundProgress
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.credits.rememberManagedWalletFor
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * Fund a Platform address from an asset lock — port of
 * `FundFromAssetLockPlatformAddressView.swift`. On submit it picks a fresh
 * unused (zero-balance) Platform-payment address as the fee-absorbing
 * remainder recipient, then drives the funding through the dismissal-safe
 * [org.dashfoundation.example.services.assetlock.AddressFundFromAssetLockCoordinator],
 * whose body invokes the now-bridged
 * `platform_address_wallet_fund_from_asset_lock_signer`
 * ([ManagedPlatformWallet.fundFromAssetLock]) with the manager's
 * platform-address signer + core mnemonic resolver. After start it navigates
 * to the live progress screen. The coordinator's resume path drives
 * [ManagedPlatformWallet.resumeFundFromAssetLock] once a resume UI entry
 * surfaces a tracked lock.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FundFromAssetLockScreen(walletIdHex: String, navController: NavHostController) {
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(walletId)

    // Fresh unused, zero-balance Platform-payment addresses are the fund
    // recipient candidates (matching Swift's `recipientCandidates`). Sorted
    // by (account, index); the first is auto-selected.
    val platformAddresses by container.database.platformAddressDao()
        .observeByWallet(walletId)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val recipient = remember(platformAddresses) {
        platformAddresses
            .filter { !it.isUsed && it.balance == 0L }
            .minWithOrNull(compareBy({ it.accountIndex }, { it.addressIndex }))
    }

    var amountText by rememberSaveable { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }

    val amount = amountText.toLongOrNull()
    val canSubmit = wallet != null && manager != null && recipient != null &&
        amount != null && amount > 0

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Fund from Asset Lock") },
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
            FormSection(title = "Wallet") {
                Text(
                    walletIdHex.take(16) + "…",
                    style = MaterialTheme.typography.bodyMedium,
                )
            }

            FormSection(title = "Recipient") {
                if (recipient == null) {
                    Text(
                        "No unused Platform-payment address available. Sync " +
                            "platform addresses first.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                } else {
                    Text(
                        "Account ${recipient.accountIndex}, address " +
                            "#${recipient.addressIndex}",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                    Text(
                        recipient.addressHash.toHex().take(24) + "…",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            FormSection(title = "Amount") {
                Text(
                    "Builds an asset-lock transaction from the wallet's Core " +
                        "balance and credits a Platform address once the lock proves.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(
                    value = amountText,
                    onValueChange = { amountText = it.filter(Char::isDigit) },
                    label = { Text("Amount (duffs)") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    modifier = Modifier.fillMaxWidth().testTag("fundFromAssetLock.amount"),
                )
            }

            SubmitButton(
                text = "Fund Platform Address",
                isLoading = false,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("fundFromAssetLock.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                val rcpt = recipient ?: return@SubmitButton
                val amt = amount ?: return@SubmitButton
                val recipientHash = rcpt.addressHash
                val platformAccountIndex = rcpt.accountIndex
                val recipientType = rcpt.addressType

                container.addressFundCoordinator.startFunding(
                    walletId = walletId,
                    platformAccountIndex = platformAccountIndex,
                    recipientHash = recipientHash,
                    recipientType = recipientType,
                ) {
                    // Single remainder recipient (credits = null) absorbs the
                    // whole asset-lock value, matching Swift's submit body.
                    val updates = w.fundFromAssetLock(
                        amountDuffs = amt,
                        fundingAccountIndex = 0,
                        platformAccountIndex = platformAccountIndex,
                        recipients = listOf(
                            ManagedPlatformWallet.FundRecipient(
                                addressType = recipientType,
                                hash = recipientHash,
                                credits = null,
                            ),
                        ),
                        signerHandle = mgr.signerHandle,
                        coreSignerHandle = mgr.mnemonicResolverHandle,
                    )
                    updates.firstOrNull { it.hash.contentEquals(recipientHash) }?.balance ?: 0L
                }

                navController.navigate(
                    AddressFundProgress(
                        walletIdHex = walletIdHex,
                        platformAccountIndex = platformAccountIndex,
                        recipientHashHex = recipientHash.toHex(),
                    ),
                )
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}
