package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog

/**
 * Orphan-mnemonic recovery — port of the recovery pipeline in
 * `ContentView.swift` (lines 140–577) + `RecoverWalletsSheet.swift`.
 *
 * On the main screen's first composition, Keystore mnemonic entries
 * (`WalletStorage.listWalletIdsWithMnemonic`) are compared against the
 * persisted `WalletEntity` rows; ids with a stored phrase but no wallet
 * data are orphans. The flow then mirrors iOS:
 *
 *   "Recover Wallets?" alert → Authorize → [RecoverWalletsSheet]
 *   (checkbox per orphan) → one biometric prompt → per-wallet
 *   `PlatformWalletManager.createWallet(mnemonic)` re-derivation (the same
 *   call Swift's `recoverWallet` makes) → failures aggregate into one
 *   "Recovery Failed" alert.  "No" (or sheet Cancel) → "Keep these
 *   Wallets?" with Recreate / Delete.
 *
 * Android has no per-wallet keychain metadata blob, so recovery always
 * routes through the ACTIVE network's manager (iOS restores the original
 * network from `WalletKeychainMetadata`). Because wallet ids are
 * network-scoped, a recovered wallet may get a NEW id; the phrase is then
 * stored under the new id by `createWallet`, so the stale orphan entry is
 * dropped to keep the check convergent.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalStdlibApi::class)
@Composable
fun OrphanRecoveryHost() {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()

    var orphans by remember { mutableStateOf<List<String>>(emptyList()) }
    var showRecoverAlert by remember { mutableStateOf(false) }
    var showSheet by remember { mutableStateOf(false) }
    var showKeepPrompt by remember { mutableStateOf(false) }
    var recoveryError by remember { mutableStateOf<String?>(null) }
    var recovering by remember { mutableStateOf(false) }

    // One check per composition of the main screen (← checkForOrphanMnemonic,
    // which runs once per launch after the first tab appears).
    LaunchedEffect(Unit) {
        val stored = runCatching { container.walletStorage.listWalletIdsWithMnemonic() }
            .getOrDefault(emptyList())
        if (stored.isEmpty()) return@LaunchedEffect
        val localIds = container.database.walletDao().observeAll().first()
            .map { it.walletId.toHexString() }
            .toSet()
        orphans = stored.filterNot { localIds.contains(it.lowercase()) }
        if (orphans.isNotEmpty()) showRecoverAlert = true
    }

    suspend fun recover(selected: List<String>) {
        if (recovering || selected.isEmpty()) return
        recovering = true
        try {
            // Single shared auth round before any mnemonic is read
            // (← the shared LAContext prompt in authorizeAndRecover).
            when (
                container.biometricGate.authenticate(
                    title = "Recover Wallets",
                    subtitle = if (selected.size == 1) {
                        "Re-derive your wallet from the stored recovery phrase."
                    } else {
                        "Re-derive ${selected.size} wallets from the stored recovery phrases."
                    },
                )
            ) {
                BiometricGate.AuthOutcome.AUTHORIZED -> Unit
                BiometricGate.AuthOutcome.DENIED -> {
                    showKeepPrompt = true
                    return
                }

                BiometricGate.AuthOutcome.UNAVAILABLE -> {
                    recoveryError = "Authentication is unavailable on this device."
                    showKeepPrompt = true
                    return
                }

                BiometricGate.AuthOutcome.FAILED -> {
                    recoveryError = "Authorization failed."
                    showKeepPrompt = true
                    return
                }
            }

            val manager = container.walletManagerStore.activeManager.value
            if (manager == null) {
                recoveryError = "Wallet manager is not active."
                return
            }

            val failures = mutableListOf<String>()
            val recovered = mutableSetOf<String>()
            for (idHex in selected) {
                try {
                    val id = idHex.hexToByteArray()
                    val mnemonic = container.walletStorage.retrieveMnemonic(id)
                        ?: error("failed to read the stored mnemonic")
                    // Re-derive via the manager — the exact call Swift's
                    // recoverWallet makes. A recovered wallet is an EXISTING
                    // one that may have received Core funds/payments before
                    // this device recovered it, so scan from genesis
                    // (birthHeight 0u) rather than the SPV tip — matching
                    // Swift recoverWallet's `birthHeight: restoredBirthHeight
                    // ?? 0` (Android stores no per-orphan birth height, so the
                    // `?? 0` fallback always applies here).
                    val managed = manager.createWallet(
                        mnemonic,
                        name = "Recovered Wallet",
                        birthHeight = 0u,
                    )
                    if (!managed.walletIdHex.equals(idHex, ignoreCase = true)) {
                        // Recovered onto this network under a NEW scoped id
                        // (the orphan was created on another network). The
                        // phrase was just re-stored under the new id, so the
                        // stale entry can be dropped safely.
                        container.walletStorage.deleteMnemonic(id)
                    }
                    recovered.add(idHex)
                } catch (e: Exception) {
                    failures.add("${idHex.take(8)}…: ${e.message ?: "recovery failed"}")
                }
            }

            orphans = orphans.filterNot { recovered.contains(it) }
            if (failures.isNotEmpty()) {
                recoveryError = if (failures.size == 1) {
                    "Recovery failed: ${failures.first()}"
                } else {
                    "Recovery failed for ${failures.size} wallets:\n" +
                        failures.joinToString("\n")
                }
            }
        } finally {
            recovering = false
        }
    }

    if (showRecoverAlert) {
        val count = orphans.size
        AlertDialog(
            onDismissRequest = { showRecoverAlert = false },
            title = { Text(if (count <= 1) "Recover Wallet?" else "Recover Wallets?") },
            text = {
                Text(
                    if (count <= 1) {
                        "A wallet mnemonic is stored on this device, but no wallet " +
                            "data was found. Authorize to re-derive the wallet's " +
                            "public keys from the stored mnemonic."
                    } else {
                        "$count wallet mnemonics are stored on this device, but no " +
                            "matching wallet data was found. Authorize to re-derive " +
                            "their public keys from the stored mnemonics."
                    },
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showRecoverAlert = false
                        showSheet = true
                    },
                    modifier = Modifier.testTag("recoverWallets.alertAuthorize"),
                ) { Text("Authorize") }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        showRecoverAlert = false
                        showKeepPrompt = true
                    },
                ) { Text("No") }
            },
        )
    }

    if (showSheet) {
        RecoverWalletsSheet(
            orphanIdsHex = orphans,
            onAuthorize = { selected ->
                showSheet = false
                scope.launch { recover(selected) }
            },
            onCancel = {
                showSheet = false
                showKeepPrompt = true
            },
        )
    }

    if (showKeepPrompt) {
        val count = orphans.size
        AlertDialog(
            onDismissRequest = { showKeepPrompt = false },
            title = { Text(if (count <= 1) "Keep this Wallet?" else "Keep these Wallets?") },
            text = {
                Text(
                    if (count <= 1) {
                        "Recreate will re-derive the wallet from the stored " +
                            "mnemonic. Delete will permanently remove the mnemonic " +
                            "from this device."
                    } else {
                        "Recreate will re-derive every wallet from its stored " +
                            "mnemonic. Delete will permanently remove $count " +
                            "mnemonics from this device."
                    },
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showKeepPrompt = false
                        showRecoverAlert = true
                    },
                ) { Text("Recreate") }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        showKeepPrompt = false
                        scope.launch {
                            val failures = mutableListOf<String>()
                            orphans.forEach { idHex ->
                                runCatching {
                                    container.walletStorage
                                        .deleteMnemonic(idHex.hexToByteArray())
                                }.onFailure {
                                    failures.add("${idHex.take(8)}…: ${it.message}")
                                }
                            }
                            orphans = emptyList()
                            if (failures.isNotEmpty()) {
                                recoveryError =
                                    "Failed to delete mnemonics: ${failures.joinToString("; ")}"
                            }
                        }
                    },
                    modifier = Modifier.testTag("recoverWallets.deleteAll"),
                ) {
                    Text(
                        if (count <= 1) "Delete" else "Delete All",
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            },
        )
    }

    ErrorAlertDialog(
        message = recoveryError,
        title = "Recovery Failed",
        onDismiss = { recoveryError = null },
    )
}

/**
 * The orphan list sheet (← `RecoverWalletsSheet.swift`), simplified to a
 * checkbox per orphan (Android has one shared auth prompt, so the iOS
 * "Same PIN" grouping toggle reduces to include/exclude).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun RecoverWalletsSheet(
    orphanIdsHex: List<String>,
    onAuthorize: (List<String>) -> Unit,
    onCancel: () -> Unit,
) {
    var selected by remember { mutableStateOf(orphanIdsHex.toSet()) }

    ModalBottomSheet(onDismissRequest = onCancel) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 24.dp)
                .padding(bottom = 32.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Recover Wallets", style = MaterialTheme.typography.titleLarge)
            Text(
                if (orphanIdsHex.size == 1) {
                    "A wallet recovery phrase is stored on this device, but no " +
                        "matching wallet data was found. Authorize to re-derive " +
                        "its public keys from the stored phrase."
                } else {
                    "${orphanIdsHex.size} wallet recovery phrases are stored on " +
                        "this device, but no matching wallet data was found. " +
                        "Authorize to re-derive their public keys from the stored phrases."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            orphanIdsHex.forEach { idHex ->
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Checkbox(
                        checked = selected.contains(idHex),
                        onCheckedChange = { checked ->
                            selected = if (checked) selected + idHex else selected - idHex
                        },
                        modifier = Modifier.testTag("recoverWallets.checkbox.${idHex.take(8)}"),
                    )
                    Column {
                        Text("Recovered Wallet", style = MaterialTheme.typography.bodyMedium)
                        Text(
                            "${idHex.take(8)}…",
                            style = MaterialTheme.typography.labelSmall,
                            fontFamily = FontFamily.Monospace,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.End),
            ) {
                TextButton(
                    onClick = onCancel,
                    modifier = Modifier.testTag("recoverWallets.cancelButton"),
                ) { Text("Cancel") }
                Button(
                    onClick = { onAuthorize(orphanIdsHex.filter { selected.contains(it) }) },
                    enabled = selected.isNotEmpty(),
                    modifier = Modifier.testTag("recoverWallets.authorizeButton"),
                ) { Text("Authorize") }
            }
        }
    }
}
