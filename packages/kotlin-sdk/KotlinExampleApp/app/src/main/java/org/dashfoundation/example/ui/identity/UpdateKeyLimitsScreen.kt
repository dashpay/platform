package org.dashfoundation.example.ui.identity

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.identity.IdentityKeyLimitsUpdatePreview
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent
import org.dashfoundation.example.ui.credits.rememberManagedWalletFor
import org.dashfoundation.example.util.KeyLimitsFormatting
import org.dashfoundation.example.util.hexToBytes

/**
 * The wallet calls the Update Limits screen makes, bound to one wallet handle. Each is one
 * Rust call (`IdentityUpdates.refreshIdentity`, `previewKeyLimitsUpdate`,
 * `updateKeyLimitsAsReviewed`); the screen decides none of what they answer.
 */
interface KeyLimitsWallet {
    /** The wallet handle these calls go to, so a reloaded wallet is noticed. */
    val walletHandle: Long

    /** Fold a proved fetch of the identity into the wallet. */
    suspend fun refresh()

    /** What the change would do, or Rust's refusal (thrown). No change: can it be raised. */
    suspend fun preview(addBudget: ULong?, expiresAt: ULong?): IdentityKeyLimitsUpdatePreview

    /** Sign exactly [reviewed], or nothing. */
    suspend fun update(reviewed: IdentityKeyLimitsUpdatePreview)
}

/**
 * [KeyLimitsWallet] over [manager]'s identity updates (signed by the manager's own signer),
 * for [wallet]; a failure when [wallet] was already closed (its handle throws), so the
 * screens show the wallet as unavailable instead of crashing.
 */
fun keyLimitsWallet(
    manager: PlatformWalletManager,
    wallet: ManagedPlatformWallet,
    identityId: ByteArray,
    keyId: Int,
): Result<KeyLimitsWallet> = runCatching { wallet.handle }.map { handle ->
    keyLimitsWalletFor(manager, handle, identityId, keyId)
}

private fun keyLimitsWalletFor(
    manager: PlatformWalletManager,
    handle: Long,
    identityId: ByteArray,
    keyId: Int,
): KeyLimitsWallet {
    val id = identityId.copyOf()
    return object : KeyLimitsWallet {
        override val walletHandle: Long = handle

        override suspend fun refresh() = manager.identityUpdates.refreshIdentity(handle, id)

        override suspend fun preview(addBudget: ULong?, expiresAt: ULong?) =
            manager.identityUpdates.previewKeyLimitsUpdate(
                walletHandle = handle,
                identityId = id,
                keyId = keyId,
                addBudget = addBudget,
                expiresAt = expiresAt,
            )

        override suspend fun update(reviewed: IdentityKeyLimitsUpdatePreview) =
            manager.identityUpdates.updateKeyLimitsAsReviewed(handle, reviewed)
    }
}

/**
 * State of the Update Limits screen: port of the state in `UpdateKeyLimitsView.swift`.
 *
 * Every rule is Rust's. Opening runs a proved refresh of the identity, then a no-change
 * preview (can this key be raised at all, and by which key); every change of the form is
 * previewed again after a short pause; Update Limits is offered only while the latest
 * preview is for exactly what the form shows, and hands that preview back to Rust, which
 * signs exactly it or nothing. The only input handling here is turning the DASH text into
 * credits and the picked instant into milliseconds.
 *
 * Unlike Swift, a failed submission is never offered again from this screen: Swift's
 * `KeyLimitsRetryGate` (offer the same preview again after a consensus refusal while the
 * wallet still shows the reviewed limits) is a decision taken in the view, so it is not
 * copied here. Every failure locks the screen; closing and reopening it starts again from a
 * proved refresh. Rust classifying the outcome for both hosts is issue #6063.
 *
 * The form, the lock, whether a submission was in flight and whether one succeeded live in
 * [saved], so they survive the process being killed while the screen is in the back stack.
 * A submission in flight when the process died comes back as a locked screen whose outcome
 * is unknown; one that succeeded comes back finished (the screen closes), never as a form
 * holding the increment already applied. Nothing here is key material: ids, limits and
 * Rust's messages.
 */
