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
 * this wallet (`IdentityDao`), walk its `PublicKeyDao` rows and verify the
 * Keystore holds private-key material for each stored pubkey
 * (`WalletStorage.hasPrivateKey`, the analogue of the iOS Keychain
 * lookup).
 *
 * Missing rows offer a Repair action: it re-derives the canonical private
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
        value = ids.map { identity ->
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
                        // reported unhealthy and gets the same repair as a
                        // missing one (the cheap isPrivateKeyDecryptable is
                        // reserved for the signer's capability callback).
                        hasPrivateKey = runCatching {
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
                    val healthy = current.count { report -> report.keys.all { it.hasPrivateKey } }
                    FormSection(title = "Summary") {
                        SummaryRow("Identities checked", current.size)
                        SummaryRow("Healthy", healthy)
                        SummaryRow("Missing key material", current.size - healthy)
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
                                    onRepair = if (!key.hasPrivateKey && activeManager != null) {
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
    val hasPrivateKey: Boolean,
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
                imageVector = if (key.hasPrivateKey) Icons.Default.CheckCircle else Icons.Default.Cancel,
                contentDescription = null,
                tint = if (key.hasPrivateKey) Color(0xFF2E7D32) else MaterialTheme.colorScheme.error,
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
            if (key.hasPrivateKey) {
                "Healthy — private key material stored on this device"
            } else {
                "Missing — no Keystore entry for this public key"
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
