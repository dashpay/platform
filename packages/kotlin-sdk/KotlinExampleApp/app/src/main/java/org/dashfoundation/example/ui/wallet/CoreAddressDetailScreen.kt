package org.dashfoundation.example.ui.wallet

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import android.os.PersistableBundle
import android.os.SystemClock
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
import androidx.compose.runtime.produceState
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
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.currentStateAsState
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.dashsdk.security.BiometricGate
import org.dashfoundation.dashsdk.wallet.CoreAddressPrivateKey
import org.dashfoundation.example.LocalSecureScreen
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.util.formatDuffs

/**
 * One core address and, behind device authentication, its private key. Port
 * of `CoreAddressDetailView` (`StorageRecordDetailViews.swift`, SwiftExampleApp),
 * reached from an address row of [AccountDetailScreen].
 *
 * **View Private Key** asks for a strong biometric or the device credential
 * ([BiometricGate], `AuthPrompt`) first, and only after that calls
 * `ManagedPlatformWallet.coreAddressPrivateKey`, which derives the key in Rust
 * with the mnemonic read through the manager's resolver. Rust decides which
 * addresses have a key to reveal; a refusal shows its message.
 *
 * The key lives only in this composition's memory: never in navigation
 * arguments, saved state or Room. It is wiped when the screen leaves, when the
 * app goes to the background, and before a new reveal. While a key is shown the
 * window is FLAG_SECURE (no screenshots, recordings or recents thumbnail), and
 * the rows are replaced by a placeholder while the screen is not resumed.
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

    val record by produceState<CoreAddressEntity?>(initialValue = null, address) {
        value = container.database.coreAddressDao().getByAddress(address)
    }

    // The revealed key, its in-flight request token, and the transient states.
    var privateKey by remember { mutableStateOf<CoreAddressPrivateKey?>(null) }
    var revealRequest by remember { mutableStateOf(0L) }
    var isRevealing by remember { mutableStateOf(false) }
    var revealError by remember { mutableStateOf<String?>(null) }
    var copiedLabel by remember { mutableStateOf<String?>(null) }

    fun clearReveal() {
        privateKey?.clear()
        privateKey = null
        copiedLabel = null
        isRevealing = false
        revealRequest += 1
    }

    // Leaving the screen wipes the key.
    DisposableEffect(Unit) { onDispose { clearReveal() } }

    // Going to the background wipes a SHOWN key. A reveal still in flight is
    // left alone: the device-credential screen of the prompt itself stops
    // this activity, and must not cancel its own reveal.
    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_STOP && privateKey != null) clearReveal()
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
        if (isRevealing) return
        val request = revealRequest + 1
        revealRequest = request
        isRevealing = true
        revealError = null
        scope.launch {
            val outcome = container.biometricGate.authenticate(
                title = "View Private Key",
                subtitle = "Reveal the private key of this address.",
            )
            if (revealRequest != request) return@launch
            when (outcome) {
                BiometricGate.AuthOutcome.AUTHORIZED -> Unit
                BiometricGate.AuthOutcome.DENIED -> {
                    isRevealing = false
                    return@launch
                }
                BiometricGate.AuthOutcome.UNAVAILABLE -> {
                    revealError = "Authentication is unavailable on this device."
                    isRevealing = false
                    return@launch
                }
                BiometricGate.AuthOutcome.FAILED -> {
                    revealError = "Authorization failed."
                    isRevealing = false
                    return@launch
                }
            }
            val manager = container.walletManagerStore.activeManager.value
            val wallet = manager?.wallet(forWalletId = walletIdHex.hexToByteArray())
            if (manager == null || wallet == null) {
                revealError = "The owning wallet is not loaded."
                isRevealing = false
                return@launch
            }
            val key = try {
                wallet.coreAddressPrivateKey(address, manager.mnemonicResolverHandle)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (revealRequest == request) {
                    revealError = e.message ?: e.javaClass.simpleName
                    isRevealing = false
                }
                return@launch
            }
            if (revealRequest != request) {
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
                    LabeledContent("Balance", formatDuffs(row.balance))
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
                                copySensitive(context, container.applicationScope, key.hex)
                                copiedLabel = "Hex"
                            },
                        )
                        KeyRow(
                            label = "WIF",
                            value = key.wif,
                            copied = copiedLabel == "WIF",
                            tag = "coreAddress.privateKey.wif",
                            onCopy = {
                                copySensitive(context, container.applicationScope, key.wif)
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

/**
 * Copy [value] to the clipboard marked sensitive (API 33+ masks the system
 * preview; earlier releases cannot), under a per-copy label, and clear it after
 * 60 s if that copy is still the current clip. Android has no clipboard expiry,
 * so the clear runs on the application scope: a composition scope would die
 * with the screen and leave the key there.
 */
private fun copySensitive(
    context: Context,
    applicationScope: CoroutineScope,
    value: CharArray,
) {
    val label = "private-key-${SystemClock.elapsedRealtimeNanos()}"
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val clip = ClipData.newPlainText(label, String(value))
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        clip.description.extras = PersistableBundle().apply {
            putBoolean(ClipDescription.EXTRA_IS_SENSITIVE, true)
        }
    }
    clipboard.setPrimaryClip(clip)
    val appContext = context.applicationContext
    applicationScope.launch {
        delay(60_000)
        val manager = appContext.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        if (manager.primaryClipDescription?.label == label) manager.clearPrimaryClip()
    }
}
