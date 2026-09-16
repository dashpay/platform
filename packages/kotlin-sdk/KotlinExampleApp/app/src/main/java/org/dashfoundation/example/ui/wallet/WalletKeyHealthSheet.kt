package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Cancel
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.util.Base58

/**
 * Wallet key-health diagnostic — port of `WalletKeyHealthSheet.swift`,
 * scoped to what the Kotlin SDK bridges today: for every identity owned by
 * this wallet (`IdentityDao`), walk its `PublicKeyDao` rows and check whether
 * the private key for each stored pubkey is actually RECOVERABLE — the probe
 * (`WalletStorage.probeIdentityKeyRecoverability`, the analogue of the iOS
 * Keychain lookup) opens the stored blob, so a present-but-stranded/
 * undecryptable key reads as unrecoverable, not just an absent one.
 *
 * Unrecoverable rows offer a Repair action: it re-derives the canonical private
 * key at `(identityIndex, keyId)` from the wallet mnemonic via the
 * resolver-keyed derive FFI (`dash_sdk_derive_identity_key_at_slot`,
 * surfaced as `PlatformWalletManager.repairIdentityKey`) and re-encrypts it
 * into `WalletStorage` — the Android realization of the iOS re-derive path
 * in `WalletKeyHealthSheet.swift` (`deriveIdentityAuthKeyAtSlot`). A
 * successful repair flips the row to Healthy on the next check pass.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WalletKeyHealthSheet(
    walletId: ByteArray,
    onDismiss: () -> Unit,
) {
    val container = LocalAppContainer.current
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    val identities by remember {
        container.database.identityDao().observeByWallet(walletId)
    }.collectAsStateWithLifecycle(initialValue = null)

    // Bumping this re-runs the key check after a repair writes new material.
    var refreshTick by remember { mutableStateOf(0) }

    // identityId(base58) → list of (keyId, purpose, pubkeyHexShort, healthy).
    val reports by produceState<List<IdentityKeyReport>?>(
        initialValue = null,
        identities,
        refreshTick,
    ) {
        val ids = identities ?: return@produceState
        // The recoverability probe opens each Keystore blob (a decrypt), so it
        // MUST run off the composition/Main thread — do the whole report build
        // on IO (the Room reads suspend, the probes are blocking Keystore work).
        value = withContext(Dispatchers.IO) {
            ids.map { identity ->
                val base58Id = Base58.encode(identity.identityId)
                val keys = container.database.publicKeyDao()
                    .observeByIdentityId(base58Id).first()
                    .map { row ->
                        val pubkeyHex = row.publicKeyData.joinToString("") { "%02x".format(it) }
                        KeyHealthRow(
                            keyId = row.keyId,
                            purpose = row.purpose,
                            securityLevel = row.securityLevel,
                            pubkeyHex = pubkeyHex,
                            publicKeyData = row.publicKeyData,
                            // Real recoverability, not mere presence — the
                            // PROBING check actually opens the blob with the
                            // candidate keys, so a stranded/sibling-alias blob is
                            // reported unrecoverable and gets the same repair as a
                            // truly-absent one (the cheap isPrivateKeyDecryptable
                            // is reserved for the signer's capability callback).
                            isRecoverable = runCatching {
                                container.walletStorage.probeIdentityKeyRecoverability(pubkeyHex)
                            }.getOrDefault(false),
                        )
                    }
                IdentityKeyReport(
                    identityIdBase58 = base58Id,
                    identityIndex = identity.identityIndex,
                    keys = keys,
                )
            }
        }
    }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp)
                .padding(bottom = 32.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text("Verify Identity Keys", style = MaterialTheme.typography.titleLarge)

            val current = reports
            when {
                current == null -> Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    CircularProgressIndicator()
                    Text("Checking keys…", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }

                current.isEmpty() -> Text(
                    "No identities to check.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(vertical = 16.dp),
                )

                else -> {
                    // Summary (← summarySection).
                    val healthy = current.count { report -> report.keys.all { it.isRecoverable } }
                    FormSection(title = "Summary") {
                        SummaryRow("Identities checked", current.size)
                        SummaryRow("Healthy", healthy)
                        SummaryRow("Unrecoverable key material", current.size - healthy)
                    }

                    current.forEach { report ->
                        FormSection(
                            title = "Identity ${report.identityIdBase58.take(12)}… " +
                                "(idx ${report.identityIndex})",
                        ) {
                            if (report.keys.isEmpty()) {
                                Text(
                                    "No public keys persisted for this identity.",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.padding(vertical = 6.dp),
                                )
                            }
                            report.keys.forEach { key ->
                                val activeManager = manager
                                KeyRow(
                                    key = key,
                                    // Repair is available only when a manager
                                    // is live (holds the resolver + storage).
                                    onRepair = if (!key.isRecoverable && activeManager != null) {
                                        {
                                            scope.launch {
                                                runCatching {
                                                    withContext(Dispatchers.IO) {
                                                        // The repair reads the derivation slot
                                                        // from the persisted breadcrumbs on the
                                                        // key's row — the example app must NOT
                                                        // pass an index (e.g. the DPP key id):
                                                        // a wrong slot derives a different valid
                                                        // scalar that round-trips fine and
                                                        // persists an unusable key
                                                        // (dashpay/platform#4060 blocker 1).
                                                        activeManager.repairIdentityKey(
                                                            walletId = walletId,
                                                            publicKeyData = key.publicKeyData,
                                                        )
                                                    }
                                                }
                                                // Re-check regardless of outcome
                                                // so the row reflects reality.
                                                refreshTick++
                                            }
                                        }
                                    } else {
                                        null
                                    },
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

private data class KeyHealthRow(
    val keyId: Int,
    val purpose: String,
    val securityLevel: String,
    val pubkeyHex: String,
    val publicKeyData: ByteArray,
    /**
     * Whether the private key actually opens on this device — the probe
     * decrypts the stored blob. `false` covers BOTH a truly-absent key and a
     * present-but-unrecoverable one (stranded ciphertext / sibling-alias blob
     * that no longer decrypts); the probe can't tell them apart, so the row is
     * labeled by recoverability, not presence, and both get the same re-derive
     * repair (dashpay/platform#4183 review).
     */
    val isRecoverable: Boolean,
)

