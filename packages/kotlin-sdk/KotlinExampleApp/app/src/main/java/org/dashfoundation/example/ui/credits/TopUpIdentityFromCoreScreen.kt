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
import org.dashfoundation.example.util.toHex
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
 *
 * A resume goes through
 * [org.dashfoundation.dashsdk.credits.IdentityCredits.resumeTopUpToRecordedRecipient]
 * (`platform_wallet_resume_identity_top_up_to_recorded_recipient_signer`):
 * the lock pays the identity it is for — a bound lock this identity, an
 * unbound lock the identity its row recorded, possibly someone else's — and
 * never the identity this screen shows by default. Unbound locks Rust reports
 * as unassigned (`TrackedAssetLock.identityRecipientUnassigned`) are listed
 * apart with an "assign identity" action
 * ([org.dashfoundation.dashsdk.credits.IdentityCredits.assignTopUpRecipient]);
 * once assigned they resume like any other (dashpay/platform#5924).
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
    var unassignedLocks by remember { mutableStateOf(emptyList<TrackedAssetLock>()) }
    var selectedUnassignedLock by remember { mutableStateOf<TrackedAssetLock?>(null) }
    var assignIdentityHex by rememberSaveable(identityIdHex) { mutableStateOf(identityIdHex) }
    var isAssigning by remember { mutableStateOf(false) }
    var paidIdentityHex by remember { mutableStateOf<String?>(null) }
    // Bumped after an assignment so the tracked-lock snapshot is re-read.
    var reloadKey by remember { mutableStateOf(0) }

    LaunchedEffect(manager, wallet, identity?.identityIndex, mode, reloadKey) {
        recoveryLoadError = null
        val rows = try {
            val mgr = manager
            val currentWallet = wallet
            if (mgr == null || currentWallet == null) emptyList() else {
                mgr.trackedIdentityRecoveryAssetLocks(currentWallet.walletId)
            }
        } catch (e: Exception) {
            recoveryLoadError = e.message ?: "Failed to load tracked identity top-up locks"
            emptyList()
        }
        val currentIdentityIndex = identity?.identityIndex
        recoveryLocks = if (currentIdentityIndex == null) emptyList() else {
            IdentityAssetLockRecovery.topUps(rows, currentIdentityIndex)
        }
        unassignedLocks = IdentityAssetLockRecovery.unassignedTopUps(rows)
        selectedRecoveryLock = recoveryLocks.firstOrNull()
        selectedUnassignedLock = unassignedLocks.firstOrNull()
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
                paidIdentityHex?.takeIf { it != identityIdHex }?.let {
                    LabeledContent("Paid identity", it.take(16) + "…")
                }
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
                            "Resumes this exact outpoint to the identity the lock pays. " +
                                "No replacement funding transaction is built.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (unassignedLocks.isNotEmpty()) {
                        val selectedLock = selectedUnassignedLock ?: unassignedLocks.first()
                        UnassignedTopUpLocks(
                            locks = unassignedLocks,
                            selected = selectedLock,
                            onSelected = { selectedUnassignedLock = it },
                            identityHex = assignIdentityHex,
                            onIdentityHexChange = { assignIdentityHex = it },
                            isAssigning = isAssigning,
                            onAssign = assign@{
                                val mgr = manager ?: return@assign
                                val w = wallet ?: return@assign
                                isAssigning = true
                                scope.launch {
                                    try {
                                        mgr.identityCredits.assignTopUpRecipient(
                                            walletHandle = w.handle,
                                            lock = selectedLock,
                                            identityId = assignIdentityHex.hexToBytes(),
                                        )
                                        reloadKey++
                                    } catch (e: Exception) {
                                        error = e.message ?: "Assigning the identity failed"
                                    } finally {
                                        isAssigning = false
                                    }
                                }
                            },
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
                            val paid = IdentityAssetLockRecovery.submitTopUpResume(lock) {
                                mgr.identityCredits.resumeTopUpToRecordedRecipient(
                                    walletHandle = w.handle,
                                    lock = it,
                                    coreSignerHandle = mgr.mnemonicResolverHandle,
                                )
                            }
                            paidIdentityHex = paid.identityId.toHex()
                            paid.newBalance
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

/**
 * Unbound top-up locks that record no identity: Rust refuses to resume them
 * until the user assigns the identity they pay, which is permanent. The id is
 * entered as hex and defaults to the screen's identity.
 */
@Composable
private fun UnassignedTopUpLocks(
    locks: List<TrackedAssetLock>,
    selected: TrackedAssetLock,
    onSelected: (TrackedAssetLock) -> Unit,
    identityHex: String,
    onIdentityHexChange: (String) -> Unit,
    isAssigning: Boolean,
    onAssign: () -> Unit,
) {
    Text(
        "Unassigned top-up locks",
        style = MaterialTheme.typography.titleSmall,
        modifier = Modifier.testTag("topUpIdentityFromCore.unassigned.header"),
    )
    Text(
        "These locks were rebuilt from the chain and do not record which identity they top up. " +
            "Assign one (permanently) before resuming it.",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
    AccessiblePicker(
        label = "Unassigned asset lock",
        options = locks,
        selected = selected,
        optionLabel = IdentityAssetLockRecovery::label,
        testTag = "topUpIdentityFromCore.unassigned.outpoint",
        onSelected = onSelected,
    )
    OutlinedTextField(
        value = identityHex,
        onValueChange = { onIdentityHexChange(it.trim().lowercase()) },
        label = { Text("Identity id (hex)") },
        singleLine = true,
        modifier = Modifier.fillMaxWidth().testTag("topUpIdentityFromCore.unassigned.identity"),
    )
    SubmitButton(
        text = "Assign identity",
        isLoading = isAssigning,
        enabled = !isAssigning && isIdentityIdHex(identityHex),
        modifier = Modifier.fillMaxWidth().testTag("topUpIdentityFromCore.unassigned.assign"),
        onClick = onAssign,
    )
}

/** A 32-byte identity id as 64 hex characters. */
internal fun isIdentityIdHex(value: String): Boolean =
    value.length == 64 && value.all { it in '0'..'9' || it in 'a'..'f' }

private enum class TopUpCoreMode(val label: String) {
    NEW_LOCK("Build new asset lock"),
    EXISTING_LOCK("Resume existing asset lock"),
}
