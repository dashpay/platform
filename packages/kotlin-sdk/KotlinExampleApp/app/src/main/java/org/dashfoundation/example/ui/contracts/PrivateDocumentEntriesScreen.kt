package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.currentStateAsState
import androidx.navigation.NavHostController
import java.text.DateFormat
import java.util.Date
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.dashfoundation.example.util.Base58
import org.dashfoundation.dashsdk.wallet.PrivateDocumentAuthMode
import org.dashfoundation.dashsdk.wallet.PrivateDocumentDirection
import org.dashfoundation.dashsdk.wallet.PrivateDocumentEntriesRead
import org.dashfoundation.dashsdk.wallet.PrivateDocumentEntry
import org.dashfoundation.dashsdk.wallet.PrivateDocumentPayload
import org.dashfoundation.dashsdk.wallet.PrivateDocumentReadCursor
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.identity.HoldSecureWindow
import org.dashfoundation.example.util.hexToBytes

/** The store pages one Read or Read more asks for. One tap, one call: the screen never reads on by itself. */
internal const val PRIVATE_ENTRIES_PAGES_PER_READ = 8

/**
 * Reads a private document type's store (protocol version 15) for one of the app's wallets
 * and lists the entries its identities opened or sent: port of
 * `PrivateDocumentEntriesView.swift`. Covers TEST_PLAN DOC-17 (Swift DOC-17).
 *
 * The read itself (which identities, which keys, how a payload reads, where the next read
 * starts) is Rust's: this screen calls `PrivateDocuments.readPrivateDocumentEntries` once per
 * tap and shows what came back. Nothing is persisted; the entries live in screen state. The
 * wallet is looked up from the active manager at tap time, so no native handle is held in
 * composition.
 *
 * While the screen is shown its window is marked secure, so the recents thumbnail and
 * screenshots show no decrypted entry, and whenever the app leaves the foreground the entries
 * are covered (`privateEntries.privacyCover`), as Swift covers them off `scenePhase.active`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PrivateDocumentEntriesScreen(
    contractIdHex: String,
    documentTypeName: String,
    navController: NavHostController,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val scope = rememberCoroutineScope()
    val contractId = remember(contractIdHex) { contractIdHex.hexToBytes() }
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallets by remember(network) {
        container.database.walletDao().observeByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(initialValue = emptyList())
    val identities by remember(network) {
        container.database.identityDao().observeByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(initialValue = emptyList())
    // The ids of the wallets the manager has loaded, not the wallets: a screen holds no native handle
    val loadedWalletHexes by remember(manager) {
        manager?.wallets?.map { it.keys } ?: flowOf(emptySet())
    }.collectAsStateWithLifecycle(initialValue = emptySet())

    var selectedWalletHex by remember { mutableStateOf<String?>(null) }
    // The entries read since the last Read (or the last read that restarted), in store order
    var entries by remember { mutableStateOf<List<PrivateDocumentEntry>>(emptyList()) }
    // The latest read's answer; null before the first read
    var lastRead by remember { mutableStateOf<PrivateDocumentEntriesRead?>(null) }
    // Pages and refused entries across this screen's reads since the last Read, or the last
    // read that restarted
    var pagesReadTotal by remember { mutableLongStateOf(0L) }
    var rejectedTotal by remember { mutableLongStateOf(0L) }
    var isReading by remember { mutableStateOf(false) }
    var failure by remember { mutableStateOf<ReadFailure?>(null) }

    fun resetRead() {
        entries = emptyList()
        lastRead = null
        pagesReadTotal = 0
        rejectedTotal = 0
        failure = null
    }

    // Default to the first wallet of this network; another wallet's identities open other entries
    LaunchedEffect(wallets) {
        val current = selectedWalletHex
        if (current == null || wallets.none { it.walletId.toHexString() == current }) {
            selectedWalletHex = wallets.firstOrNull()?.walletId?.toHexString()
            resetRead()
        }
    }
    val selectedLoaded = selectedWalletHex != null && selectedWalletHex in loadedWalletHexes

    fun read(from: PrivateDocumentReadCursor?) {
        val walletHex = selectedWalletHex ?: return
        val activeManager = manager ?: return
        val wallet = activeManager.wallets.value[walletHex] ?: return
        isReading = true
        failure = null
        scope.launch {
            try {
                val result = wallet.privateDocuments.readPrivateDocumentEntries(
                    contractId = contractId,
                    documentTypeName = documentTypeName,
                    coreSignerHandle = activeManager.mnemonicResolverHandle,
                    from = from,
                    maxPages = PRIVATE_ENTRIES_PAGES_PER_READ,
                )
                // The user picked another wallet while this read ran
                if (walletHex != selectedWalletHex) return@launch
                // A read that restarted (the wallet's keys changed since the cursor) read from
                // the store's start: it replaces the list
                if (from != null && !result.restarted) {
                    entries = entries + result.entries
                    pagesReadTotal += result.pagesRead
                    rejectedTotal += result.rejectedCount
                } else {
                    entries = result.entries
                    pagesReadTotal = result.pagesRead
                    rejectedTotal = result.rejectedCount
                }
                lastRead = result
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (walletHex == selectedWalletHex) failure = ReadFailure.of(e)
            } finally {
                isReading = false
            }
        }
    }

    // The app's FLAG_SECURE toggle (LocalSecureScreen), held while this screen is shown
    HoldSecureWindow()
    val lifecycleState by LocalLifecycleOwner.current.lifecycle.currentStateAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("$documentTypeName Entries") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Box(Modifier.fillMaxSize().padding(padding)) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                FormSection(title = "Wallet") {
                    if (wallets.isEmpty()) {
                        Text(
                            "No wallets on this network. Create or import a wallet first.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    } else {
                        val selected = wallets.firstOrNull { it.walletId.toHexString() == selectedWalletHex }
                            ?: wallets.first()
                        AccessiblePicker(
                            label = "Wallet",
                            options = wallets,
                            selected = selected,
                            optionLabel = ::walletLabel,
                            testTag = "privateEntries.walletPicker",
                            enabled = !isReading,
                        ) { picked ->
                            selectedWalletHex = picked.walletId.toHexString()
                            resetRead()
                        }
                        if (selectedWalletHex != null && !selectedLoaded) {
                            Text(
                                "This wallet is not loaded yet.",
                                style = MaterialTheme.typography.bodySmall,
                                color = ClaimOrange,
                            )
                        }
                    }
                    EntriesFooter(
                        "Entries name nobody: the wallet finds its identities' entries by trying " +
                            "to open each one with their keys, and recovers the ones their keys sealed.",
                    )
                }

                FormSection {
                    SubmitButton(
                        text = if (lastRead == null) "Read" else "Read from the start",
                        isLoading = isReading,
                        enabled = selectedLoaded,
                        modifier = Modifier.fillMaxWidth().testTag("privateEntries.readButton"),
                    ) { read(from = null) }
                    val read = lastRead
                    if (read != null && !read.complete && read.identityCount > 0) {
                        OutlinedButton(
                            onClick = { read(from = read.cursor) },
                            enabled = !isReading && selectedLoaded,
                            modifier = Modifier.fillMaxWidth().padding(top = 8.dp)
                                .testTag("privateEntries.readMoreButton"),
                        ) { Text("Read more") }
                    }
                    EntriesFooter("Each read covers at most $PRIVATE_ENTRIES_PAGES_PER_READ pages of the store.")
                }

                failure?.let { FailureSection(it) }

                lastRead?.let { read ->
                    ResultSection(read, entries) { id -> identityName(id, identities) }
                    StatusSection(read, pagesReadTotal, rejectedTotal)
                }
            }

            if (!lifecycleState.isAtLeast(Lifecycle.State.RESUMED)) {
                PrivacyCover()
            }
        }
    }
}

/** How a failed read is shown: a network before protocol version 15 as that, everything else as the error says. */
internal sealed class ReadFailure {
    /** The network does not serve private stores yet. */
    data object NotActive : ReadFailure()