class UpdateKeyLimitsViewModel(private val saved: SavedStateHandle) : ViewModel() {

    /** The answer to "can this key be raised at all". */
    sealed interface Gate {
        data object Loading : Gate
        data class Refused(val reason: String) : Gate
        data class Ready(val opening: IdentityKeyLimitsUpdatePreview) : Gate
    }

    /** The change the form describes, in protocol units. */
    data class Inputs(
        val addBudget: ULong?,
        val expiresAt: ULong?,
        /** Budget text was typed but is not an amount of DASH. */
        val budgetTextInvalid: Boolean,
        /** Extend expiry is on but the picked instant is before 1970. */
        val expiryUnrepresentable: Boolean,
    ) {
        val isInvalid: Boolean get() = budgetTextInvalid || expiryUnrepresentable
        val isEmpty: Boolean get() = addBudget == null && expiresAt == null && !isInvalid
    }

    /** The preview of the inputs the form describes. */
    sealed interface InputPreview {
        data object None : InputPreview
        data object Checking : InputPreview
        data object InvalidAmount : InputPreview
        data object InvalidExpiry : InputPreview
        data class Ready(val preview: IdentityKeyLimitsUpdatePreview, val forInputs: Inputs) : InputPreview
        data class Refused(val reason: String, val forInputs: Inputs) : InputPreview
    }

    var gate by mutableStateOf<Gate>(Gate.Loading)
        private set
    var inputPreview by mutableStateOf<InputPreview>(InputPreview.None)
        private set
    var isSubmitting by mutableStateOf(false)
        private set
    /** The update landed; the screen closes. */
    var finished by mutableStateOf(false)
        private set

    var addBudgetText by mutableStateOf(saved.get<String>(ADD_BUDGET_TEXT) ?: "")
        private set
    var extendExpiry by mutableStateOf(saved.get<Boolean>(EXTEND_EXPIRY) ?: false)
        private set
    /** The picked expiry as local epoch milliseconds; null until the gate sets a default. */
    var pickedExpiryMillis by mutableStateOf(saved.get<Long>(PICKED_EXPIRY))
        private set

    /**
     * A restored Extend expiry edit was dropped because the key's expiry, read from Platform
     * on opening, is now past what the picker can show.
     */
    var restoredExpiryDropped by mutableStateOf(false)
        private set

    /** The last submission's error text (Rust's), with the lock reason. */
    var submitError by mutableStateOf(saved.get<String>(SUBMIT_ERROR))
        private set
    var submitConsensusCode by mutableStateOf(saved.get<Int>(SUBMIT_CONSENSUS_CODE))
        private set
    /** A submission failed or its outcome is unknown: nothing more is offered here. */
    var locked by mutableStateOf(saved.get<Boolean>(LOCKED) ?: false)
        private set

    private var wallet: KeyLimitsWallet? = null
    private var refreshedHandle: Long? = null
    private var loadStarted = false
    private var previewJob: Job? = null
    /** Bumped when a submission starts: a preview started before it may not publish. */
    private var previewEpoch = 0

    /**
     * The key's current expiry is within what the date picker can show (year 9999), so
     * Extend expiry can be offered. Platform imposes no such bound.
     */
    val expiryPickable: Boolean
        get() {
            val expiresAt = (gate as? Gate.Ready)?.opening?.currentExpiresAt ?: return false
            return expiresAt < KeyLimitsFormatting.MAX_PICKABLE_MILLIS.toULong()
        }

