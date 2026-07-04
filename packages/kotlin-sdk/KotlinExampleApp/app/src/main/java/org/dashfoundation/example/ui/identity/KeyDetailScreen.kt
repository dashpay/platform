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
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
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
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.toHex

/**
 * One public key's detail — port of `KeyDetailView.swift`. Shows the key
 * metadata + public key bytes, and (for a key whose private material is
 * stored) a biometric-gated reveal that reads the scalar from
 * [org.dashfoundation.dashsdk.security.WalletStorage]. The disable-key action
 * is a named deferral (needs the `updateIdentity` FFI); the presence
 * indicator + reveal are wired.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KeyDetailScreen(identityIdHex: String, keyId: Int, navController: NavHostController) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()

    var key by remember { mutableStateOf<PublicKeyEntity?>(null) }
    var hasPrivate by remember { mutableStateOf(false) }
    var revealed by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(identityIdHex, keyId) {
        val row = container.database.publicKeyDao().getByIdentityAndKeyId(identityIdHex, keyId)
        key = row
        hasPrivate = row != null && container.walletStorage.hasPrivateKey(row.publicKeyData.toHex())
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Key #$keyId") },
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
                LabeledContent("Purpose", k.purpose)
                LabeledContent("Type", k.keyType)
                LabeledContent("Security Level", k.securityLevel)
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

            if (k.disabledAt == null) {
                Text(
                    "Disable Key arrives with the key-management milestone.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}
