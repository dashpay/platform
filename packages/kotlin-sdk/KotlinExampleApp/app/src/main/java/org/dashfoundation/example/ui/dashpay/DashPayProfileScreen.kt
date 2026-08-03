package org.dashfoundation.example.ui.dashpay

import android.graphics.Bitmap
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.generateQrBitmap
import org.dashfoundation.example.util.hexToBytes

/**
 * Own DashPay profile — port of `DashPayProfileView.swift` plus its companion
 * editor: read-only display (avatar / name / DPNS / public message / id), the
 * DIP-15 auto-accept QR (via `buildAutoAcceptQr`, rendered with the ZXing
 * `generateQrBitmap` helper), and an inline edit mode calling
 * `createOrUpdateProfile` (doCreate when no profile exists yet).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashPayProfileScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val identity by remember(idBytes) {
        container.database.identityDao().observeByIdentityId(idBytes)
    }.collectAsStateWithLifecycle(initialValue = null)
    val walletId = identity?.walletId
    val wallet = remember(manager, walletId) { walletId?.let { manager?.wallet(forWalletId = it) } }

    var profile by remember { mutableStateOf<DashPayProfile?>(null) }
    var profileExists by remember { mutableStateOf(false) }
    var qrUri by remember { mutableStateOf<String?>(null) }
    var qrError by remember { mutableStateOf<String?>(null) }

    // Encode the QR off the main thread (ZXing is CPU work).
    val qrBitmap by produceState<Bitmap?>(initialValue = null, qrUri) {
        val uri = qrUri
        value = if (uri != null) withContext(Dispatchers.Default) { generateQrBitmap(uri) } else null
    }

    var isEditing by remember { mutableStateOf(false) }
    var displayNameField by remember { mutableStateOf("") }
    var publicMessageField by remember { mutableStateOf("") }
    var avatarUrlField by remember { mutableStateOf("") }
    var isSaving by remember { mutableStateOf(false) }
    var saveError by remember { mutableStateOf<String?>(null) }

    suspend fun loadProfile() {
        val w = wallet ?: return
        val raw = w.dashpay.getProfile(idBytes)
        profileExists = raw != null
        profile = parseDashPayProfile(raw)
    }

    LaunchedEffect(wallet) {
        val w = wallet ?: return@LaunchedEffect
        loadProfile()
        val m = manager
        if (m != null && qrUri == null && qrError == null) {
            val username = (identity?.mainDpnsName ?: identity?.dpnsName)?.trim().orEmpty()
            try {
                qrUri = w.dashpay.buildAutoAcceptQr(idBytes, username, m.mnemonicResolverHandle)
            } catch (e: Exception) {
                qrError = "Couldn't build the QR: ${e.message ?: "unknown error"}"
            }
        }
    }

    val displayName = profile?.displayName?.trim()?.takeIf { it.isNotEmpty() }
        ?: (identity?.mainDpnsName ?: identity?.dpnsName)
        ?: (Base58.encode(idBytes).take(12) + "…")

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Your Profile") },
                navigationIcon = {
                    TextButton(
                        onClick = { navController.popBackStack() },
                        modifier = Modifier.testTag("dashpay.profile.done"),
                    ) { Text("Done") }
                },
                actions = {
                    TextButton(
                        onClick = {
                            if (!isEditing) {
                                displayNameField = profile?.displayName.orEmpty()
                                publicMessageField = profile?.publicMessage.orEmpty()
                                avatarUrlField = profile?.avatarUrl.orEmpty()
                                saveError = null
                            }
                            isEditing = !isEditing
                        },
                        modifier = Modifier.testTag("dashpay.profile.edit"),
                    ) { Text(if (isEditing) "Cancel" else "Edit") }
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
            if (isEditing) {
                FormSection(title = "Edit profile") {
                    OutlinedTextField(
                        value = displayNameField,
                        onValueChange = { displayNameField = it },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("Display name") },
                        singleLine = true,
                    )
                    OutlinedTextField(
                        value = publicMessageField,
                        onValueChange = { publicMessageField = it },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("Public message") },
                    )
                    OutlinedTextField(
                        value = avatarUrlField,
                        onValueChange = { avatarUrlField = it },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("Avatar URL") },
                        singleLine = true,
                    )
                    saveError?.let {
                        Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                    }
                    SubmitButton(
                        text = "Save",
                        isLoading = isSaving,
                        enabled = !isSaving,
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        val m = manager ?: return@SubmitButton
                        val w = wallet ?: return@SubmitButton
                        isSaving = true
                        saveError = null
                        scope.launch {
                            try {
                                w.dashpay.createOrUpdateProfile(
                                    identityId = idBytes,
                                    displayName = displayNameField.trim().ifEmpty { null },
                                    publicMessage = publicMessageField.trim().ifEmpty { null },
                                    avatarUrl = avatarUrlField.trim().ifEmpty { null },
                                    doCreate = !profileExists,
                                    signerHandle = m.signerHandle,
                                )
                                loadProfile()
                                isEditing = false
                            } catch (e: Exception) {
                                saveError = e.message ?: "Save failed"
                            } finally {
                                isSaving = false
                            }
                        }
                    }
                }
            } else {
                FormSection {
                    Column(
                        modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        DashPayAvatar(profile?.avatarUrl, displayName, size = 96.dp)
                        Text(displayName, style = MaterialTheme.typography.titleLarge)
                        (identity?.mainDpnsName ?: identity?.dpnsName)?.let {
                            Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary)
                        }
                        profile?.publicMessage?.trim()?.takeIf { it.isNotEmpty() }?.let {
                            Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
                        }
                    }
                }
            }

            FormSection(title = "Identity") {
                Text(
                    Base58.encode(idBytes),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            FormSection(title = "Add me (DIP-15 QR)") {
                val uri = qrUri
                when {
                    uri != null -> {
                        val bitmap = qrBitmap
                        Column(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            if (bitmap != null) {
                                Image(
                                    bitmap = bitmap.asImageBitmap(),
                                    contentDescription = "Auto-accept QR",
                                    modifier = Modifier
                                        .size(200.dp)
                                        .background(Color.White, RoundedCornerShape(12.dp))
                                        .padding(8.dp),
                                )
                            }
                            Text(
                                "Scan to send me a contact request — auto-accepted for 1 hour.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                textAlign = TextAlign.Center,
                            )
                            Text(
                                uri,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 2,
                                modifier = Modifier.testTag("dashpay.profile.qrURI"),
                            )
                        }
                    }
                    qrError != null -> Text(
                        qrError!!,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.tertiary,
                    )
                    else -> Row(
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        CircularProgressIndicator(modifier = Modifier.size(18.dp), strokeWidth = 2.dp)
                        Text("Generating QR…", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
    }
}