    init {
        if (saved.get<Boolean>(SUCCEEDED) == true) {
            // Confirmed before the process died: close, never offer the increment again.
            finished = true
            locked = true
        } else if (saved.get<Boolean>(SUBMISSION_IN_FLIGHT) == true) {
            // The process died while an update was being sent: it may have landed.
            lock(
                "The app was closed while this update was being sent. It may have landed. " +
                    "Close this screen and check the key before trying again.",
                consensusCode = null,
            )
            saved[SUBMISSION_IN_FLIGHT] = false
        }
    }

    /** The form's inputs in protocol units; fields the key does not show stay empty. */
    val inputs: Inputs
        get() {
            val text = addBudgetText.trim()
            val parsed = if (text.isEmpty()) null else KeyLimitsFormatting.credits(text)
            val picked = pickedExpiryMillis
            // The same availability check as the Extend expiry control: an expiry edit the
            // screen no longer offers is never sent.
            val extending = extendExpiry && expiryPickable
            val expiry = if (extending && picked != null) KeyLimitsFormatting.millis(picked) else null
            return Inputs(
                addBudget = parsed,
                expiresAt = expiry,
                budgetTextInvalid = text.isNotEmpty() && parsed == null,
                expiryUnrepresentable = extending && expiry == null,
            )
        }

    /** The preview for exactly what the form shows, if there is one. */
    val matchingPreview: IdentityKeyLimitsUpdatePreview?
        get() = (inputPreview as? InputPreview.Ready)?.takeIf { it.forInputs == inputs }?.preview

    val canSubmit: Boolean
        get() {
            val current = inputs
            return !locked && !isSubmitting && matchingPreview != null &&
                !current.isEmpty && !current.isInvalid
        }

    /**
     * Refresh the identity from Platform with a proof, then ask whether the key can be
     * raised. Runs once per screen: a configuration change keeps this ViewModel. [resolve]
     * returns the wallet calls, or the reason there are none.
     */
    fun load(resolve: () -> Result<KeyLimitsWallet>) {
        if (loadStarted) return
        loadStarted = true
        val resolved = resolve().getOrElse {
            gate = Gate.Refused(it.message ?: "Wallet not loaded.")
            return
        }
        wallet = resolved
        viewModelScope.launch {
            try {
                resolved.refresh()
                refreshedHandle = resolved.walletHandle
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                gate = Gate.Refused(
                    "Could not read this identity from Platform: ${e.message ?: e}. " +
                        "Close this screen and try again when the network is reachable.",
                )
                return@launch
            }
            gate = try {
                val opening = resolved.preview(null, null)
                val expiresAt = opening.currentExpiresAt
                val maxPickable = KeyLimitsFormatting.MAX_PICKABLE_MILLIS
                if (expiresAt != null && expiresAt < maxPickable.toULong() && pickedExpiryMillis == null) {
                    // A starting point for the picker only, kept within what it can show;
                    // Rust judges the pick.
                    val base = maxOf(expiresAt.toLong(), System.currentTimeMillis())
                    setPickedExpiry(minOf(base, maxPickable - THIRTY_DAYS_MILLIS) + THIRTY_DAYS_MILLIS)
                }
                Gate.Ready(opening)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Gate.Refused(e.message ?: e.toString())
            }
            if (gate is Gate.Ready) {
                if (extendExpiry && !expiryPickable) {
                    extendExpiry = false
                    saved[EXTEND_EXPIRY] = false
                    restoredExpiryDropped = true
                }
                schedulePreview(debounce = false)
            }
        }
    }

    fun onAddBudgetText(text: String) {
        if (isSubmitting || locked) return
        addBudgetText = text
        saved[ADD_BUDGET_TEXT] = text
        schedulePreview()
    }

    fun onExtendExpiry(on: Boolean) {
        if (isSubmitting || locked) return
        extendExpiry = on
        saved[EXTEND_EXPIRY] = on
        schedulePreview()
    }

    fun onPickedExpiry(millis: Long) {
        if (isSubmitting || locked) return
        setPickedExpiry(millis)
        schedulePreview()
    }

