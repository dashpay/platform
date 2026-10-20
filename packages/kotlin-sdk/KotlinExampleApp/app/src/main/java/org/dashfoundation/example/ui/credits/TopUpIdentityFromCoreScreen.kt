package org.dashfoundation.example.ui.credits

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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.dashfoundation.example.services.assetlock.IdentityAssetLockRecovery

/**
 * Top up an identity's credit balance by either building a **new Core asset
 * lock** (ID-05) or resuming an existing Rust-tracked type-1/type-2 lock
 * (ID-16) with its exact outpoint — distinct from
 * [TopUpIdentityScreen] (ID-06) which spends already-funded
 * Platform-payment addresses.
 *
 * On submit it draws the asset-lock UTXOs from the wallet's BIP44 standard
 * account 0 (matching `CreateIdentityScreen`'s hardcoded `accountIndex = 0`)
 * and credits the identity through
 * `platform_wallet_top_up_identity_with_funding_signer`
 * ([org.dashfoundation.dashsdk.credits.IdentityCredits.topUpFromCore]). The
 * `IdentityTopUp` transition is signed by the asset lock's Core key via the
 * manager's `MnemonicResolverHandle`, so no identity signer is involved.
 *
 * Amount is entered in **duffs** (Dash * 1e8), matching the create-identity
 * screen's `amountText`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TopUpIdentityFromCoreScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val wallet = rememberManagedWalletFor(identity?.walletId)

    var amountText by rememberSaveable { mutableStateOf("50000000") } // 0.5 DASH in duffs
    var error by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var newBalance by remember { mutableStateOf<Long?>(null) }
    var mode by rememberSaveable { mutableStateOf(TopUpCoreMode.NEW_LOCK) }
    var recoveryLocks by remember { mutableStateOf(emptyList<TrackedAssetLock>()) }
    var selectedRecoveryLock by remember { mutableStateOf<TrackedAssetLock?>(null) }
    var recoveryLoadError by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(manager, wallet, identity?.identityIndex, mode) {
        recoveryLoadError = null
        recoveryLocks = try {
            val mgr = manager
            val currentWallet = wallet
            val currentIdentityIndex = identity?.identityIndex
            if (mgr == null || currentWallet == null || currentIdentityIndex == null) emptyList() else {
                IdentityAssetLockRecovery.topUps(
                    mgr.trackedIdentityRecoveryAssetLocks(currentWallet.walletId),
                    currentIdentityIndex,
                )
            }
        } catch (e: Exception) {
            recoveryLoadError = e.message ?: "Failed to load tracked identity top-up locks"
            emptyList()
        }
        selectedRecoveryLock = recoveryLocks.firstOrNull()
    }

    val amount = amountText.toLongOrNull()
    val canSubmit = wallet != null && manager != null &&
        ((mode == TopUpCoreMode.NEW_LOCK && amount != null && amount > 0) ||
            (mode == TopUpCoreMode.EXISTING_LOCK && selectedRecoveryLock != null)) &&
        !isSubmitting && newBalance == null

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Top Up from Core") },
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
            FormSection(title = "Identity") {
                LabeledContent("Identity", identity?.mainDpnsName ?: identityIdHex.take(16) + "…")
                LabeledContent("Current balance", "${identity?.balance ?: 0} credits")
                newBalance?.let { LabeledContent("New balance", "$it credits") }
            }

            FormSection(title = "Funding") {
                AccessiblePicker(
                    label = "Funding operation",
                    options = TopUpCoreMode.entries,
                    selected = mode,
                    optionLabel = { it.label },
                    testTag = "topUpIdentityFromCore.mode",
                    onSelected = { mode = it },
                )
                if (mode == TopUpCoreMode.EXISTING_LOCK) {
                    if (recoveryLoadError != null) {
                        Text(
                            recoveryLoadError!!,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("topUpIdentityFromCore.resume.error"),
                        )
                    } else if (recoveryLocks.isEmpty()) {
                        Text(
                            "No Rust-tracked identity top-up locks are currently resumable.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.testTag("topUpIdentityFromCore.resume.empty"),
                        )
                    } else {
                        AccessiblePicker(
                            label = "Existing asset lock",
                            options = recoveryLocks,
                            selected = selectedRecoveryLock ?: recoveryLocks.first(),
                            optionLabel = IdentityAssetLockRecovery::label,
                            testTag = "topUpIdentityFromCore.resume.outpoint",
                            onSelected = { selectedRecoveryLock = it },
                        )
                        Text(
                            "Resumes this exact outpoint. No replacement funding transaction is built.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else {
                    Text(
                        "Funded by building and broadcasting a new Core asset lock " +
                            "from the wallet's Core balance (BIP44 account 0) — the " +
                            "same mechanism as identity creation.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    OutlinedTextField(
                        value = amountText,
                        onValueChange = { amountText = it.filter(Char::isDigit) },
                        label = { Text("Amount (duffs)") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier.fillMaxWidth().testTag("topUpIdentityFromCore.amount"),
                    )
                }
            }

            SubmitButton(
                text = if (newBalance != null) "Topped Up" else "Top Up from Core",
                isLoading = isSubmitting,
                enabled = canSubmit,
                modifier = Modifier.fillMaxWidth().testTag("topUpIdentityFromCore.submit"),
            ) {
                val mgr = manager ?: return@SubmitButton
                val w = wallet ?: return@SubmitButton
                isSubmitting = true
                scope.launch {
                    try {
                        // One Rust-authoritative FFI call: either build a new
                        // lock or resume the selected lock's exact outpoint.
                        val balance = if (mode == TopUpCoreMode.EXISTING_LOCK) {
                            val lock = selectedRecoveryLock
                                ?: error("No resumable identity top-up lock is selected")
                            IdentityAssetLockRecovery.submitTopUpResume(lock) {
                                mgr.identityCredits.resumeTopUpWithExistingAssetLock(
                                    walletHandle = w.handle,
                                    identityId = idBytes,
                                    lock = it,
                                    coreSignerHandle = mgr.mnemonicResolverHandle,
                                )
                            }
                        } else {
                            mgr.identityCredits.topUpFromCore(
                                walletHandle = w.handle,
                                identityId = idBytes,
                                amountDuffs = amount
                                    ?: error("A positive amount is required for a new asset lock"),
                                accountIndex = 0,
                                coreSignerHandle = mgr.mnemonicResolverHandle,
                            )
                        }
                        newBalance = balance
                    } catch (e: Exception) {
                        error = e.message ?: "Top up failed"
                    } finally {
                        isSubmitting = false
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

private enum class TopUpCoreMode(val label: String) {
    NEW_LOCK("Build new asset lock"),
    EXISTING_LOCK("Resume existing asset lock"),
}