private data class IdentityKeyReport(
    val identityIdBase58: String,
    val identityIndex: Int,
    val keys: List<KeyHealthRow>,
)

@Composable
private fun SummaryRow(label: String, count: Int) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(label, style = MaterialTheme.typography.bodyMedium)
        Text(
            count.toString(),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun KeyRow(key: KeyHealthRow, onRepair: (() -> Unit)? = null) {
    Column(modifier = Modifier.padding(vertical = 6.dp)) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Icon(
                imageVector = if (key.isRecoverable) Icons.Default.CheckCircle else Icons.Default.Cancel,
                contentDescription = null,
                tint = if (key.isRecoverable) Color(0xFF2E7D32) else MaterialTheme.colorScheme.error,
            )
            Text(
                "Key #${key.keyId} — purpose ${key.purpose}, level ${key.securityLevel}",
                style = MaterialTheme.typography.bodyMedium,
            )
        }
        Text(
            key.pubkeyHex.take(24) + "…",
            style = MaterialTheme.typography.labelSmall,
            fontFamily = FontFamily.Monospace,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            if (key.isRecoverable) {
                "Healthy — private key material recoverable on this device"
            } else {
                // Not necessarily absent: the blob may be present but stranded/
                // undecryptable (sibling-alias or invalidated Keystore key).
                // The probe can't distinguish, so don't claim "no entry".
                "Unrecoverable — key material missing or stranded; re-derive to repair"
            },
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        if (onRepair != null) {
            TextButton(
                onClick = onRepair,
                modifier = Modifier.testTag("walletKeyHealth.repair.${key.keyId}"),
            ) {
                Text("Re-derive from mnemonic")
            }
        }
    }
}
