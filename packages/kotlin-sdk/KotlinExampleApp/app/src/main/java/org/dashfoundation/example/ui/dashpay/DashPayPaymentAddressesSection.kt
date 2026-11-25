package org.dashfoundation.example.ui.dashpay

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressKind
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressSupport
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressUpdate
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddresses
import org.dashfoundation.example.ui.components.FormSection

// The public payment addresses a DashPay profile publishes (Core, Platform,
// shielded). Port of SwiftExampleApp `DashPayPaymentAddressesSection.swift`.
// Parsing, checking and rendering an address, and whether the network lets a
// profile set one, all come from platform-wallet through
// `DashPayPaymentAddresses.bytes/string` and `Dashpay.paymentAddressSupport()`.
// These views only show the results and collect the user's choice.

/** Display title of an address kind (Swift `DashPayPaymentAddressKind.title`). */
val DashPayPaymentAddressKind.title: String
    get() = when (this) {
        DashPayPaymentAddressKind.CORE -> "Core"
        DashPayPaymentAddressKind.PLATFORM -> "Platform"
        DashPayPaymentAddressKind.SHIELDED -> "Shielded"
    }

/** testTag component, Swift's `identifierName`. */
val DashPayPaymentAddressKind.identifierName: String
    get() = when (this) {
        DashPayPaymentAddressKind.CORE -> "core"
        DashPayPaymentAddressKind.PLATFORM -> "platform"
        DashPayPaymentAddressKind.SHIELDED -> "shielded"
    }

private val DashPayPaymentAddressKind.placeholder: String
    get() = when (this) {
        DashPayPaymentAddressKind.CORE -> "Core address (X… / y…)"
        DashPayPaymentAddressKind.PLATFORM -> "Platform address (dash1… / tdash1…)"
        DashPayPaymentAddressKind.SHIELDED -> "Shielded address"
    }

private const val PUBLIC_NOTICE =
    "Published addresses are public. Anyone can see them, and payments to them can be linked to this profile."

private const val LAST_KNOWN_NOTE =
    "Read from this contact's profile when it was last checked. They may have changed since."

private const val CORE_REUSE_NOTE =
    "This wallet can offer the same address again for another identity's profile until it receives a payment. " +
        "Publishing one address in two profiles links them."

/** Render [bytes] for display, or null when the wallet does not read them as a [kind] address. */
private fun renderAddress(kind: DashPayPaymentAddressKind, bytes: ByteArray, network: Network): String? =
    runCatching { DashPayPaymentAddresses.string(kind, bytes, network) }.getOrNull()

private fun copyToClipboard(context: Context, label: String, text: String) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
}

// ── Read-only list ────────────────────────────────────────────────────

/**
 * Read-only "Payment addresses" section for a profile: one row per published
 * address with a copy button. With [showsUnpublished] false the section is
 * hidden when the profile publishes none (contact detail); with it true each
 * unpublished kind reads "Not published" (own profile). Port of Swift
 * `DashPayPublishedPaymentAddressesSection`; testTags `"$identifierPrefix.<kind>"`
 * and `"$identifierPrefix.<kind>.copy"`.
 *
 * [isLastKnown] marks addresses read from a cached copy of someone else's
 * profile (contact detail): the section is titled as the last known
 * addresses and says they may have changed since
 * (`"$identifierPrefix.lastKnownNote"`).
 */