    private fun setPickedExpiry(millis: Long) {
        pickedExpiryMillis = millis
        saved[PICKED_EXPIRY] = millis
    }

    /** The wallet calls, unless the wallet was reloaded after the opening refresh. */
    private fun currentWallet(): Result<KeyLimitsWallet> {
        val resolved = wallet ?: return Result.failure(IllegalStateException("Wallet not loaded."))
        val refreshed = refreshedHandle
        if (refreshed != null && refreshed != resolved.walletHandle) {
            return Result.failure(
                IllegalStateException(
                    "The wallet was reloaded after this screen read the key from Platform. " +
                        "Close this screen and open it again.",
                ),
            )
        }
        return Result.success(resolved)
    }

    /**
     * Rebind to the wallet the app holds now ([resolve]); a different handle than the one
     * the opening refresh used makes every later call refuse.
     */
    fun rebind(resolve: () -> Result<KeyLimitsWallet>) {
        val now = resolve().getOrNull() ?: return
        if (wallet?.walletHandle != now.walletHandle) wallet = now
    }

    private fun schedulePreview(debounce: Boolean = true) {
        if (gate !is Gate.Ready) return
        previewJob?.cancel()
        val current = inputs
        when {
            current.budgetTextInvalid -> { inputPreview = InputPreview.InvalidAmount; return }
            current.expiryUnrepresentable -> { inputPreview = InputPreview.InvalidExpiry; return }
            current.isEmpty -> { inputPreview = InputPreview.None; return }
        }
        inputPreview = InputPreview.Checking
        val epoch = previewEpoch
        previewJob = viewModelScope.launch {
            if (debounce) delay(PREVIEW_DEBOUNCE_MILLIS)
            val result = currentWallet().mapCatching { it.preview(current.addBudget, current.expiresAt) }
            // Only an answer for what the form shows now, asked since the last submission
            // started, may replace the current one.
            if (current != inputs || epoch != previewEpoch) return@launch
            inputPreview = result.fold(
                onSuccess = { InputPreview.Ready(it, current) },
                onFailure = {
                    if (it is CancellationException) throw it
                    InputPreview.Refused(it.message ?: it.toString(), current)
                },
            )
        }
    }

    /** Send the reviewed preview. A second tap while one is in flight is ignored. */
    fun submit() {
        if (!canSubmit) return
        val reviewed = matchingPreview ?: return
        val resolved = currentWallet().getOrElse {
            submitError = it.message
            submitConsensusCode = null
            return
        }
        isSubmitting = true
        previewEpoch++
        previewJob?.cancel()
        submitError = null
        submitConsensusCode = null
        saved[SUBMISSION_IN_FLIGHT] = true
        viewModelScope.launch {
            try {
                resolved.update(reviewed)
                // Recorded before the screen closes: a state saved in between restores as
                // finished, not as a form offering the applied increment again.
                saved[SUCCEEDED] = true
                saved[ADD_BUDGET_TEXT] = ""
                saved[EXTEND_EXPIRY] = false
                saved[SUBMISSION_IN_FLIGHT] = false
                locked = true
                isSubmitting = false
                // The wallet already stored the proved key row through the persister. No
                // second fetch: a node one block behind could answer with the old limits.
                finished = true
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                saved[SUBMISSION_IN_FLIGHT] = false
                isSubmitting = false
                val message = e.message ?: e.toString()
                val consensus = (e as? DashSdkError)?.consensusError
                val detail = when {
                    e is DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed ->
                        "The update may have landed."
                    consensus != null -> "Platform refused this update."
                    else -> "The wallet could not confirm that this update did not land."
                }
                lock("$message\n$detail Close this screen and check the key before trying again.", consensus?.code)
            }
        }
    }

    private fun lock(message: String, consensusCode: Int?) {
        locked = true
        submitError = message
        submitConsensusCode = consensusCode
        saved[LOCKED] = true
        saved[SUBMIT_ERROR] = message
        saved[SUBMIT_CONSENSUS_CODE] = consensusCode
    }