    /** Shown as given. */
    data class Message(val text: String) : ReadFailure()

    companion object {
        fun of(error: Throwable): ReadFailure = when (error) {
            is DashSdkError.PlatformWallet.ProtocolVersionNotActive -> NotActive
            is DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion -> Message(
                "This build's SDK is pinned below protocol version 15, which private document " +
                    "types need. Raise or remove the SDK's protocol version pin.",
            )
            else -> Message(error.message ?: "Read failed")
        }
    }
}

/**
 * Opaque cover over a private screen while the app is not in the foreground. It only hides what
 * is shown; the create screen uses it too.
 */
@Composable
internal fun PrivacyCover(
    text: String = "Private entries hidden",
    testTag: String = "privateEntries.privacyCover",
) {
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .testTag(testTag),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text,
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun FailureSection(failure: ReadFailure) {
    FormSection {
        when (failure) {
            ReadFailure.NotActive -> Text(
                "This network does not serve private document stores yet: they arrive with " +
                    "protocol version 15.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("privateEntries.notActive"),
            )
            is ReadFailure.Message -> SelectionContainer {
                Text(
                    failure.text,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("privateEntries.error"),
                )
            }
        }
    }
}

@Composable
private fun ResultSection(
    read: PrivateDocumentEntriesRead,
    entries: List<PrivateDocumentEntry>,
    identityName: (ByteArray) -> String,
) {
    when {
        read.identityCount == 0L -> FormSection {
            Text(
                "No identity of this wallet can open entries, so nothing was read. An " +
                    "identity needs an encryption or decryption key to be sent entries.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("privateEntries.noIdentity"),
            )
        }
        entries.isEmpty() -> FormSection {
            Text(
                "Nothing for this wallet in the entries read.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("privateEntries.empty"),
            )
        }
        else -> FormSection(title = "Entries (${entries.size})") {
            entries.forEachIndexed { index, entry ->
                if (index > 0) HorizontalDivider()
                PrivateDocumentEntryRow(entry, identityName)
            }
        }
    }
}

@Composable
private fun StatusSection(read: PrivateDocumentEntriesRead, pagesReadTotal: Long, rejectedTotal: Long) {
    FormSection(title = "Status") {
        Column(Modifier.testTag("privateEntries.status")) {
            LabeledContent("Pages read", "$pagesReadTotal")
            LabeledContent("Store size", read.cursor.provedTotalCount?.toString() ?: "Unknown")
            LabeledContent("Next position", "${read.cursor.position}")
            // Entries that opened for the wallet but whose envelope was refused; a payload the
            // schema refused shows on its row
            LabeledContent("Refused envelopes", "$rejectedTotal")
            LabeledContent("Read to the end", if (read.complete) "Yes" else "No")
        }
        if (read.unscannedKeyCount > 0) {
            Column(Modifier.padding(top = 8.dp).testTag("privateEntries.unscannedKeys")) {
                LabeledContent("Keys not scanned", "${read.unscannedKeyCount}")
                Text(
                    "Entries sealed to these keys (such as a DashPay Connect app key) are not read.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/**
 * One opened entry: which way it went, where and when it was appended, who it is for, who it
 * claims to be from, and what it holds (← `PrivateDocumentEntryRow`).
 */
@Composable
private fun PrivateDocumentEntryRow(entry: PrivateDocumentEntry, identityName: (ByteArray) -> String) {
    val received = entry.direction == PrivateDocumentDirection.RECEIVED
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp)
            .testTag("privateEntries.entry.${entry.rowId}"),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row {
            Text(
                if (received) "Received" else "Recovered (outgoing)",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = if (received) ReceivedGreen else Color(0xFF1565C0),
                modifier = Modifier.weight(1f),
            )
            Text(
                "#${entry.position}",
                style = MaterialTheme.typography.labelSmall,
                fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Text(
            "Appended ${DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.MEDIUM)
                .format(Date(entry.blockTimeMs))} · block ${entry.blockHeight}",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        // Recovery with an outgoing viewing key shows only that one of the wallet's keys can
        // recover this ciphertext, not that the wallet appended it: a later copy recovers too
        LabeledContent(
            if (received) "Opened by" else "Recovered with",
            "${identityName(entry.walletIdentityId)} (key ${entry.walletKeyId})",
        )
        LabeledContent("Recipient", identityName(entry.recipientIdentityId))
        LabeledContent("Authentication", entry.authMode.label)
        entry.claimedSenderIdentityId?.let { sender ->
            Text(
                "Claimed sender: ${identityName(sender)}" +
                    (entry.claimedSenderKeyId?.let { " (key $it)" } ?: "") + ", not verified",
                style = MaterialTheme.typography.bodySmall,
                color = ClaimOrange,
            )
        }
        when (val payload = entry.payload) {
            is PrivateDocumentPayload.Properties -> propertyRows(payload.json).forEach { (key, value) ->
                Row {
                    Text(
                        key,
                        style = MaterialTheme.typography.bodySmall,
                        fontWeight = FontWeight.Medium,
                        modifier = Modifier.weight(1f),
                    )
                    SelectionContainer {
                        Text(
                            value,
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                }
            }
            is PrivateDocumentPayload.Refused -> Text(
                "Payload refused: ${payload.reason}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}

/** Unique per row: an entry between two of the wallet's identities is listed once each way. */
internal val PrivateDocumentEntry.rowId: String
    get() = "$position." + if (direction == PrivateDocumentDirection.RECEIVED) "received" else "sent"

private val PrivateDocumentAuthMode.label: String
    get() = when (this) {
        PrivateDocumentAuthMode.ANONYMOUS -> "Anonymous"
        PrivateDocumentAuthMode.DENIABLE_MAC -> "Deniable MAC"
        PrivateDocumentAuthMode.SIGNATURE -> "Signature"
    }

/**
 * The JSON object's members as text, sorted by name: a string as it is, any other value as
 * its JSON. Text that is not a JSON object is shown whole under "properties".
 */
internal fun propertyRows(json: String): List<Pair<String, String>> {
    val parsed = try {
        Json.parseToJsonElement(json) as? JsonObject
    } catch (e: IllegalArgumentException) {
        null
    } ?: return listOf("properties" to json)
    return parsed.keys.sorted().map { key ->
        val value = parsed.getValue(key)
        key to if (value is JsonPrimitive && value.isString) value.content else value.toString()
    }
}

/**
 * A known identity's alias or DPNS name on the current network, else its base58 id
 * shortened (← Swift `identityName(for:)`).
 */
internal fun identityName(id: ByteArray, identities: List<IdentityEntity>): String {
    val known = identities.firstOrNull { it.identityId.contentEquals(id) }
    known?.alias?.takeIf { it.isNotBlank() }?.let { return it }
    known?.dpnsName?.takeIf { it.isNotBlank() }?.let { return it }
    val base58 = Base58.encode(id)
    return if (base58.length > 14) "${base58.take(8)}…${base58.takeLast(4)}" else base58
}

private fun walletLabel(wallet: WalletEntity): String {
    val fingerprint = wallet.walletId.take(4).joinToString("") { "%02x".format(it) }
    return "${wallet.name ?: "Wallet"} ($fingerprint)"
}

private fun ByteArray.toHexString(): String = joinToString("") { "%02x".format(it) }

private val ReceivedGreen = Color(0xFF2E7D32)
private val ClaimOrange = Color(0xFFEF6C00)

/** A form section's explanatory footer. */
@Composable
private fun EntriesFooter(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(top = 8.dp),
    )
}
