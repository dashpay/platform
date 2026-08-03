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
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
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
 * to the live progress screen.
 *
 * RESUME mode (ADDR-03, ← the Swift view's `resumeFromLock` parameter): when
 * [resumeOutPointHex] is non-null the screen hides the Amount section (the
 * lock + amount were fixed at original build time), still picks a fresh
 * recipient, and routes Submit to
 * [ManagedPlatformWallet.resumeFundFromAssetLock] instead of
 * `fundFromAssetLock`, seeded with the parsed outpoint. The
 * [PendingPlatformFundFromAssetLocksList] orphan surface opens the screen in
 * this mode.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FundFromAssetLockScreen(
    walletIdHex: String,
    navController: NavHostController,
    resumeOutPointHex: String? = null,
) {
    val walletId = remember(walletIdHex) { walletIdHex.hexToBytes() }
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(walletId)
    val isResume = resumeOutPointHex != null

    // In resume mode, load the tracked lock so we can show its amount +
    // status. The row is keyed by `outPointHex` (primary key); a null result
    // means the lock was swept between opening the list and this screen.
    val resumeLock by produceState<AssetLockEntity?>(initialValue = null, resumeOutPointHex) {
        value = resumeOutPointHex?.let {
            container.database.assetLockDao().getByOutPointHex(it)
        }
    }

    // Fresh unused, zero-balance Platform-payment addresses are the fund
    // recipient candidates (matching Swift's `recipientCandidates`). Sorted
    // by (account, index); the first is auto-selected. Resume mode reuses the
    // exact same recipient logic — the orphan lock doesn't carry a recipient,
    // it's picked at ST-submit time (← Swift's resume-mode comment).
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
    // Resume only needs a recipient (+ the loaded lock); fresh needs an
    // amount too. ← Swift `canSubmit`.
    val canSubmit = wallet != null && manager != null && recipient != null &&
        if (isResume) resumeLock != null else (amount != null && amount > 0)

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(if (isResume) "Resume Platform Top Up" else "Fund from Asset Lock") },
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

            if (isResume) {
                // Read-only summary of the lock being resumed — replaces the
                // Amount section (the locked amount is fixed by the original
                // build). ← Swift `resumeFromAssetLockSection`.
                FormSection(title = "Resuming") {
                    val lock = resumeLock
                    if (lock == null) {
                        Text(
                            "This asset lock is no longer tracked. Return to the " +
                                "Pending Platform Top Ups list.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("fundFromAssetLock.resume.missing"),
                        )
                    } else {
                        Text(
                            "Asset Lock ${lock.shortOutPointDisplay}",
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.testTag("fundFromAssetLock.resume.outpoint"),
                        )
                        Text(
                            "${lock.amountDuffs} duffs · ${lock.statusLabel}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Text(
                            if (lock.canFundIdentity) {
                                "The asset lock already reached a usable proof state. " +
                                    "Pick a destination address to complete the funding."
                            } else {
                                "The asset lock is broadcast and still awaiting InstantSend / " +
                                    "ChainLock finality. Resuming will wait for finality, then " +
                                    "credit the address."
                            },
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
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

            if (!isResume) {
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
            }

            SubmitButton(
                text = if (isResume) "Resume Top Up" else "Fund Platform Address",
                isLoading = false,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("fundFromAssetLock.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                val rcpt = recipient ?: return@SubmitButton
                val recipientHash = rcpt.addressHash
                val platformAccountIndex = rcpt.accountIndex
                val recipientType = rcpt.addressType

                if (isResume) {
                    val lock = resumeLock ?: return@SubmitButton
                    val parsed = parseOutPoint(lock.outPointHex)
                    if (parsed == null) {
                        error = "Could not parse asset lock outpoint: ${lock.outPointHex}"
                        return@SubmitButton
                    }
                    val (txid, vout) = parsed

                    container.addressFundCoordinator.startFunding(
                        walletId = walletId,
                        platformAccountIndex = platformAccountIndex,
                        recipientHash = recipientHash,
                        recipientType = recipientType,
                    ) {
                        val updates = w.resumeFundFromAssetLock(
                            outPointTxid = txid,
                            outPointVout = vout,
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
                } else {
                    val amt = amount ?: return@SubmitButton
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

/**
 * Parse a persisted `<txid display hex>:<vout>` outpoint back into the
 * 32-byte WIRE-order (little-endian) txid + vout that
 * [ManagedPlatformWallet.resumeFundFromAssetLock] requires. Inverse of the
 * SDK's `encodeOutPointHex` — the display hex is byte-reversed relative to
 * wire order, so we reverse the decoded 32 bytes. ← Swift `parseOutPoint`
 * (`Data(bytes.reversed())`) and the SDK-internal `decodeOutPointHex`.
 * Returns null on any malformed input.
 */
internal fun parseOutPoint(hex: String): Pair<ByteArray, Int>? {
    val sep = hex.indexOf(':')
    if (sep < 0) return null
    val txidDisplay = hex.substring(0, sep)
    val vout = hex.substring(sep + 1).toUIntOrNull()?.toInt() ?: return null
    if (txidDisplay.length != 64) return null
    val displayBytes = ByteArray(32)
    for (i in 0 until 32) {
        val hi = Character.digit(txidDisplay[i * 2], 16)
        val lo = Character.digit(txidDisplay[i * 2 + 1], 16)
        if (hi < 0 || lo < 0) return null
        displayBytes[i] = ((hi shl 4) or lo).toByte()
    }
    // Reverse display order → wire (little-endian) order for the FFI.
    val txid = ByteArray(32)
    for (i in 0 until 32) txid[i] = displayBytes[31 - i]
    return txid to vout
}
