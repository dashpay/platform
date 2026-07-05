package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.services.IdentityKeys
import org.dashfoundation.example.services.KeyDisableGate
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.credits.rememberManagedWalletFor
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

/**
 * One public key's detail — port of `KeyDetailView.swift`. Shows the key
 * metadata + public key bytes, a biometric-gated private-key reveal (read
 * from [org.dashfoundation.dashsdk.security.WalletStorage]), and the Key
 * Status section with the gated destructive Disable Key action: the
 * [KeyDisableGate] pre-flight (master / last-auth / last-transfer refusals)
 * in front of `PlatformWalletManager.identityUpdates.disableKeys` — the
 * same `platform_wallet_update_identity_with_signer` call Swift's
 * `KeyDetailView.disableKey()` makes.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KeyDetailScreen(identityIdHex: String, keyId: Int, navController: NavHostController) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()

    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val idBase58 = remember(identityIdHex) { Base58.encode(idBytes) }

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    // The persister keys `public_keys.identityId` by base58; older callers
    // passed the hex nav-arg through — observe base58 and fall back to hex
    // so both encodings resolve.
    val keysBase58 by container.database.publicKeyDao()
        .observeByIdentityId(idBase58)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val keysHex by container.database.publicKeyDao()
        .observeByIdentityId(identityIdHex)
        .collectAsStateWithLifecycle(initialValue = emptyList())
    val allKeys = if (keysBase58.isNotEmpty()) keysBase58 else keysHex
    val key = allKeys.firstOrNull { it.keyId == keyId }

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(identity?.walletId)

    var hasPrivate by remember { mutableStateOf(false) }
    var revealed by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var showDisableConfirm by remember { mutableStateOf(false) }
    var isDisabling by remember { mutableStateOf(false) }

    LaunchedEffect(key?.publicKeyData) {
        val row = key
        hasPrivate = row != null &&
            container.walletStorage.hasPrivateKey(row.publicKeyData.toHex())
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Key #$keyId") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }, enabled = !isDisabling) {
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
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            val k = key
            if (k == null) {
                Text("Key not found.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                return@Column
            }

            FormSection(title = "Key Information") {
                LabeledContent("Key ID", "${k.keyId}")
                LabeledContent("Purpose", IdentityKeys.purposeName(k.purpose))
                LabeledContent("Type", IdentityKeys.keyTypeName(k.keyType))
                LabeledContent("Security Level", IdentityKeys.securityLevelName(k.securityLevel))
                if (k.disabledAt != null) LabeledContent("Status", "Disabled")
            }

            FormSection(title = "Public Key") {
                Text(
                    k.publicKeyData.toHex(),
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.testTag("keyDetail.publicKey"),
                )
            }

            FormSection(title = "Private Key") {
                if (!hasPrivate) {
                    Text(
                        "No private key stored on this device for this key.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else if (revealed == null) {
                    Text(
                        "Private key is stored securely.",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                    Button(
                        onClick = {
                            scope.launch {
                                when (container.biometricGate.authenticate(
                                    title = "Reveal private key",
                                    subtitle = "Key #${k.keyId}",
                                )) {
                                    BiometricGate.AuthOutcome.AUTHORIZED -> {
                                        try {
                                            val bytes = container.walletStorage
                                                .retrievePrivateKey(k.publicKeyData.toHex())
                                            revealed = bytes?.toHex()
                                                ?: run { error = "Private key could not be read."; null }
                                            bytes?.fill(0)
                                        } catch (e: Exception) {
                                            error = e.message ?: "Failed to read private key"
                                        }
                                    }
                                    BiometricGate.AuthOutcome.DENIED,
                                    BiometricGate.AuthOutcome.FAILED,
                                    -> error = "Authentication was not completed."
                                    BiometricGate.AuthOutcome.UNAVAILABLE ->
                                        error = "Biometric authentication is unavailable on this device."
                                }
                            }
                        },
                        modifier = Modifier.fillMaxWidth().testTag("keyDetail.reveal"),
                    ) { Text("Reveal Private Key") }
                } else {
                    Text(
                        revealed!!,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        modifier = Modifier.testTag("keyDetail.privateKey"),
                    )
                }
            }

            FormSection(title = "Key Status") {
                when (val evaluation = KeyDisableGate.evaluate(
                    target = k.normalizedForGate(),
                    allKeys = allKeys.map { it.normalizedForGate() },
                )) {
                    KeyDisableGate.Evaluation.AlreadyDisabled -> {
                        LabeledContent("Disabled", "Permanent")
                    }
                    KeyDisableGate.Evaluation.Allowed -> {
                        Button(
                            onClick = { showDisableConfirm = true },
                            enabled = !isDisabling && wallet != null && manager != null,
                            colors = ButtonDefaults.buttonColors(
                                containerColor = MaterialTheme.colorScheme.error,
                                contentColor = MaterialTheme.colorScheme.onError,
                            ),
                            modifier = Modifier.fillMaxWidth().testTag("keyDetail.disable"),
                        ) { Text(if (isDisabling) "Disabling…" else "Disable Key") }
                        Text(
                            "Disabling a key is permanent and irreversible on-chain.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        if (wallet == null) {
                            Text(
                                "Identity not attached to a loaded wallet — cannot sign " +
                                    "the disable transition.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    is KeyDisableGate.Evaluation.Forbidden -> {
                        Button(
                            onClick = {},
                            enabled = false,
                            modifier = Modifier.fillMaxWidth().testTag("keyDetail.disable"),
                        ) { Text("Disable Key") }
                        Text(
                            evaluation.reason,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }

    if (showDisableConfirm) {
        AlertDialog(
            onDismissRequest = { if (!isDisabling) showDisableConfirm = false },
            title = { Text("Disable Key #$keyId?") },
            text = {
                Text(
                    "This permanently and irreversibly disables key #$keyId on-chain. " +
                        "It can never be re-enabled — you would have to add a new key " +
                        "instead. Continue?",
                )
            },
            confirmButton = {
                TextButton(
                    enabled = !isDisabling,
                    onClick = {
                        // Re-check the gate at submit time — the key set could
                        // have changed since the view rendered. Never broadcast
                        // a doomed transition (← Swift disableKey()).
                        val target = key ?: return@TextButton
                        val gate = KeyDisableGate.evaluate(
                            target.normalizedForGate(),
                            allKeys.map { it.normalizedForGate() },
                        )
                        if (gate != KeyDisableGate.Evaluation.Allowed) {
                            showDisableConfirm = false
                            error = (gate as? KeyDisableGate.Evaluation.Forbidden)?.reason
                            return@TextButton
                        }
                        val w = wallet ?: return@TextButton
                        val mgr = manager ?: return@TextButton
                        isDisabling = true
                        scope.launch {
                            try {
                                mgr.identityUpdates.disableKeys(
                                    walletHandle = w.handle,
                                    identityId = idBytes,
                                    keyIds = listOf(keyId),
                                    signerHandle = mgr.signerHandle,
                                )
                                showDisableConfirm = false
                                navController.popBackStack()
                            } catch (e: Exception) {
                                showDisableConfirm = false
                                error = e.message ?: "Disable failed"
                            } finally {
                                isDisabling = false
                            }
                        }
                    },
                ) { Text(if (isDisabling) "Disabling…" else "Disable Key") }
            },
            dismissButton = {
                TextButton(
                    enabled = !isDisabling,
                    onClick = { showDisableConfirm = false },
                ) { Text("Cancel") }
            },
        )
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

/**
 * [KeyDisableGate] compares DPP enum *names*; Room rows may store the
 * stringified rawValue. Normalize before evaluating.
 */
private fun PublicKeyEntity.normalizedForGate(): PublicKeyEntity = copy(
    purpose = IdentityKeys.purposeName(purpose),
    securityLevel = IdentityKeys.securityLevelName(securityLevel),
)
