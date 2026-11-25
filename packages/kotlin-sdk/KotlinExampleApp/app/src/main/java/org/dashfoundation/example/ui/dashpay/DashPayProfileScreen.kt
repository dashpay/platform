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
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressKind
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressUpdate
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.components.collectEachAsStateWithLifecycle
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.generateQrBitmap
import org.dashfoundation.example.util.hexToBytes

/**
 * Own DashPay profile — port of `DashPayProfileView.swift` plus its companion
 * editor: read-only display (avatar / name / DPNS / public message / id), the
 * DIP-15 auto-accept QR (via `buildAutoAcceptQr`, rendered with the ZXing
 * `generateQrBitmap` helper), and an inline edit mode calling
 * `createOrUpdateProfile` (doCreate when no profile exists yet).
 *
 * The profile's public payment addresses show read-only below the header
 * and are edited in the edit mode's Payment addresses section, ported from
 * `DashPayPaymentAddressesSection.swift` and the `DashPayProfileEditorView`
 * wiring in `IdentityDetailView.swift`. Each address is Keep by default, so
 * an edit that does not touch them leaves them as published.
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
    }.collectEachAsStateWithLifecycle(initialValue = null)
    val walletId = identity?.walletId
    val wallet = remember(manager, walletId) { walletId?.let { manager?.wallet(forWalletId = it) } }

    // The profile as the wallet holds it in memory. Only a wallet read is
    // trusted to say whether a profile exists: [profileChecked] is true once
    // it holds the profile, or once a DashPay sync pass has completed with
    // none. The screen does not sync profiles itself: a second sync racing
    // the app's DashPay sync could store an older read over a newer one.
    var profile by remember { mutableStateOf<DashPayProfile?>(null) }
    var profileExists by remember { mutableStateOf(false) }
    var profileChecked by remember { mutableStateOf(false) }
    var isCheckingProfile by remember { mutableStateOf(false) }
    // The profile Edit started from: an update sends only the text fields
    // the user changed from it, so it cannot write back stale text.
    var editSnapshot by remember { mutableStateOf<DashPayProfile?>(null) }
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
    // A fresh editor model per edit session, over the profile as it was
    // when Edit was tapped (Swift's `DashPayPaymentAddressEditorModel`).
    var addressEditor by remember { mutableStateOf<DashPayPaymentAddressEditorModel?>(null) }

    LaunchedEffect(addressEditor) {
        val model = addressEditor ?: return@LaunchedEffect
        val w = wallet
        model.start(
            network = manager?.network?.takeIf { w != null },
            loadSupport = w?.let { { it.dashpay.paymentAddressSupport() } },
        )
    }

    // The persisted own profile on this identity's network. Rust does not
    // restore an own profile at launch, so its in-memory copy is empty until
    // a profile sync. This row is shown as the last known profile only until
    // the wallet has checked: it is display only, never a reason to update
    // rather than create, since a profile deleted on Platform leaves it here.
    val persistedOwnRow by remember(idBytes) {
        container.database.dashpayDao().observeProfileByIdentity(idBytes)
    }.collectAsStateWithLifecycle(initialValue = null)
    val persistedOwnProfile = persistedOwnRow?.takeIf { it.networkRaw == identity?.networkRaw }

    // The app's DashPay sync (`PlatformWalletManager.startDashPaySync`), one
    // pass at a time, fetches the own profiles. Reading the wallet again
    // when a pass ends lets the screen follow it.
    val dashPaySyncing by remember(manager) {
        manager?.dashPaySyncIsSyncing ?: MutableStateFlow(false)
    }.collectAsStateWithLifecycle(initialValue = false)

    // A miss counts as "no profile" only after a whole sync pass that
    // started after this screen opened ([OwnProfileChecker]). The wallet
    // cannot yet say whether that pass's fetch failed (#6187).
    val checker = remember(manager, wallet) {
        val w = wallet
        val m = manager
        if (w == null || m == null) {
            null
        } else {
            OwnProfileChecker(
                readProfile = { w.dashpay.getProfile(idBytes) },
                // A failed read is unknown, never a recorded "no pass yet".
                readLastSync = { runCatching { m.dashPayLastSyncUnixSeconds() }.getOrNull() },
            )
        }
    }

    fun applyAnswer(answer: OwnProfileChecker.Answer) {
        profileChecked = answer.checked
        isCheckingProfile = !answer.checked
        profileExists = answer.raw != null
        profile = parseDashPayProfile(answer.raw)
    }

    /**
     * Read the profile from the wallet. A miss counts as "no profile" only
     * after a sync pass that started after the screen opened has completed.
     */
    suspend fun loadProfile() {
        val c = checker ?: run {
            // No wallet on the selected network holds this identity (for
            // example after a network switch): nothing is checked, so Edit
            // stays off and the wallet's earlier answer is not reused.
            profileChecked = false
            profileExists = false
            profile = null
            return
        }
        applyAnswer(c.read())
    }

    LaunchedEffect(wallet, dashPaySyncing, persistedOwnProfile?.lastUpdated?.time) {
        if (!isEditing) loadProfile()
    }

    // Until the wallet has answered, read again every two seconds: a pass
    // that finds no profile writes nothing to Room and can run between the
    // one-second samples of the sync status, so neither key above changes.
    LaunchedEffect(checker) {
        checker?.pollUntilChecked { answer -> if (!isEditing) applyAnswer(answer) }
    }

    // What the screen shows: the wallet's profile, or the last known one
    // until the wallet has checked.
    val shownProfile = profile ?: persistedOwnProfile?.takeIf { !profileChecked }?.toDashPayProfile()

    LaunchedEffect(wallet) {
        val w = wallet ?: return@LaunchedEffect
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

    val displayName = shownProfile?.displayName?.trim()?.takeIf { it.isNotEmpty() }
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
                                val start = if (profileExists) profile else null
                                editSnapshot = start
                                displayNameField = start?.displayName.orEmpty()
                                publicMessageField = start?.publicMessage.orEmpty()
                                avatarUrlField = start?.avatarUrl.orEmpty()
                                saveError = null
                                addressEditor = DashPayPaymentAddressEditorModel(existing = start)
                            } else {
                                addressEditor = null
                                editSnapshot = null
                            }
                            isEditing = !isEditing
                        },
                        // Editing waits for the wallet's check: until then
                        // it is unknown whether Save creates or updates.
                        enabled = isEditing || profileChecked,
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
                }
                addressEditor?.let { model ->
                    DashPayPaymentAddressEditorSection(
                        model = model,
                        onUseCoreWalletAddress = {
                            scope.launch {
                                model.useCoreWalletAddress(
                                    wallet?.let { w ->
                                        { onIo { w.coreWallet().use { core -> core.nextReceiveAddress(0) } } }
                                    },
                                )
                            }
                        },
                        onUseShieldedWalletAddress = {
                            val m = manager
                            val wid = walletId
                            scope.launch {
                                model.useShieldedWalletAddress(
                                    if (m != null && wid != null) {
                                        { m.prepareShieldedIdentityAddress(wid, idBytes) }
                                    } else {
                                        null
                                    },
                                )
                            }
                        },
                    )
                }
                FormSection {
                    saveError?.let {
                        Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                    }
                    // An update with nothing changed would still be a paid
                    // document replacement, so Save needs a change.
                    val hasChanges = !profileExists ||
                        profileTextFieldUpdate(displayNameField, editSnapshot, editSnapshot?.displayName) != null ||
                        profileTextFieldUpdate(publicMessageField, editSnapshot, editSnapshot?.publicMessage) != null ||
                        profileTextFieldUpdate(avatarUrlField, editSnapshot, editSnapshot?.avatarUrl) != null ||
                        addressEditor?.hasChanges == true
                    SubmitButton(
                        text = if (profileExists) "Save" else "Create",
                        isLoading = isSaving,
                        enabled = !isSaving && hasChanges && addressEditor?.hasIncompleteSet != true,
                        modifier = Modifier.fillMaxWidth().testTag("dashpay.profile.save"),
                    ) {
                        val m = manager ?: return@SubmitButton
                        val w = wallet ?: return@SubmitButton
                        val model = addressEditor
                        // Recheck at submit: a wallet fill may have started
                        // since the button was last enabled.
                        if (model?.hasIncompleteSet == true) return@SubmitButton
                        val updates = try {
                            DashPayPaymentAddressKind.entries.associateWith {
                                model?.update(it) ?: DashPayPaymentAddressUpdate.Keep
                            }
                        } catch (e: IllegalStateException) {
                            saveError = e.message
                            return@SubmitButton
                        }
                        // A create sends every filled field. An update sends a
                        // text field only when the user changed it from the
                        // profile Edit started from (null keeps the field as
                        // published), so an address-only edit cannot write
                        // back text another device changed since.
                        val snapshot = editSnapshot
                        val displayNameUpdate = profileTextFieldUpdate(displayNameField, snapshot, snapshot?.displayName)
                        val publicMessageUpdate = profileTextFieldUpdate(publicMessageField, snapshot, snapshot?.publicMessage)
                        val avatarUrlUpdate = profileTextFieldUpdate(avatarUrlField, snapshot, snapshot?.avatarUrl)
                        isSaving = true
                        saveError = null
                        scope.launch {
                            try {
                                // Before a create, ask the wallet again: a sync
                                // pass may have found the profile while the form
                                // was open, and a second create is rejected.
                                if (!profileExists) {
                                    val found = parseDashPayProfile(w.dashpay.getProfile(idBytes))
                                    if (found != null) {
                                        profile = found
                                        profileExists = true
                                        editSnapshot = found
                                        displayNameField = found.displayName.orEmpty()
                                        publicMessageField = found.publicMessage.orEmpty()
                                        avatarUrlField = found.avatarUrl.orEmpty()
                                        addressEditor = DashPayPaymentAddressEditorModel(existing = found)
                                        // The draft is not carried over: Save stays
                                        // off until something is changed again,
                                        // so a second tap cannot pay for an
                                        // update that changes nothing.
                                        saveError = "Your profile was found on Platform while you were editing, " +
                                            "so your changes were not saved. Make them again and save."
                                        return@launch
                                    }
                                }
                                w.dashpay.createOrUpdateProfile(
                                    identityId = idBytes,
                                    displayName = displayNameUpdate,
                                    publicMessage = publicMessageUpdate,
                                    avatarUrl = avatarUrlUpdate,
                                    doCreate = !profileExists,
                                    signerHandle = m.signerHandle,
                                    corePaymentAddress = updates.getValue(DashPayPaymentAddressKind.CORE),
                                    platformPaymentAddress = updates.getValue(DashPayPaymentAddressKind.PLATFORM),
                                    shieldedAddress = updates.getValue(DashPayPaymentAddressKind.SHIELDED),
                                )
                                loadProfile()
                                addressEditor = null
                                editSnapshot = null
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
                        DashPayAvatar(shownProfile?.avatarUrl, displayName, size = 96.dp)
                        Text(displayName, style = MaterialTheme.typography.titleLarge)
                        (identity?.mainDpnsName ?: identity?.dpnsName)?.let {
                            Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary)
                        }
                        shownProfile?.publicMessage?.trim()?.takeIf { it.isNotEmpty() }?.let {
                            Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
                        }
                        when {
                            isCheckingProfile && wallet != null -> Text(
                                "Waiting for the DashPay sync to read your profile…",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.testTag("dashpay.profile.checking"),
                            )
                            !profileChecked && shownProfile != null -> Text(
                                "Last known profile; it may have changed since.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
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

            if (!isEditing) {
                // Rendered for the identity's own network, never the
                // selected one: a route restored after a network switch
                // still names this identity, and the stored Core and
                // Platform bytes carry no network.
                DashPayPublishedPaymentAddressesSection(
                    profile = shownProfile,
                    network = identity?.networkRaw?.let { raw -> Network.entries.firstOrNull { it.ffiValue == raw } },
                    identifierPrefix = "dashpay.profile.paymentAddress",
                    showsUnpublished = true,
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

/**
 * The value a profile write sends for one text field. A create ([snapshot]
 * null) sends the typed text, or null when empty. An update sends the typed
 * text only when it differs from [before], the field in the profile Edit
 * started from, and null otherwise, which keeps the published value: an
 * update that left the field alone cannot write back text another device
 * changed since. An emptied field also sends null, as before.
 */
internal fun profileTextFieldUpdate(value: String, snapshot: DashPayProfile?, before: String?): String? {
    val typed = value.trim().ifEmpty { null }
    if (snapshot == null) return typed
    return if (typed != before?.trim()?.ifEmpty { null }) typed else null
}

/**
 * Reads Your Profile's answer from the wallet: the cached own profile, and
 * whether a miss there may count as "no profile" yet ([OwnProfileCheckGate]).
 * The readers are injected so the polling is testable without JNI.
 */
internal class OwnProfileChecker(
    private val readProfile: suspend () -> String?,
    private val readLastSync: suspend () -> Long?,
) {
    /** The cached profile JSON, and whether the screen may act on it (a hit, or a miss after a pass). */
    data class Answer(val raw: String?, val checked: Boolean)

    private val gate = OwnProfileCheckGate()

    suspend fun read(): Answer {
        val raw = readProfile()
        val passed = gate.observe(readLastSync())
        return Answer(raw, raw != null || passed)
    }

    /** Read now and every [intervalMs] until an answer is checked, handing each to [onAnswer]. */
    suspend fun pollUntilChecked(intervalMs: Long = 2_000, onAnswer: (Answer) -> Unit) {
        while (true) {
            val answer = read()
            onAnswer(answer)
            if (answer.checked) return
            delay(intervalMs)
        }
    }
}

/**
 * Decides when a miss in the wallet's own-profile cache can count as "no
 * profile": only after a whole DashPay sync pass that started after the
 * screen opened, since a pass queries only the identities it collected when
 * it started (an identity recovered later is not in it).
 *
 * Fed the manager's last completed pass time on every read. The first value
 * is the baseline. Each later change to a new nonzero value is one more pass
 * completed: the first may be the pass that was already running at open, so
 * the second is the first pass that surely started after it. Values are
 * compared for change, not order, so a wall-clock correction still counts,
 * and zero (no pass recorded) is never a completion. A null value is a read
 * that failed: it is ignored, and never becomes the baseline. Once passed
 * it stays passed. To be replaced by the identity-scoped result in #6187.
 */
internal class OwnProfileCheckGate {
    private var lastSeen: Long? = null
    private var changes = 0

    /** Record [lastSyncUnixSeconds]; true once a pass started after the first read has completed. */
    fun observe(lastSyncUnixSeconds: Long?): Boolean {
        if (lastSyncUnixSeconds == null) return changes >= 2
        val seen = lastSeen
        when {
            seen == null -> lastSeen = lastSyncUnixSeconds
            lastSyncUnixSeconds != 0L && lastSyncUnixSeconds != seen -> {
                lastSeen = lastSyncUnixSeconds
                changes++
            }
        }
        return changes >= 2
    }
}
