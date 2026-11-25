package org.dashfoundation.example.ui.identity

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.dpns.DpnsNameGiveUpChoices
import org.dashfoundation.dashsdk.dpns.DpnsNameGiveUpMethod
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.di.AppContainer
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.collectEachAsStateWithLifecycle
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes

/**
 * Give up a DPNS name the identity holds: delete it (the name becomes free)
 * or tombstone it (the name is retired for good). Port of
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/GiveUpNameView.swift`;
 * the test tags are its accessibility identifiers.
 *
 * Which ways are offered comes only from
 * [org.dashfoundation.dashsdk.dpns.DpnsMarketplace.giveUpChoices]: the
 * protocol's rules (a contested name can only be tombstoned, the feature
 * needs protocol version 15) are applied in Rust, and this screen shows what
 * it reports. The name leaves the identity's list through persistence once
 * the network confirmed it gone; this screen never removes or hides rows
 * itself. The one write it makes is clearing the identity's main-name pick
 * after a confirmed give-up (even if the screen was left meanwhile), when Rust's
 * [org.dashfoundation.dashsdk.dpns.DpnsMarketplace.giveUpClearsMainName]
 * says the pick names this name, with the targeted update
 * [SelectMainNameScreen]'s pick also lands in.
 *
 * Nothing here outlives the process: after a restart the screen reads the
 * choices again, and Rust's hold on a give-up that was sent but not
 * confirmed does not survive a restart either (issue #6072), as on iOS.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GiveUpNameScreen(identityIdHex: String, name: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val identity by container.database.identityDao().observeByIdentityId(idBytes)
        .collectEachAsStateWithLifecycle(initialValue = null)
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    var phase by remember { mutableStateOf<GiveUpNamePhase>(GiveUpNamePhase.Loading) }
    var reload by remember { mutableIntStateOf(0) }
    var submitting by remember { mutableStateOf<DpnsNameGiveUpMethod?>(null) }
    var submitError by remember { mutableStateOf<String?>(null) }
    var confirming by remember { mutableStateOf<DpnsNameGiveUpMethod?>(null) }
    // Set once a give-up was confirmed or may have been delivered: from then on no read
    // may put the actions back, whatever it saw
    var giveUpSent by remember { mutableStateOf(false) }

    val walletId = identity?.walletId
    val mainDpnsName = identity?.mainDpnsName
    // Rust's rule, so the notice and the clear after success never disagree
    val isMainName = remember(mainDpnsName, manager, name) {
        runCatching { manager?.dpnsMarketplace?.giveUpClearsMainName(mainDpnsName, name) }
            .getOrNull() == true
    }

    BackHandler(enabled = submitting != null) { /* stay while a give-up is in flight */ }

    val identityLoaded = identity != null
    LaunchedEffect(manager, identityLoaded, walletId, reload) {
        val activeManager = manager
        if (!identityLoaded || giveUpSent) return@LaunchedEffect
        val wallet = walletId?.let { activeManager?.wallet(forWalletId = it) }
        if (activeManager == null || wallet == null) {
            phase = GiveUpNamePhase.Failed("This identity's wallet is not loaded.")
            return@LaunchedEffect
        }
        phase = GiveUpNamePhase.Loading
        val loaded = try {
            GiveUpNamePhase.Loaded(
                activeManager.dpnsMarketplace.giveUpChoices(wallet.handle, idBytes, name),
            )
        } catch (t: Throwable) {
            GiveUpNameUi.loadFailure(t)
        }
        // A give-up sent while this read ran wins over what the read saw
        if (!giveUpSent) phase = loaded
    }

    fun submit(method: DpnsNameGiveUpMethod) {
        val choices = (phase as? GiveUpNamePhase.Loaded)?.choices ?: return
        if (submitting != null) return
        val activeManager = manager
        val wallet = walletId?.let { activeManager?.wallet(forWalletId = it) }
        if (activeManager == null || wallet == null) {
            submitError = "This identity's wallet is not loaded."
            return
        }
        submitting = method
        submitError = null
        scope.launch {
            try {
                activeManager.dpnsMarketplace.giveUpName(
                    walletHandle = wallet.handle,
                    ownerIdentityId = idBytes,
                    name = name,
                    expectedDocumentId = choices.documentId,
                    method = method,
                    signerHandle = activeManager.signerHandle,
                )
            } catch (t: Throwable) {
                when (val failure = GiveUpNameUi.submitFailure(t)) {
                    is GiveUpNameSubmitFailure.Unconfirmed -> {
                        giveUpSent = true
                        submitError = null
                        phase = GiveUpNamePhase.Unconfirmed(failure.message)
                    }
                    is GiveUpNameSubmitFailure.Failed -> submitError = failure.message
                }
                submitting = null
                return@launch
            }
            // Platform confirmed it: from here nothing offers another give-up of the name.
            // The cleanup runs even if this screen's scope was cancelled meanwhile.
            giveUpSent = true
            val cleanupError = clearMainNamePickIfGivenUp(container, activeManager, idBytes, name)
            submitting = null
            if (cleanupError == null) {
                navController.popBackStack()
            } else {
                phase = GiveUpNamePhase.GivenUpCleanupFailed(cleanupError)
            }
        }
    }

    fun retryCleanup() {
        val activeManager = manager ?: return
        if (submitting != null) return
        scope.launch {
            val cleanupError = clearMainNamePickIfGivenUp(container, activeManager, idBytes, name)
            if (cleanupError == null) {
                navController.popBackStack()
            } else {
                phase = GiveUpNamePhase.GivenUpCleanupFailed(cleanupError)
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Give Up Name") },
                navigationIcon = {
                    IconButton(
                        onClick = { navController.popBackStack() },
                        enabled = submitting == null,
                        modifier = Modifier.testTag("giveUpName.cancel"),
                    ) {
                        Icon(Icons.Filled.Close, contentDescription = "Cancel")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Name") {
                LabeledContent("DPNS name", "$name.dash")
                LabeledContent("Identity", shortIdentifier(idBytes))
            }

            if (isMainName) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.testTag("giveUpName.mainNameNotice"),
                ) {
                    Icon(Icons.Filled.Star, contentDescription = null, modifier = Modifier.size(18.dp))
                    Text(
                        GiveUpNameUi.MAIN_NAME_NOTICE,
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.padding(start = 8.dp),
                    )
                }
            }

            when (val current = phase) {
                GiveUpNamePhase.Loading -> Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.testTag("giveUpName.loading"),
                ) {
                    CircularProgressIndicator(Modifier.size(20.dp))
                    Text(
                        "Checking how this name can be given up…",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 12.dp),
                    )
                }
                is GiveUpNamePhase.NotAvailable -> ExplainedMessage(
                    title = "Not available on this network yet",
                    explanation = GiveUpNameUi.NOT_AVAILABLE_EXPLANATION,
                    message = current.message,
                    tag = "giveUpName.notAvailable",
                )
                is GiveUpNamePhase.SdkPinned -> ExplainedMessage(
                    title = "This app needs an update",
                    explanation = GiveUpNameUi.SDK_PINNED_EXPLANATION,
                    message = current.message,
                    tag = "giveUpName.sdkPinned",
                )
                is GiveUpNamePhase.Unconfirmed -> ExplainedMessage(
                    title = "Sent, not confirmed yet",
                    explanation = GiveUpNameUi.UNCONFIRMED_EXPLANATION,
                    message = current.message,
                    tag = "giveUpName.unconfirmed",
                )
                is GiveUpNamePhase.Failed -> FormSection(title = "Could not check this name") {
                    SelectionContainer {
                        Text(current.message, modifier = Modifier.testTag("giveUpName.error"))
                    }
                    TextButton(
                        onClick = { reload++ },
                        modifier = Modifier.testTag("giveUpName.retry"),
                    ) { Text("Retry") }
                }
                is GiveUpNamePhase.GivenUpCleanupFailed -> FormSection(title = "Given up") {
                    Text(
                        GiveUpNameUi.CLEANUP_FAILED_EXPLANATION,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                    SelectionContainer {
                        Text(
                            current.message,
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.testTag("giveUpName.cleanupError"),
                        )
                    }
                    TextButton(
                        onClick = { retryCleanup() },
                        modifier = Modifier.testTag("giveUpName.retryCleanup"),
                    ) { Text("Clear Main Name Again") }
                }
                is GiveUpNamePhase.Loaded -> LoadedSections(
                    choices = current.choices,
                    submitting = submitting,
                    submitError = submitError,
                    onDelete = { confirming = DpnsNameGiveUpMethod.DELETE },
                    onTombstone = { confirming = DpnsNameGiveUpMethod.TOMBSTONE },
                )
            }
        }
    }

    when (confirming) {
        DpnsNameGiveUpMethod.DELETE -> AlertDialog(
            onDismissRequest = { confirming = null },
            title = { Text("Delete $name.dash?") },
            text = { Text(GiveUpNameUi.deleteConfirmation(name)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirming = null
                        submit(DpnsNameGiveUpMethod.DELETE)
                    },
                    modifier = Modifier.testTag("giveUpName.confirmDelete"),
                ) { Text("Delete Name", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton(onClick = { confirming = null }) { Text("Cancel") } },
        )
        DpnsNameGiveUpMethod.TOMBSTONE -> AlertDialog(
            onDismissRequest = { confirming = null },
            title = { Text("Tombstone $name.dash permanently?") },
            text = { Text(GiveUpNameUi.tombstoneConfirmation(name)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirming = null
                        submit(DpnsNameGiveUpMethod.TOMBSTONE)
                    },
                    modifier = Modifier.testTag("giveUpName.confirmTombstone"),
                ) { Text("Tombstone Permanently", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton(onClick = { confirming = null }) { Text("Cancel") } },
        )
        null -> Unit
    }
}

@Composable
private fun LoadedSections(
    choices: DpnsNameGiveUpChoices,
    submitting: DpnsNameGiveUpMethod?,
    submitError: String?,
    onDelete: () -> Unit,
    onTombstone: () -> Unit,
) {
    if (choices.canDelete) {
        FormSection(title = "Delete") {
            Text(GiveUpNameUi.DELETE_EXPLANATION, style = MaterialTheme.typography.bodyMedium)
            ActionButton(
                title = "Delete Name",
                inFlight = submitting == DpnsNameGiveUpMethod.DELETE,
                enabled = submitting == null,
                tag = "giveUpName.delete",
                onClick = onDelete,
            )
        }
    }
    if (choices.canTombstone) {
        FormSection(title = "Tombstone") {
            if (!choices.canDelete) {
                Text(
                    GiveUpNameUi.TOMBSTONE_ONLY_NOTICE,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.testTag("giveUpName.tombstoneOnlyNotice"),
                )
            }
            Text(GiveUpNameUi.TOMBSTONE_EXPLANATION, style = MaterialTheme.typography.bodyMedium)
            ActionButton(
                title = "Tombstone Name",
                inFlight = submitting == DpnsNameGiveUpMethod.TOMBSTONE,
                enabled = submitting == null,
                tag = "giveUpName.tombstone",
                onClick = onTombstone,
            )
        }
    }
    if (!choices.canDelete && !choices.canTombstone) {
        Text(
            "The network accepts no way to give up this name.",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.testTag("giveUpName.noAction"),
        )
    }
    submitError?.let { message ->
        FormSection(title = "Give-up failed") {
            SelectionContainer {
                Text(
                    message,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("giveUpName.submitError"),
                )
            }
        }
    }
    Text(
        "The app may ask you to unlock the identity signing key. The name leaves " +
            "your list only after Platform confirms it is gone.",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun ActionButton(
    title: String,
    inFlight: Boolean,
    enabled: Boolean,
    tag: String,
    onClick: () -> Unit,
) {
    OutlinedButton(
        onClick = onClick,
        enabled = enabled,
        colors = ButtonDefaults.outlinedButtonColors(contentColor = MaterialTheme.colorScheme.error),
        modifier = Modifier.fillMaxWidth().testTag(tag),
    ) {
        if (inFlight) {
            CircularProgressIndicator(Modifier.size(16.dp))
            Text(
                "Confirming…",
                modifier = Modifier.padding(start = 8.dp).testTag("giveUpName.submitting"),
            )
        } else {
            Text(title)
        }
    }
}

@Composable
private fun ExplainedMessage(title: String, explanation: String, message: String, tag: String) {
    Column(Modifier.fillMaxWidth().testTag(tag)) {
        FormSection(title = title) {
            Text(explanation, style = MaterialTheme.typography.bodyMedium)
            SelectionContainer {
                Text(
                    message,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/**
 * After a confirmed give-up, clear the identity's main-name pick when Rust's
 * rule says it names the given-up name, with the targeted update
 * [SelectMainNameScreen]'s pick also lands in. Runs [NonCancellable]: the
 * give-up already happened, so leaving the screen must not skip it. Returns
 * the failure's message, or null when the pick is cleared or stays.
 */
private suspend fun clearMainNamePickIfGivenUp(
    container: AppContainer,
    manager: PlatformWalletManager,
    identityId: ByteArray,
    name: String,
): String? = withContext(NonCancellable) {
    try {
        val pick = container.database.identityDao().getByIdentityId(identityId)?.mainDpnsName
        if (manager.dpnsMarketplace.giveUpClearsMainName(pick, name)) {
            container.database.identityDao()
                .updateMainDpnsName(identityId, null, System.currentTimeMillis())
        }
        null
    } catch (t: Throwable) {
        t.message ?: t.javaClass.simpleName
    }
}

private fun shortIdentifier(id: ByteArray): String {
    val encoded = Base58.encode(id)
    return if (encoded.length <= 12) encoded else "${encoded.take(6)}…${encoded.takeLast(6)}"
}

/** What the screen shows below the name. */
internal sealed interface GiveUpNamePhase {
    data object Loading : GiveUpNamePhase
    /** The network does not serve the feature yet; the message is the SDK's own. */
    data class NotAvailable(val message: String) : GiveUpNamePhase
    /** This app's SDK is pinned to an older protocol version; the message is the SDK's own. */
    data class SdkPinned(val message: String) : GiveUpNamePhase
    data class Failed(val message: String) : GiveUpNamePhase
    data class Loaded(val choices: DpnsNameGiveUpChoices) : GiveUpNamePhase
    /**
     * Platform confirmed the give-up, but clearing the main-name pick in Room
     * failed. Only that cleanup is offered again, never another give-up.
     */
    data class GivenUpCleanupFailed(val message: String) : GiveUpNamePhase
    /**
     * A give-up was sent and may have been delivered, but was not
     * confirmed. No further give-up is offered for this name here.
     */
    data class Unconfirmed(val message: String) : GiveUpNamePhase
}

internal sealed interface GiveUpNameSubmitFailure {
    /** The give-up may have been delivered: offer nothing more. */
    data class Unconfirmed(val message: String) : GiveUpNameSubmitFailure
    data class Failed(val message: String) : GiveUpNameSubmitFailure
}

/**
 * Wording and error routing of [GiveUpNameScreen], kept out of the
 * composable so tests can pin them. Port of `GiveUpNameUI` in
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Views/GiveUpNameView.swift`.
 * The routing reads the SDK's typed errors only, never their messages.
 */
internal object GiveUpNameUi {
    const val MAIN_NAME_NOTICE =
        "This is your main name. After you give it up, it is no longer shown as your main name."

    const val NOT_AVAILABLE_EXPLANATION =
        "Names can be deleted or tombstoned from Platform protocol version 15. " +
            "This network does not serve that yet, so this name can not be given up here."

    const val SDK_PINNED_EXPLANATION =
        "This app's SDK is fixed to an older Platform protocol version, so it can not give " +
            "names up. Update the app to use this feature."

    const val UNCONFIRMED_EXPLANATION =
        "The give-up was sent but Platform has not confirmed it, so it may still take " +
            "effect. To avoid a second paid or permanent change, nothing more is offered " +
            "here. To update which names you hold, refresh the Name Marketplace later: " +
            "its sync finds names that left. A refresh does not show that the give-up failed."

    const val DELETE_EXPLANATION =
        "Deleting makes the name free again. Anybody, including someone else, can then " +
            "register it, and anything still pointing at this name would resolve to them."

    const val TOMBSTONE_EXPLANATION =
        "Tombstoning retires the name for good. Nobody can register it again, you included, " +
            "and nobody can undo it."

    const val CLEANUP_FAILED_EXPLANATION =
        "Platform confirmed the name is given up, but the app could not clear it as your " +
            "main name. Try again, or pick another main name from the identity."

    const val TOMBSTONE_ONLY_NOTICE =
        "The network accepts only a tombstone for this name: it can not be deleted."

    fun deleteConfirmation(name: String): String =
        "$name.dash becomes free and anybody can register it, including someone else. " +
            "Anything still pointing at it would then resolve to them."

    fun tombstoneConfirmation(name: String): String =
        "This is permanent. Nobody can ever register $name.dash again, you included, " +
            "and nobody can undo a tombstone."

    fun loadFailure(error: Throwable): GiveUpNamePhase = when (error) {
        is DashSdkError.PlatformWallet.ProtocolVersionNotActive ->
            GiveUpNamePhase.NotAvailable(error.messageOrType())
        is DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion ->
            GiveUpNamePhase.SdkPinned(error.messageOrType())
        is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed ->
            GiveUpNamePhase.Unconfirmed(error.messageOrType())
        else -> GiveUpNamePhase.Failed(error.messageOrType())
    }

    fun submitFailure(error: Throwable): GiveUpNameSubmitFailure = when (error) {
        is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed ->
            GiveUpNameSubmitFailure.Unconfirmed(error.messageOrType())
        is DashSdkError.PlatformWallet.ProtocolVersionNotActive ->
            GiveUpNameSubmitFailure.Failed("Not available on this network yet. $NOT_AVAILABLE_EXPLANATION")
        is DashSdkError.PlatformWallet.SdkPinnedBelowProtocolVersion ->
            GiveUpNameSubmitFailure.Failed(SDK_PINNED_EXPLANATION)
        else -> GiveUpNameSubmitFailure.Failed(error.messageOrType())
    }

    private fun Throwable.messageOrType(): String = message ?: javaClass.simpleName
}