    companion object {
        const val PREVIEW_DEBOUNCE_MILLIS = 300L
        private const val THIRTY_DAYS_MILLIS = 30L * 24 * 60 * 60 * 1000
        private const val ADD_BUDGET_TEXT = "keyLimits.addBudgetText"
        private const val EXTEND_EXPIRY = "keyLimits.extendExpiry"
        private const val PICKED_EXPIRY = "keyLimits.pickedExpiry"
        private const val SUBMIT_ERROR = "keyLimits.submitError"
        private const val SUBMIT_CONSENSUS_CODE = "keyLimits.submitConsensusCode"
        private const val LOCKED = "keyLimits.locked"
        private const val SUBMISSION_IN_FLIGHT = "keyLimits.submissionInFlight"
        private const val SUCCEEDED = "keyLimits.succeeded"
    }
}

/**
 * Raise one key's protocol 14 limits: top up its budget and move its expiry later
 * (`IdentityKeyLimitsUpdate`). Port of `UpdateKeyLimitsView.swift`, reached from
 * [KeyDetailScreen] (**Limits -> Update Limits**, `keyLimits.update.open`) and from the
 * [KeysListScreen] row (`keyLimits.update.swipe.<keyId>`), with Swift's `keyLimits.update.*`
 * identifiers as test tags. State and rules: [UpdateKeyLimitsViewModel].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UpdateKeyLimitsScreen(
    identityIdHex: String,
    keyId: Int,
    navController: NavHostController,
    state: UpdateKeyLimitsViewModel = viewModel(),
) {
    val container = LocalAppContainer.current
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }
    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val wallet = rememberManagedWalletFor(identity?.walletId)

    // Rebuilt each composition; the effect below relaunches whenever its inputs change.
    val resolve: () -> Result<KeyLimitsWallet> = {
        val mgr = manager
        val w = wallet
        when {
            identity != null && identity?.walletId == null -> Result.failure(
                IllegalStateException("Identity has no wallet linkage; cannot sign a key limits update."),
            )
            mgr == null || w == null -> Result.failure(IllegalStateException("Wallet not loaded in the wallet manager."))
            else -> keyLimitsWallet(mgr, w, idBytes, keyId)
        }
    }
    // Wait for the identity row and its wallet before the one opening load.
    LaunchedEffect(identity, manager, wallet) {
        if (identity == null) return@LaunchedEffect
        if (identity?.walletId != null && (manager == null || wallet == null)) return@LaunchedEffect
        state.rebind(resolve)
        state.load(resolve)
    }
    LaunchedEffect(state.finished) {
        if (state.finished) navController.popBackStack()
    }
    // The native write cannot be cancelled once sent: leaving would drop its outcome, so
    // system Back is held while it runs, as the toolbar arrow is.
    BackHandler(enabled = state.isSubmitting) {}

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Update Limits") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }, enabled = !state.isSubmitting) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Cancel")
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
            when (val gate = state.gate) {
                UpdateKeyLimitsViewModel.Gate.Loading -> FormSection {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                        Spacer(Modifier.width(8.dp))
                        Text("Checking this key", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                is UpdateKeyLimitsViewModel.Gate.Refused -> FormSection(title = "Key #$keyId limits cannot be raised") {
                    ErrorText(gate.reason, "keyLimits.update.error")
                }
                is UpdateKeyLimitsViewModel.Gate.Ready -> Unit
            }
            // A restored lock (a submission whose outcome is unknown) is shown whatever the
            // opening refresh answers; under Ready it is part of the submit section.
            if (state.gate !is UpdateKeyLimitsViewModel.Gate.Ready && state.submitError != null) {
                FormSection { SubmitErrorText(state) }
            }
            when (val gate = state.gate) {
                UpdateKeyLimitsViewModel.Gate.Loading, is UpdateKeyLimitsViewModel.Gate.Refused -> Unit
                is UpdateKeyLimitsViewModel.Gate.Ready -> {
                    // Current limits and signer from the same preview as the new ones when
                    // there is one for what the form shows, so "now" and "after" agree.
                    CurrentSection(keyId, state.matchingPreview ?: gate.opening)
                    ChangeSection(gate.opening, state)
                    PreviewSection(state.inputPreview)
                    SubmitSection(state)
                }
            }
        }
    }
}

@Composable
private fun CurrentSection(keyId: Int, current: IdentityKeyLimitsUpdatePreview) {
    FormSection(title = "Key #$keyId now") {
        current.currentTotalBudget?.let { LabeledContent("Total budget", KeyLimitsFormatting.dash(it)) }
        current.currentExpiresAt?.let {
            LabeledContent("Expires", KeyLimitsFormatting.expiryDate(it))
            Text(
                KeyLimitsFormatting.expiryRelative(it, System.currentTimeMillis()),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        LabeledContent(
            "Signed by",
            "Key #${current.signingKeyId.toUInt()}",
            modifier = Modifier.testTag("keyLimits.update.signingKey"),
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ChangeSection(opening: IdentityKeyLimitsUpdatePreview, state: UpdateKeyLimitsViewModel) {
    // Frozen while a submission runs and once the screen is locked.
    val enabled = !state.isSubmitting && !state.locked
    var showDatePicker by remember { mutableStateOf(false) }
    var showTimePicker by remember { mutableStateOf(false) }
    FormSection(title = "Change") {
        if (opening.currentTotalBudget != null) {
            OutlinedTextField(
                value = state.addBudgetText,
                onValueChange = state::onAddBudgetText,
                label = { Text("Add to budget (DASH)") },
                singleLine = true,
                enabled = enabled,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                modifier = Modifier.fillMaxWidth().testTag("keyLimits.update.addBudget"),
            )
        }
        if (opening.currentExpiresAt != null && !state.expiryPickable) {
            Text(
                "This key's expiry is at or past the end of year 9999, the date picker's last day, so only " +
                    "its budget can be changed here." +
                    if (state.restoredExpiryDropped) " The expiry change you had started was dropped." else "",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (opening.currentExpiresAt != null && state.expiryPickable) {
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                Text("Extend expiry", modifier = Modifier.weight(1f))
                Switch(
                    checked = state.extendExpiry,
                    onCheckedChange = state::onExtendExpiry,
                    enabled = enabled,
                    modifier = Modifier.testTag("keyLimits.update.extendExpiryToggle"),
                )
            }
            val picked = state.pickedExpiryMillis
            if (state.extendExpiry && picked != null) {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.fillMaxWidth().testTag("keyLimits.update.expiryPicker"),
                ) {
                    OutlinedButton(
                        onClick = { showDatePicker = true },
                        enabled = enabled,
                        modifier = Modifier.weight(1f).testTag("keyLimits.update.expiryPicker.date"),
                    ) { Text(java.text.DateFormat.getDateInstance(java.text.DateFormat.MEDIUM).format(java.util.Date(picked))) }
                    OutlinedButton(
                        onClick = { showTimePicker = true },
                        enabled = enabled,
                        modifier = Modifier.testTag("keyLimits.update.expiryPicker.time"),
                    ) { Text(java.text.DateFormat.getTimeInstance(java.text.DateFormat.SHORT).format(java.util.Date(picked))) }
                }
            }
        }
        Text(
            "The amount is added to the total budget and to what is left of it. Limits can only be raised.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }

    val picked = state.pickedExpiryMillis
    if (showDatePicker && picked != null) {
        val pickerState = rememberDatePickerState(
            initialSelectedDateMillis = KeyLimitsFormatting.pickerDateUtcMillis(picked),
            yearRange = KeyLimitsFormatting.PICKABLE_YEARS,
        )
        DatePickerDialog(
            onDismissRequest = { showDatePicker = false },
            confirmButton = {
                TextButton(onClick = {
                    pickerState.selectedDateMillis?.let {
                        state.onPickedExpiry(KeyLimitsFormatting.withDate(picked, it))
                    }
                    showDatePicker = false
                }) { Text("OK") }
            },
            dismissButton = { TextButton(onClick = { showDatePicker = false }) { Text("Cancel") } },
        ) { DatePicker(state = pickerState) }
    }
    if (showTimePicker && picked != null) {
        val (hour, minute) = KeyLimitsFormatting.timeOfDay(picked)
        val pickerState = rememberTimePickerState(initialHour = hour, initialMinute = minute)
        AlertDialog(
            onDismissRequest = { showTimePicker = false },
            confirmButton = {
                TextButton(onClick = {
                    state.onPickedExpiry(KeyLimitsFormatting.withTime(picked, pickerState.hour, pickerState.minute))
                    showTimePicker = false
                }) { Text("OK") }
            },
            dismissButton = { TextButton(onClick = { showTimePicker = false }) { Text("Cancel") } },
            text = { TimePicker(state = pickerState) },
        )
    }
}

@Composable
private fun PreviewSection(preview: UpdateKeyLimitsViewModel.InputPreview) {
    when (preview) {
        UpdateKeyLimitsViewModel.InputPreview.None -> Unit
        UpdateKeyLimitsViewModel.InputPreview.Checking -> FormSection(title = "After the update") {
            Row(verticalAlignment = Alignment.CenterVertically) {
                CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                Spacer(Modifier.width(8.dp))
                Text("Checking", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        UpdateKeyLimitsViewModel.InputPreview.InvalidAmount -> FormSection(title = "After the update") {
            ErrorText("Enter an amount of DASH, with at most 11 decimals.", "keyLimits.update.error")
        }
        UpdateKeyLimitsViewModel.InputPreview.InvalidExpiry -> FormSection(title = "After the update") {
            ErrorText("Pick an expiry date after 1970.", "keyLimits.update.error")
        }
        is UpdateKeyLimitsViewModel.InputPreview.Refused -> FormSection(title = "After the update") {
            ErrorText(preview.reason, "keyLimits.update.error")
        }
        is UpdateKeyLimitsViewModel.InputPreview.Ready -> FormSection(title = "After the update") {
            Column(
                verticalArrangement = Arrangement.spacedBy(6.dp),
                modifier = Modifier.testTag("keyLimits.update.preview"),
            ) {
                preview.preview.newTotalBudget?.let { Text("New total budget ${KeyLimitsFormatting.dash(it)}") }
                preview.preview.newExpiresAt?.let { Text("New expiry ${KeyLimitsFormatting.expiryDate(it)}") }
                Text(
                    "Signed by key #${preview.preview.signingKeyId.toUInt()}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun SubmitSection(state: UpdateKeyLimitsViewModel) {
    FormSection {
        Button(
            onClick = state::submit,
            enabled = state.canSubmit,
            modifier = Modifier.fillMaxWidth().testTag("keyLimits.update.submit"),
        ) {
            if (state.isSubmitting) {
                CircularProgressIndicator(
                    modifier = Modifier.size(16.dp),
                    strokeWidth = 2.dp,
                    color = MaterialTheme.colorScheme.onPrimary,
                )
                Spacer(Modifier.width(8.dp))
            }
            Text("Update Limits")
        }
        SubmitErrorText(state)
    }
}

@Composable
private fun SubmitErrorText(state: UpdateKeyLimitsViewModel) {
    val message = state.submitError ?: return
    Column(
        verticalArrangement = Arrangement.spacedBy(4.dp),
        modifier = Modifier.testTag("keyLimits.update.submitError"),
    ) {
        Text(message, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.error)
        state.submitConsensusCode?.let {
            Text(
                "Consensus error $it",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun ErrorText(text: String, tag: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.error,
        modifier = Modifier.testTag(tag),
    )
}