@Composable
fun DashPayPublishedPaymentAddressesSection(
    profile: DashPayProfile?,
    network: Network?,
    identifierPrefix: String,
    showsUnpublished: Boolean,
    isLastKnown: Boolean = false,
) {
    val context = LocalContext.current
    var copiedKind by remember { mutableStateOf<DashPayPaymentAddressKind?>(null) }
    val kinds = DashPayPaymentAddressKind.entries.filter {
        showsUnpublished || profile?.paymentAddress(it) != null
    }
    if (kinds.isEmpty()) return

    FormSection(title = if (isLastKnown) "Last known payment addresses" else "Payment addresses") {
        kinds.forEach { kind ->
            val bytes = profile?.paymentAddress(kind)
            val rendered = if (bytes != null && network != null) renderAddress(kind, bytes, network) else null
            Row(
                modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                verticalAlignment = Alignment.Top,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Column(
                    modifier = Modifier.weight(1f).testTag("$identifierPrefix.${kind.identifierName}"),
                    verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    Text(kind.title, style = MaterialTheme.typography.titleSmall)
                    when {
                        rendered != null -> SelectionContainer {
                            Text(
                                rendered,
                                style = MaterialTheme.typography.bodySmall,
                                fontFamily = FontFamily.Monospace,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        // Neutral: a build without shielded support can hold a
                        // valid shielded address it cannot render.
                        bytes != null -> Text(
                            "Published, but this address could not be displayed on ${network?.displayName ?: "this network"}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.tertiary,
                        )
                        else -> Text(
                            "Not published",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                if (rendered != null) {
                    IconButton(
                        onClick = {
                            copyToClipboard(context, "${kind.title} address", rendered)
                            copiedKind = kind
                        },
                        modifier = Modifier.testTag("$identifierPrefix.${kind.identifierName}.copy"),
                    ) {
                        Icon(
                            if (copiedKind == kind) Icons.Default.Check else Icons.Default.ContentCopy,
                            contentDescription = "Copy ${kind.title} address",
                        )
                    }
                }
            }
        }
        if (isLastKnown) {
            Text(
                LAST_KNOWN_NOTE,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("$identifierPrefix.lastKnownNote"),
            )
        }
        Text(
            PUBLIC_NOTICE,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

// ── Editor model ──────────────────────────────────────────────────────

/**
 * Keep / set / remove choice for each payment address of the profile being
 * edited. A typed or filled-in address is checked by platform-wallet as it
 * changes; its error is shown under the field. Port of Swift
 * `DashPayPaymentAddressEditorModel`.
 *
 * [parse] and the wallet lookups are injected so the model's state rules are
 * unit-testable without the native library; production passes the SDK calls.
 */
class DashPayPaymentAddressEditorModel(
    val existing: DashPayProfile?,
    private val parse: (DashPayPaymentAddressKind, String, Network) -> ByteArray = DashPayPaymentAddresses::bytes,
) {
    enum class Mode { KEEP, SET, REMOVE }

    data class Row(
        val mode: Mode = Mode.KEEP,
        val text: String = "",
        /** Stored bytes of [text], when platform-wallet accepts it. */
        val bytes: ByteArray? = null,
        /** platform-wallet's reason [text] is not an address. */
        val error: String? = null,
        val isFetchingWalletAddress: Boolean = false,
        val walletAddressError: String? = null,
        /** True while [text] is the address "Use this wallet's address" filled in; a later edit clears it. */
        val filledFromWallet: Boolean = false,
        /**
         * Changes on every text or mode change. A fill that awaits applies
         * its result only if this is unchanged, so a newer edit wins.
         */
        val revision: Int = 0,
    )

    private val rows = mutableStateMapOf<DashPayPaymentAddressKind, Row>().apply {
        DashPayPaymentAddressKind.entries.forEach { put(it, Row()) }
    }

    var network by mutableStateOf<Network?>(null)
        private set
    var networkError by mutableStateOf<String?>(null)
        private set
    var support by mutableStateOf<DashPayPaymentAddressSupport?>(null)
        private set
    var supportError by mutableStateOf<String?>(null)
        private set
    var isLoadingSupport by mutableStateOf(false)
        private set

    /**
     * The Core address `nextReceiveAddress` handed out this editor session,
     * reused for later taps (Swift fetches it once for the same reason).
     */
    private var coreWalletAddress: String? = null
    private var started = false

    fun row(kind: DashPayPaymentAddressKind): Row = rows[kind] ?: Row()

    fun published(kind: DashPayPaymentAddressKind): ByteArray? = existing?.paymentAddress(kind)

    /**
     * Take the wallet's [network] and ask which addresses the network lets a
     * profile set ([loadSupport], `Dashpay.paymentAddressSupport()`). Runs
     * once per editor; a null [network] means no wallet is available.
     */
    suspend fun start(network: Network?, loadSupport: (suspend () -> DashPayPaymentAddressSupport)?) {
        if (started) return
        started = true
        if (network == null || loadSupport == null) {
            networkError = "No wallet available for this identity."
            supportError = networkError
            return
        }
        this.network = network
        isLoadingSupport = true
        try {
            support = loadSupport()
        } catch (e: Exception) {
            supportError = "Couldn't check which addresses the network supports: ${e.message ?: "unknown error"}"
        } finally {
            isLoadingSupport = false
        }
    }

    /** Why Set is unavailable for [kind], or null when it is available. */
    fun setUnavailableReason(kind: DashPayPaymentAddressKind): String? {
        if (network == null) {
            return if (networkError == null) "Reading the wallet's network…"
            else "Set is unavailable because the wallet's network could not be read."
        }
        if (supportError != null) return "Set is unavailable because the network support check failed."
        val support = support ?: return "Checking network support…"
        if (!support.supports(kind)) {
            return if (kind == DashPayPaymentAddressKind.SHIELDED) {
                "The connected network or this build does not support publishing a shielded address yet."
            } else {
                "The connected network does not support publishing a ${kind.title} address yet."
            }
        }
        return null
    }

    fun availableModes(kind: DashPayPaymentAddressKind): List<Mode> =
        if (published(kind) == null) listOf(Mode.KEEP, Mode.SET) else Mode.entries.toList()

    fun setMode(mode: Mode, kind: DashPayPaymentAddressKind) {
        if (mode == Mode.SET && setUnavailableReason(kind) != null) return
        val row = row(kind)
        rows[kind] = row.copy(mode = mode, revision = row.revision + 1)
    }

    /** The user's edit of the field. */
    fun setText(text: String, kind: DashPayPaymentAddressKind) = apply(text, kind, fromWallet = false)

    private fun apply(text: String, kind: DashPayPaymentAddressKind, fromWallet: Boolean) {
        val row = row(kind)
        rows[kind] = revalidate(
            kind,
            row.copy(text = text, filledFromWallet = fromWallet, revision = row.revision + 1),
        )
    }

    private fun revalidate(kind: DashPayPaymentAddressKind, row: Row): Row {
        if (row.text.isBlank()) return row.copy(bytes = null, error = null)
        val network = network ?: return row.copy(
            bytes = null,
            error = networkError ?: "The wallet's network is unknown.",
        )
        return try {
            row.copy(bytes = parse(kind, row.text, network), error = null)
        } catch (e: Exception) {
            row.copy(bytes = null, error = e.message ?: "Not a ${kind.title} address")
        }
    }

    private fun updateRow(kind: DashPayPaymentAddressKind, transform: (Row) -> Row) {
        rows[kind] = transform(row(kind))
    }

    // ── Use this wallet's address ──

    /**
     * Fill the Core field with this wallet's next receive address
     * (`ManagedCoreWallet.nextReceiveAddress(0)`, via [fetch]; the wallet
     * picks it). Fetched once per editor session and reused for later taps.
     * The Platform field has no such fill: no wallet operation picks a
     * Platform address for a profile yet (#6164), so it is typed or pasted.
     */
    suspend fun useCoreWalletAddress(fetch: (suspend () -> String)?) {
        val kind = DashPayPaymentAddressKind.CORE
        // One fill at a time: a second tap while the lookup runs is ignored.
        if (row(kind).isFetchingWalletAddress) return
        updateRow(kind) { it.copy(walletAddressError = null) }
        coreWalletAddress?.let {
            apply(it, kind, fromWallet = true)
            return
        }
        if (fetch == null) {
            updateRow(kind) { it.copy(walletAddressError = "No wallet available for this identity.") }
            return
        }
        // The lookup runs off the main thread: an edit made meanwhile wins,
        // and Save is held until it ends ([hasIncompleteSet]).
        val revision = row(kind).revision
        updateRow(kind) { it.copy(isFetchingWalletAddress = true) }
        try {
            val address = fetch()
            coreWalletAddress = address
            if (row(kind).revision != revision) return
            apply(address, kind, fromWallet = true)
        } catch (e: Exception) {
            if (row(kind).revision != revision) return
            updateRow(kind) {
                it.copy(walletAddressError = "Couldn't get a Core receive address: ${e.message ?: "unknown error"}")
            }
        } finally {
            updateRow(kind) { it.copy(isFetchingWalletAddress = false) }
        }
    }

    /**
     * Fill the shielded field with this identity's dedicated shielded tip
     * address, prepared now ([prepare], `prepareShieldedIdentityAddress`).
     * It is the only shielded address of this wallet a profile may publish.
     * If the user edits the field or changes its mode while the address is
     * being prepared, the result is dropped.
     */
    suspend fun useShieldedWalletAddress(
        prepare: (suspend () -> ByteArray)?,
        render: (ByteArray, Network) -> String? = { raw, net ->
            renderAddress(DashPayPaymentAddressKind.SHIELDED, raw, net)
        },
    ) {
        val kind = DashPayPaymentAddressKind.SHIELDED
        if (row(kind).isFetchingWalletAddress) return
        updateRow(kind) { it.copy(walletAddressError = null) }
        if (prepare == null) {
            updateRow(kind) { it.copy(walletAddressError = "No wallet available for this identity.") }
            return
        }
        val network = network ?: run {
            updateRow(kind) { it.copy(walletAddressError = networkError ?: "The wallet's network is unknown.") }
            return
        }
        val revision = row(kind).revision
        updateRow(kind) { it.copy(isFetchingWalletAddress = true) }
        try {
            val raw = prepare()
            if (row(kind).revision != revision) return
            val text = render(raw, network)
            if (text == null) {
                updateRow(kind) {
                    it.copy(walletAddressError = "The prepared shielded address could not be shown on ${network.displayName}.")
                }
            } else {
                apply(text, kind, fromWallet = true)
            }
        } catch (e: Exception) {
            if (row(kind).revision != revision) return
            updateRow(kind) {
                it.copy(walletAddressError = "Couldn't prepare the shielded address: ${e.message ?: "unknown error"}")
            }
        } finally {
            updateRow(kind) { it.copy(isFetchingWalletAddress = false) }
        }
    }

    // ── Submit ──

    /**
     * True when any address would change: Remove, or Set to bytes other
     * than the published ones (a fill that returns the published address
     * changes nothing).
     */
    val hasChanges: Boolean
        get() = DashPayPaymentAddressKind.entries.any { kind ->
            val row = row(kind)
            when (row.mode) {
                Mode.KEEP -> false
                Mode.REMOVE -> true
                Mode.SET -> row.bytes?.contentEquals(published(kind)) != true
            }
        }

    /**
     * True when a row is set to Set without an address platform-wallet
     * accepted, or while its wallet address is still being filled in (so
     * Save cannot submit the address the fill is about to replace).
     */
    val hasIncompleteSet: Boolean
        get() = DashPayPaymentAddressKind.entries.any {
            val row = row(it)
            row.mode == Mode.SET && (row.bytes == null || row.isFetchingWalletAddress)
        }

    /** The update for [kind]. Throws for a Set row without an accepted address. */
    fun update(kind: DashPayPaymentAddressKind): DashPayPaymentAddressUpdate {
        val row = row(kind)
        return when (row.mode) {
            Mode.KEEP -> DashPayPaymentAddressUpdate.Keep
            Mode.REMOVE -> DashPayPaymentAddressUpdate.Remove
            Mode.SET -> {
                val bytes = row.bytes
                    ?: throw IllegalStateException("Enter a valid ${kind.title} address, or choose Keep.")
                // Setting the address already published changes nothing.
                if (bytes.contentEquals(published(kind))) {
                    DashPayPaymentAddressUpdate.Keep
                } else {
                    DashPayPaymentAddressUpdate.Set(bytes)
                }
            }
        }
    }
}

// ── Editor section ────────────────────────────────────────────────────

/**
 * Editor rows for the three payment addresses: the published address, a
 * Keep / Set / Remove choice, and for Set a field checked by platform-wallet.
 * The Core and shielded fields also offer "Use this wallet's address"; the
 * Platform field is typed or pasted. Port of Swift
 * `DashPayPaymentAddressEditorSection`; testTags reuse its accessibility
 * identifiers (`dashpay.profile.paymentAddress.<kind>.current`, `.mode`,
 * `.mode.keep|set|remove`, `.setUnavailable`, `.field`, `.error`,
 * `.useWallet`, `.reuseNote`, `.useWalletError`, and
 * `dashpay.profile.paymentAddress.supportError`).
 */
@Composable
fun DashPayPaymentAddressEditorSection(
    model: DashPayPaymentAddressEditorModel,
    onUseCoreWalletAddress: () -> Unit,
    onUseShieldedWalletAddress: () -> Unit,
) {
    FormSection(title = "Payment addresses") {
        Text(PUBLIC_NOTICE, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        if (model.isLoadingSupport) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                Text(
                    "Checking which addresses the network supports…",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        (model.supportError ?: model.networkError)?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.testTag("dashpay.profile.paymentAddress.supportError"),
            )
        }
    }
    DashPayPaymentAddressKind.entries.forEach { kind ->
        FormSection(title = "${kind.title} payment address") {
            EditorRow(
                model = model,
                kind = kind,
                // Not offered for Platform: no wallet operation picks a
                // Platform address for a profile yet.
                onUseWalletAddress = when (kind) {
                    DashPayPaymentAddressKind.CORE -> onUseCoreWalletAddress
                    DashPayPaymentAddressKind.PLATFORM -> null
                    DashPayPaymentAddressKind.SHIELDED -> onUseShieldedWalletAddress
                },
            )
        }
    }
}

@Composable
private fun EditorRow(
    model: DashPayPaymentAddressEditorModel,
    kind: DashPayPaymentAddressKind,
    onUseWalletAddress: (() -> Unit)?,
) {
    val prefix = "dashpay.profile.paymentAddress.${kind.identifierName}"
    val row = model.row(kind)
    val setReason = model.setUnavailableReason(kind)

    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        PublishedAddress(model, kind, Modifier.testTag("$prefix.current"))

        Row(
            modifier = Modifier.fillMaxWidth().testTag("$prefix.mode"),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            model.availableModes(kind).forEach { mode ->
                val name = mode.name.lowercase()
                FilterChip(
                    selected = row.mode == mode,
                    onClick = { model.setMode(mode, kind) },
                    enabled = !(mode == DashPayPaymentAddressEditorModel.Mode.SET && setReason != null),
                    label = { Text(name.replaceFirstChar { it.uppercase() }) },
                    modifier = Modifier.testTag("$prefix.mode.$name"),
                )
            }
        }

        if (setReason != null && !model.isLoadingSupport) {
            Text(
                setReason,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("$prefix.setUnavailable"),
            )
        }

        when (row.mode) {
            DashPayPaymentAddressEditorModel.Mode.SET -> {
                OutlinedTextField(
                    value = row.text,
                    onValueChange = { model.setText(it, kind) },
                    modifier = Modifier.fillMaxWidth().testTag("$prefix.field"),
                    placeholder = { Text(kind.placeholder) },
                    textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.None, autoCorrectEnabled = false),
                    isError = row.error != null,
                )
                when {
                    row.error != null -> Text(
                        row.error,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("$prefix.error"),
                    )
                    row.bytes != null -> Text(
                        "Valid ${kind.title} address",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF2E7D32),
                    )
                }
                if (onUseWalletAddress != null) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        TextButton(
                            onClick = onUseWalletAddress,
                            enabled = !row.isFetchingWalletAddress,
                            modifier = Modifier.testTag("$prefix.useWallet"),
                        ) { Text("Use this wallet's address") }
                        if (row.isFetchingWalletAddress) {
                            CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                        }
                    }
                }
                if (kind == DashPayPaymentAddressKind.CORE && row.filledFromWallet) {
                    Text(
                        CORE_REUSE_NOTE,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("$prefix.reuseNote"),
                    )
                }
                if (kind == DashPayPaymentAddressKind.SHIELDED) {
                    Text(
                        "Uses this identity's dedicated shielded tip address. Other shielded addresses of this wallet cannot be published.",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                row.walletAddressError?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.testTag("$prefix.useWalletError"),
                    )
                }
            }
            DashPayPaymentAddressEditorModel.Mode.REMOVE -> Text(
                "The ${kind.title} address will be removed from your profile when you save.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
            DashPayPaymentAddressEditorModel.Mode.KEEP -> Unit
        }
    }
}

@Composable
private fun PublishedAddress(
    model: DashPayPaymentAddressEditorModel,
    kind: DashPayPaymentAddressKind,
    modifier: Modifier,
) {
    val bytes = model.published(kind)
    val network = model.network
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text("Published", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        val rendered = if (bytes != null && network != null) renderAddress(kind, bytes, network) else null
        when {
            bytes == null -> Text(
                "Not published",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            rendered != null -> SelectionContainer {
                Text(rendered, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
            }
            network == null -> Text(
                "Published (the wallet's network is unknown, so it cannot be shown)",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            else -> Text(
                "Published, but this address could not be displayed on this network",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
        }
    }
}

/** Run a blocking wallet read off the main thread. */
internal suspend fun <T> onIo(block: () -> T): T = withContext(Dispatchers.IO) { block() }
