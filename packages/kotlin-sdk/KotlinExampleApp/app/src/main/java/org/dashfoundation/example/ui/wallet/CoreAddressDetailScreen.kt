package org.dashfoundation.example.ui.wallet

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.withResumed
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.currentStateAsState
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.dashsdk.wallet.CoreAddressPrivateKey
import org.dashfoundation.example.LocalSecureScreen
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.SensitiveClipboard

/**
 * One core address and, behind device authentication, its private key. Port
 * of `CoreAddressDetailView` (`StorageRecordDetailViews.swift`, SwiftExampleApp),
 * reached from an address row of [AccountDetailScreen].
 *
 * **View Private Key** asks for a strong biometric or the device credential
 * ([BiometricGate], `AuthPrompt`) first, and only after that calls
 * `PlatformWalletManager.coreAddressPrivateKey`, which derives the key in Rust
 * with the mnemonic read through the manager's resolver. Rust decides which
 * addresses have a key to reveal; a refusal shows its message.
 *
 * The key lives only in this composition's memory: never in navigation
 * arguments, saved state or Room. It is wiped when the screen leaves, when the
 * app goes to the background, and before a new reveal. A derivation still
 * running when the app goes to the background is dropped, and its key wiped
 * when it returns ([RevealSession]). While a key is shown the window is
 * FLAG_SECURE (no screenshots, recordings or recents thumbnail), and the rows
 * are replaced by a placeholder while the screen is not resumed.
 *
 * No balance is shown: the per-address balance in Room is pool metadata that
 * the wallet does not keep up to date.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalStdlibApi::class)
@Composable
fun CoreAddressDetailScreen(
    walletIdHex: String,
    address: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val lifecycleOwner = LocalLifecycleOwner.current
    val lifecycleState by lifecycleOwner.lifecycle.currentStateAsState()

    val record by remember(address) {
        container.database.coreAddressDao().observeByAddress(address)
    }.collectAsStateWithLifecycle(initialValue = null)

    // The revealed key, the reveal in flight, and the transient states.
    var privateKey by remember { mutableStateOf<CoreAddressPrivateKey?>(null) }
    val session = remember { RevealSession() }
    var isRevealing by remember { mutableStateOf(false) }
    var revealError by remember { mutableStateOf<String?>(null) }
    var copiedLabel by remember { mutableStateOf<String?>(null) }

    fun clearReveal() {
        privateKey?.clear()
        privateKey = null
        copiedLabel = null
        isRevealing = false
        session.invalidate()
    }

    // Leaving the screen wipes the key.
    DisposableEffect(Unit) { onDispose { clearReveal() } }

    // Going to the background wipes a shown key and drops a derivation in
    // flight. A stop while the prompt is up is left alone: the prompt's
    // device-credential screen stops this activity itself (see RevealSession).
    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event != Lifecycle.Event.ON_STOP) return@LifecycleEventObserver
            if (privateKey != null) {
                clearReveal()
            } else if (session.phase == RevealSession.Phase.DERIVING) {
                session.onStop()
                isRevealing = false
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }

    // FLAG_SECURE while a key is shown.
    if (privateKey != null) {
        val setSecure = LocalSecureScreen.current
        DisposableEffect(Unit) {
            setSecure(true)
            onDispose { setSecure(false) }
        }
    }

    fun authorizeAndReveal() {
        val token = session.begin() ?: return
        isRevealing = true
        revealError = null
        scope.launch {
            val outcome = container.biometricGate.authenticate(
                title = "View Private Key",
                subtitle = "Reveal the private key of this address.",
            )
            if (outcome != BiometricGate.AuthOutcome.AUTHORIZED) {
                session.abandon(token)
                isRevealing = false
                revealError = when (outcome) {
                    BiometricGate.AuthOutcome.UNAVAILABLE ->
                        "Authentication is unavailable on this device."
                    BiometricGate.AuthOutcome.FAILED -> "Authorization failed."
                    else -> null
                }
                return@launch
            }
            // Derive only once the screen is back in front (the PIN screen
            // has closed), so a stop from here on is the owner leaving.
            lifecycleOwner.lifecycle.withResumed {}
            if (!session.authorized(token)) return@launch
            val manager = container.walletManagerStore.activeManager.value
            if (manager == null) {
                session.abandon(token)
                revealError = "The owning wallet is not loaded."
                isRevealing = false
                return@launch
            }
            val key = try {
                manager.coreAddressPrivateKey(walletIdHex.hexToByteArray(), address)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (session.finish(token)) {
                    revealError = e.message ?: e.javaClass.simpleName
                    isRevealing = false
                }
                return@launch
            }
            if (!session.finish(token)) {
                key.clear()
                return@launch
            }
            privateKey?.clear()
            privateKey = key
            isRevealing = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Address") },
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
            FormSection(title = "Address") {
                Text(
                    address,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.padding(vertical = 6.dp),
                )
                record?.let { row ->
                    LabeledContent("Derivation Path", row.derivationPath)
                    LabeledContent("Index", "#${row.addressIndex}")
                    LabeledContent("Used", if (row.isUsed) "Yes" else "No")
                }
            }

            FormSection(title = "Private Key") {
                val key = privateKey
                if (key != null) {
                    if (lifecycleState.isAtLeast(Lifecycle.State.RESUMED)) {
                        KeyRow(
                            label = "Hex",
                            value = key.hex,
                            copied = copiedLabel == "Hex",
                            tag = "coreAddress.privateKey.hex",
                            onCopy = {
                                SensitiveClipboard.copy(context, container.applicationScope, key.hex)
                                copiedLabel = "Hex"
                            },
                        )
                        KeyRow(
                            label = "WIF",
                            value = key.wif,
                            copied = copiedLabel == "WIF",
                            tag = "coreAddress.privateKey.wif",
                            onCopy = {
                                SensitiveClipboard.copy(context, container.applicationScope, key.wif)
                                copiedLabel = "WIF"
                            },
                        )
                    } else {
                        Text(
                            "Hidden while the app is not active",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.testTag("coreAddress.privateKey.hidden"),
                        )
                    }
                    Text(
                        "Anyone with this key controls this address's funds. Never share it.",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    TextButton(
                        onClick = { authorizeAndReveal() },
                        enabled = !isRevealing,
                        modifier = Modifier
                            .fillMaxWidth()
                            .testTag("coreAddress.revealPrivateKeyButton"),
                    ) { Text("View Private Key") }
                    Text(
                        "The private key grants full control of this address's funds. " +
                            "Only reveal it somewhere private.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    revealError?.let {
                        Text(
                            it,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("coreAddress.revealError"),
                        )
                    }
                }
            }
        }
    }
}

/** One monospaced key row (hex or WIF); tapping it copies the value. */
@Composable
private fun KeyRow(
    label: String,
    value: CharArray,
    copied: Boolean,
    tag: String,
    onCopy: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onCopy)
            .padding(vertical = 6.dp)
            .testTag(tag),
    ) {
        Text(
            if (copied) "$label (Copied)" else label,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        // A String is needed to draw the text; it is a copy the JVM cannot
        // wipe, rebuilt only while the key is shown.
        Text(
            String(value),
            style = MaterialTheme.typography.bodySmall,
            fontFamily = FontFamily.Monospace,
        )
    }
}
