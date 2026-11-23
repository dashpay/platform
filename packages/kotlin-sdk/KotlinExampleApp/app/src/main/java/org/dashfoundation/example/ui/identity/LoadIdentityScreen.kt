package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.dao.IdentityDao
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.VotingKeyAssignment
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyAddress
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyCheck
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyCheckOutcome
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyRecords
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyRules
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeySlot
import org.dashfoundation.dashsdk.voting.MasternodeVotingLoad
import org.dashfoundation.dashsdk.voting.recordedVotingKey
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.IdentityDetail
import org.dashfoundation.example.navigation.QrScanner
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.ui.voting.wireOrderProTxHash
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex

/**
 * Load an existing identity by id: port of `LoadIdentityView.swift`. Paste or scan an
 * identity id, pick its type, fetch it via `sdk.identities.fetch`, and persist a Room
 * [IdentityEntity] with `isLocal = true`: a manual add is an identity the owner deliberately
 * tracks, which is what the flag means under the owner semantics. A row already on the
 * device is merged, never replaced: only the balance, the alias (when one is typed), the
 * type, `isLocal` and `lastUpdated` change, in one targeted update of the current row, so a
 * wallet link, key identifiers and DPNS names survive a reload, as do writes made while the
 * fetch was in flight. A row stored for another network is not touched: the load is refused.
 * The type picker starts from the stored row's type once the typed id names one, and a type
 * the user did not pick never overwrites the stored one.
 *
 * A masternode or evonode identity can bring its voting private key (hex or WIF, read by
 * Rust through [MasternodeVotingKeyStore.privateKeyBytesFromText]). The key is read before
 * the fetch, so a refused key stops the load with Rust's reason and nothing is written. Once
 * the identity is saved, the key is stored in the identity's own `WalletStorage` entry, the
 * identifier the row records, and the vote dialogs sign with it through the wallet manager's
 * signer (Swift `storeSpecialKeyIfAbsent`, see [decideVotingKeyOutcome]). Entering the key
 * the row already names writes the entry again from the entered bytes, which repairs an
 * unreadable one. A different key already stored for the identity is kept until the user
 * confirms replacing it, and the replacement happens only while the row still names the key
 * the question showed. Owner and payout keys and identity private keys are not imported:
 * nothing in this app signs with them.
 *
 * A masternode or evonode is loaded through Rust's rule (`Voting.loadMasternodeForVoting`,
 * #6099): its owner identity, or, for a masternode without one (a shared masternode), its voter
 * identity when Platform proves that one holds the voting key. The row is stored under the
 * loaded identity's id with the wire-order pro_tx_hash kept beside it
 * (`IdentityEntity.masternodeProTxHash`), which votes name.
 *
 * Before anything is saved, an entered voting key is checked against Platform with that load
 * (the voter identity's proof-verified fetch), as `LoadIdentityView.swift` does since #6086. Rust decides from the check and what
 * the row's voting key slot holds whether the key is stored and what is recorded for it
 * ([checkEnteredVotingKey], `MasternodeVotingKeyRules.outcome`); a refused key stops the load
 * with Rust's reason, and a refusal of the key already stored is recorded on its row. Once the
 * key is stored, what Rust says to record is written with the address of the key it was made
 * of (`MasternodeVotingKeyRecords.record`). Nothing is recorded before the key is in its slot,
 * so a load that dies in between leaves the key unchecked, and it is checked when picked.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LoadIdentityScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    var identityIdText by remember { mutableStateOf("") }
    var alias by remember { mutableStateOf("") }
    var identityType by remember { mutableStateOf(LoadIdentityType.USER) }
    // Whether the user picked the type: an unpicked type never overwrites a stored row's.
    var identityTypePicked by remember { mutableStateOf(false) }
    // The typed text, never the decoded bytes: a confirmed replacement reads it again.
    var votingKeyText by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var pendingReplacement by remember { mutableStateOf<PendingVotingKeyReplacement?>(null) }
    // Shown after a replacement whose old key could not be removed yet; the identity is
    // opened once the message is dismissed.
    var showAfterError by remember { mutableStateOf<ByteArray?>(null) }

    // QR scanner hand-back.
    val savedStateHandle = navController.currentBackStackEntry?.savedStateHandle
    LaunchedEffect(savedStateHandle) {
        savedStateHandle
            ?.getStateFlow<String?>(QrScanner.RESULT_KEY, null)
            ?.collect { raw ->
                if (raw != null) {
                    identityIdText = raw.trim()
                    savedStateHandle[QrScanner.RESULT_KEY] = null
                }
            }
    }

    // The voting key draft belongs to the identity (decoded id) and network it was typed
    // for: when either changes, by a manual edit or a QR scan, the draft is cleared, so one
    // masternode's key is never saved under another identity. Re-encoding the same id (hex
    // or Base58) keeps it.
    val draftScope = votingKeyDraftScope(identityIdText, network.ffiValue)
    var lastDraftScope by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(draftScope) {
        if (shouldClearVotingKeyDraft(lastDraftScope, draftScope)) votingKeyText = ""
        if (draftScope != null) lastDraftScope = draftScope
    }

    // Until the user picks a type, the picker follows the typed id: the stored row's type
    // when the id names a row on this network, so a reload shows (and keeps) it and its
    // voting key field, else User, so another identity's type never carries over.
    LaunchedEffect(identityIdText, network) {
        if (identityTypePicked) return@LaunchedEffect
        val row = Base58.decodeIdentifier(identityIdText.trim())
            ?.let { container.database.identityDao().getByIdentityId(it) }
        // The lookup suspended: a type the user picked meanwhile wins.
        if (!identityTypePicked) identityType = inferredLoadIdentityType(row, network.ffiValue)
    }

    fun showIdentity(idBytes: ByteArray) {
        navController.navigate(IdentityDetail(idBytes.toHex())) {
            popUpTo(org.dashfoundation.example.navigation.LoadIdentity) { inclusive = true }
        }
    }

    // The voting key field is a secret: block screenshots while it can be shown.
    if (identityType.isMasternode) HoldSecureWindow()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Load Identity") },
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
            FormSection(title = "Identity") {
                OutlinedTextField(
                    value = identityIdText,
                    onValueChange = { identityIdText = it },
                    label = { Text("Identity ID / ProTxHash (hex or Base58)") },
                    singleLine = true,
                    enabled = !isLoading,
                    modifier = Modifier.fillMaxWidth().testTag("loadIdentity.idField"),
                )
                Row(
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                    horizontalArrangement = Arrangement.End,
                ) {
                    OutlinedButton(
                        onClick = { navController.navigate(QrScanner) },
                        enabled = !isLoading,
                        modifier = Modifier.testTag("loadIdentity.scan"),
                    ) {
                        Icon(Icons.Default.QrCodeScanner, contentDescription = null)
                        Text("Scan")
                    }
                }
                AccessiblePicker(
                    label = "Identity Type",
                    options = LoadIdentityType.entries,
                    selected = identityType,
                    optionLabel = { it.rawValue },
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                    testTag = "loadIdentity.identityType",
                    enabled = !isLoading,
                    onSelected = {
                        identityType = it
                        identityTypePicked = true
                        // A key typed for another type is not stored.
                        if (!it.isMasternode) votingKeyText = ""
                    },
                )
                OutlinedTextField(
                    value = alias,
                    onValueChange = { alias = it },
                    label = { Text("Alias (optional)") },
                    singleLine = true,
                    enabled = !isLoading,
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                )
            }

            if (identityType.isMasternode) {
                FormSection(title = "Masternode Keys") {
                    OutlinedTextField(
                        value = votingKeyText,
                        onValueChange = { votingKeyText = it },
                        label = { Text("Voting private key (hex or WIF, optional)") },
                        singleLine = true,
                        enabled = !isLoading,
                        // KeyboardType.Password only configures the keyboard; the
                        // transformation masks what the field shows.
                        visualTransformation = PasswordVisualTransformation(),
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                        modifier = Modifier.fillMaxWidth().testTag("loadIdentity.votingKey"),
                    )
                    Text(
                        "Stored on this device so the vote screens can sign this identity's " +
                            "masternode votes. Without it the identity cannot vote here.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = 4.dp),
                    )
                }
            }

            SubmitButton(
                text = "Load Identity",
                isLoading = isLoading,
                enabled = identityIdText.isNotBlank() && !isLoading,
                modifier = Modifier.fillMaxWidth().testTag("loadIdentity.submit"),
            ) {
                val activeSdk = sdk ?: run {
                    error = "SDK not initialized"
                    return@SubmitButton
                }
                val activeNetwork = network
                val id = identityIdText.trim()
                val type = identityType
                val typePicked = identityTypePicked
                val keyText = if (type.isMasternode) votingKeyText.trim() else ""
                val trimmedAlias = alias.trim().ifBlank { null }
                isLoading = true
                scope.launch {
                    val keys = MasternodeVotingKeyStore(container.walletStorage)
                    var votingKey: ByteArray? = null
                    try {
                        // Decode the input id (64-char hex or Base58) to raw 32 bytes for
                        // the primary key. `Base58.decodeIdentifier` accepts either encoding
                        // and enforces the 32-byte width.
                        val idBytes = Base58.decodeIdentifier(id) ?: run {
                            error = "Could not decode the identity id. Enter a 64-character " +
                                "hex or Base58 identity id."
                            return@launch
                        }

                        // Read the key before anything is fetched or written: a typed key
                        // Rust refuses stops the load with its reason, never the text.
                        if (keyText.isNotEmpty()) {
                            votingKey = try {
                                keys.privateKeyBytesFromText(keyText)
                            } catch (e: CancellationException) {
                                throw e
                            } catch (e: Exception) {
                                error = votingKeyRefusedMessage(e.message)
                                return@launch
                            }
                        }

                        // A masternode or evonode is loaded through Rust's rule
                        // (`Voting.loadMasternodeForVoting`): its owner identity, whose id is
                        // the pro_tx_hash, or, for a masternode without an owner identity (a
                        // shared masternode, #6099), its voter identity when Platform proves
                        // that one holds the voting key; the voting key is checked with the
                        // load. The row is stored under the loaded identity's id, with the
                        // pro_tx_hash kept beside it. Any other identity is fetched at the
                        // typed id.
                        val rowId: ByteArray
                        val json: String
                        var masternodeProTxHash: ByteArray? = null
                        var loadedVotingKeyCheck: MasternodeVotingKeyCheck? = null
                        if (type.isMasternode) {
                            val proTxHash = wireOrderProTxHash(idBytes)
                            val address = votingKey?.let {
                                MasternodeVotingKeyAddress.of(keys.votingAddress(it))
                            }
                            when (val load = activeSdk.voting.loadMasternodeForVoting(proTxHash, address)) {
                                is MasternodeVotingLoad.Refused -> {
                                    // A voting key refused for a voter identity is recorded on
                                    // a row this device already stores for it holding the same
                                    // key, as Rust's outcome says; nothing is created or
                                    // replaced (Swift `applyVotingKeyCheck`).
                                    val refusedFor = load.votingKeyRefusedFor
                                    val key = votingKey
                                    error = if (refusedFor != null && key != null &&
                                        appState.sdk.value === activeSdk
                                    ) {
                                        val applied = checkEnteredVotingKey(
                                            dao = container.database.identityDao(),
                                            entries = StoredVotingKeyEntries(keys, container.database),
                                            rules = MasternodeVotingKeyRules.Native,
                                            identityId = refusedFor,
                                            networkRaw = activeNetwork.ffiValue,
                                            enteredIdentifier = MasternodeVotingKeyStore
                                                .identifierFor(refusedFor, keys.votingAddress(key)),
                                            check = MasternodeVotingKeyCheck.Invalid(load.message),
                                            refusalText = load.message,
                                        )
                                        (applied as? EnteredVotingKeyCheck.Refused)?.message ?: load.message
                                    } else {
                                        load.message
                                    }
                                    return@launch
                                }
                                is MasternodeVotingLoad.Loaded -> {
                                    rowId = load.identity.identityId
                                    json = load.identity.identityJson
                                    masternodeProTxHash = proTxHash
                                    loadedVotingKeyCheck = load.identity.votingKeyCheck
                                }
                            }
                        } else {
                            rowId = idBytes
                            json = activeSdk.identities.fetch(id) ?: run {
                                error = "Identity not found on ${activeNetwork.displayName}"
                                return@launch
                            }
                        }
                        // The network was switched while the fetch was in flight: the
                        // answer is from the previous network, so nothing is written.
                        if (appState.sdk.value !== activeSdk) {
                            error = "The network changed during the load, so the identity " +
                                "was not saved. Load it again."
                            return@launch
                        }
                        val balance = runCatching {
                            Json.parseToJsonElement(json).jsonObject["balance"]
                                ?.jsonPrimitive?.longOrNull ?: 0L
                        }.getOrDefault(0L)

                        val entries = StoredVotingKeyEntries(keys, container.database)
                        // The voting key was checked against Platform with the load; Rust
                        // decides from the check and the slot whether it is stored.
                        var votingKeyCheck: MasternodeVotingKeyCheck? = null
                        val key = votingKey
                        val check = loadedVotingKeyCheck
                        if (key != null && check != null) {
                            val entered = entries.identifierFor(rowId, key)
                            val preflight = checkEnteredVotingKey(
                                dao = container.database.identityDao(),
                                entries = entries,
                                rules = MasternodeVotingKeyRules.Native,
                                identityId = rowId,
                                networkRaw = activeNetwork.ffiValue,
                                enteredIdentifier = entered,
                                check = check,
                            )
                            if (preflight is EnteredVotingKeyCheck.Refused) {
                                error = preflight.message
                                return@launch
                            }
                            votingKeyCheck = check
                        }

                        val result = saveLoadedIdentity(
                            dao = container.database.identityDao(),
                            entries = entries,
                            identityId = rowId,
                            networkRaw = activeNetwork.ffiValue,
                            balance = balance,
                            alias = trimmedAlias,
                            pickedType = type,
                            typePicked = typePicked,
                            votingKey = votingKey,
                            votingKeyCheck = votingKeyCheck,
                            masternodeProTxHash = masternodeProTxHash,
                        )
                        when (result) {
                            LoadSaveResult.Saved -> showIdentity(rowId)
                            is LoadSaveResult.Failed -> error = result.message
                            is LoadSaveResult.SavedWithWarning -> {
                                showAfterError = rowId
                                error = result.message
                            }
                            is LoadSaveResult.ConfirmReplacement ->
                                pendingReplacement = PendingVotingKeyReplacement(
                                    identityId = rowId,
                                    networkRaw = activeNetwork.ffiValue,
                                    storedIdentifier = result.storedIdentifier,
                                    votingKeyCheck = votingKeyCheck,
                                )
                        }
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        error = e.message ?: "Failed to load identity"
                    } finally {
                        votingKey?.fill(0)
                        isLoading = false
                    }
                }
            }

            Text(
                "Loaded identities are read-only: no identity keys are imported. For a " +
                    "masternode or evonode, the voting private key you enter is stored on " +
                    "this device so the identity can cast masternode votes; owner and payout " +
                    "keys are not imported.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }

    pendingReplacement?.let { pending ->
        AlertDialog(
            onDismissRequest = {},
            properties = DialogProperties(dismissOnBackPress = false, dismissOnClickOutside = false),
            title = { Text("Replace the stored key?") },
            text = {
                Text(
                    "A different voting private key is already stored for this identity and " +
                        "was kept. Replace it with the one you entered?",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        pendingReplacement = null
                        val keyText = votingKeyText.trim()
                        isLoading = true
                        scope.launch {
                            try {
                                val keys = MasternodeVotingKeyStore(container.walletStorage)
                                // Read the text again: the bytes decoded for the load were
                                // zeroed when the question was asked.
                                val key = try {
                                    keys.privateKeyBytesFromText(keyText)
                                } catch (e: CancellationException) {
                                    throw e
                                } catch (e: Exception) {
                                    error = votingKeyRefusedMessage(e.message)
                                    return@launch
                                }
                                val result = try {
                                    replaceVotingKey(
                                        dao = container.database.identityDao(),
                                        entries = StoredVotingKeyEntries(keys, container.database),
                                        identityId = pending.identityId,
                                        networkRaw = pending.networkRaw,
                                        expectedIdentifier = pending.storedIdentifier,
                                        votingKey = key,
                                        votingKeyCheck = pending.votingKeyCheck,
                                    )
                                } finally {
                                    key.fill(0)
                                }
                                when (result) {
                                    LoadSaveResult.Saved -> showIdentity(pending.identityId)
                                    is LoadSaveResult.Failed -> error = result.message
                                    is LoadSaveResult.SavedWithWarning -> {
                                        showAfterError = pending.identityId
                                        error = result.message
                                    }
                                    // A replacement never asks again by itself.
                                    is LoadSaveResult.ConfirmReplacement -> Unit
                                }
                            } catch (e: CancellationException) {
                                throw e
                            } catch (e: Exception) {
                                error = e.message ?: "Failed to replace the voting key"
                            } finally {
                                isLoading = false
                            }
                        }
                    },
                    modifier = Modifier.testTag("loadIdentity.replaceKey.voting"),
                ) { Text("Replace stored voting key") }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        pendingReplacement = null
                        isLoading = true
                        scope.launch {
                            try {
                                // Keeping the stored key is a completed load too: it retries
                                // the cleanup a previous warning promised.
                                val result = keepStoredVotingKey(
                                    StoredVotingKeyEntries(
                                        MasternodeVotingKeyStore(container.walletStorage),
                                        container.database,
                                    ),
                                    pending.identityId,
                                )
                                if (result is LoadSaveResult.SavedWithWarning) {
                                    showAfterError = pending.identityId
                                    error = result.message
                                } else {
                                    showIdentity(pending.identityId)
                                }
                            } finally {
                                isLoading = false
                            }
                        }
                    },
                    modifier = Modifier.testTag("loadIdentity.replaceKey.cancel"),
                ) { Text("Keep stored key") }
            },
        )
    }

    ErrorAlertDialog(
        message = error,
        onDismiss = {
            error = null
            showAfterError?.let {
                showAfterError = null
                showIdentity(it)
            }
        },
    )
}

/** The identity types Load Identity offers (Swift `IdentityType`). */
internal enum class LoadIdentityType(val rawValue: String) {
    USER("User"),
    MASTERNODE("Masternode"),
    EVONODE("Evonode"),
    ;

    /** Whether the type carries masternode keys. */
    val isMasternode: Boolean get() = this != USER
}

/** What to do with an entered voting key, given the identifier the row already records. */
internal sealed interface VotingKeyOutcome {
    /** The row already names this key: write its entry again from the entered bytes. */
    data object AlreadyStored : VotingKeyOutcome

    /** The row names no key: store it and record its identifier. */
    data object StoreNew : VotingKeyOutcome

    /** The row names a different key, which is kept until the user confirms replacing it. */
    data class Conflicting(val storedIdentifier: String) : VotingKeyOutcome
}

/**
 * Swift `KeychainManager.storeSpecialKeyIfAbsent`, adapted to identifiers that name the key:
 * the identifier [entered] (the entered key's `MasternodeVotingKeyStore` identifier for this
 * identity) against the row's [current] one.
 */
internal fun decideVotingKeyOutcome(current: String?, entered: String): VotingKeyOutcome = when {
    current == null -> VotingKeyOutcome.StoreNew
    current.equals(entered, ignoreCase = true) -> VotingKeyOutcome.AlreadyStored
    else -> VotingKeyOutcome.Conflicting(current)
}

/** The load error for a voting key Rust refused, with its [reason] and never the text. */
internal fun votingKeyRefusedMessage(reason: String?): String {
    val trimmed = reason?.trim()?.trimEnd('.')
    return if (trimmed.isNullOrEmpty()) {
        "The voting private key was refused."
    } else {
        "The voting private key was refused: $trimmed."
    }
}

/**
 * A conflicting voting key waiting for the user's answer. Holds no key bytes; [votingKeyCheck]
 * is the entered key's check at the load, recorded once the replacement stores it.
 */
private data class PendingVotingKeyReplacement(
    val identityId: ByteArray,
    val networkRaw: Int,
    val storedIdentifier: String,
    val votingKeyCheck: MasternodeVotingKeyCheck?,
) {
    override fun equals(other: Any?): Boolean =
        other is PendingVotingKeyReplacement && identityId.contentEquals(other.identityId) &&
            networkRaw == other.networkRaw && storedIdentifier == other.storedIdentifier &&
            votingKeyCheck == other.votingKeyCheck

    override fun hashCode(): Int =
        ((31 * identityId.contentHashCode() + networkRaw) * 31 + storedIdentifier.hashCode()) * 31 +
            (votingKeyCheck?.hashCode() ?: 0)
}

/**
 * The type the picker shows, until the user picks one, for the typed id's stored [row]: its
 * type when the row is on [networkRaw] and the type is one Load Identity offers, else User.
 */
internal fun inferredLoadIdentityType(row: IdentityEntity?, networkRaw: Int): LoadIdentityType =
    row?.takeIf { it.networkRaw == networkRaw }
        ?.let { stored -> LoadIdentityType.entries.firstOrNull { it.rawValue == stored.identityType } }
        ?: LoadIdentityType.USER

/**
 * The type a load writes: the [picked] one when the user picked it; otherwise the stored
 * row's [storedType], so a reload with the form's default type keeps a masternode or evonode
 * a voter, and User for a new row, so a type inferred for another id never carries over.
 */
internal fun identityTypeToSave(
    storedType: String?,
    picked: LoadIdentityType,
    typePicked: Boolean,
): String = when {
    typePicked -> picked.rawValue
    else -> storedType ?: LoadIdentityType.USER.rawValue
}

/** The load error for an identity already stored for another network. */
internal fun networkMismatchMessage(storedNetworkRaw: Int): String {
    val name = try {
        Network.fromFfiValue(storedNetworkRaw).displayName
    } catch (_: IllegalArgumentException) {
        "another network"
    }
    return "This identity is already stored for $name. Switch to that network to reload it."
}

/** Shown when another write changed the row while a load was saving it. */
internal const val IDENTITY_CHANGED_MESSAGE =
    "The identity changed while it was loading. Load it again."

/** Shown when the row stopped naming the key a replacement question showed. */
internal const val VOTING_KEY_CHANGED_MESSAGE =
    "The identity's stored voting key changed before it could be replaced, so it was kept. " +
        "Load the identity again with the new key to be asked again."

/** The load error for a voting key that could not be written; the identity was saved. */
internal fun votingKeyNotStoredMessage(reason: String?): String {
    val trimmed = reason?.trim()?.trimEnd('.')
    val because = if (trimmed.isNullOrEmpty()) "" else ": $trimmed"
    return "The identity was saved, but its voting private key could not be stored$because. " +
        "Load it again with the key to vote with it."
}

/** The error for a replacement key that could not be written; the stored key was kept. */
internal fun votingKeyNotReplacedMessage(reason: String?): String {
    val trimmed = reason?.trim()?.trimEnd('.')
    val because = if (trimmed.isNullOrEmpty()) "" else ": $trimmed"
    return "The new voting private key could not be stored$because. The stored key was kept."
}

/** Shown when a load succeeded but an earlier voting key entry could not be deleted yet. */
internal const val STALE_VOTING_KEY_NOT_REMOVED_MESSAGE =
    "The identity was loaded, but a previous voting key could not be removed from secure " +
        "storage yet. It is removed the next time this identity is loaded, or with Clear " +
        "Identities & Keys."

/** Shown when a replacement succeeded but the replaced entry could not be deleted yet. */
internal const val PREVIOUS_VOTING_KEY_NOT_REMOVED_MESSAGE =
    "The voting key was replaced, but the previous key could not be removed from secure " +
        "storage yet. It is removed the next time this identity is loaded, or with Clear " +
        "Identities & Keys."

/** What a load or a confirmed replacement did. */
internal sealed interface LoadSaveResult {
    /** Everything was saved; show the identity. */
    data object Saved : LoadSaveResult

    /** Saved, but show [message] before showing the identity. */
    data class SavedWithWarning(val message: String) : LoadSaveResult

    /** Show [message] and stay on the form. */
    data class Failed(val message: String) : LoadSaveResult

    /** The row names another voting key; ask before replacing [storedIdentifier]. */
    data class ConfirmReplacement(val storedIdentifier: String) : LoadSaveResult
}

/** The voting key persistence a load uses; [StoredVotingKeyEntries] in the app. */
internal interface VotingKeyEntries {
    /** The identifier [assign] would record for [votingKey] and [identityId]. */
    fun identifierFor(identityId: ByteArray, votingKey: ByteArray): String

    /** [MasternodeVotingKeyStore.assignToIdentity]: store and record, under the lock. */
    suspend fun assign(
        identityId: ByteArray,
        networkRaw: Int,
        votingKey: ByteArray,
        expectedIdentifier: String?,
        nowMillis: Long,
    ): VotingKeyAssignment

    /** [MasternodeVotingKeyStore.removeStaleEntries]: false when it failed. */
    suspend fun removeStaleEntries(identityId: ByteArray): Boolean

    /**
     * [MasternodeVotingKeyRecords.record]: write on the row what Rust says to record for
     * [check] of the key [identifier] names, given what the slot held before. Null when the
     * row no longer names that key, so nothing was written.
     */
    suspend fun recordCheck(
        identityId: ByteArray,
        networkRaw: Int,
        identifier: String,
        check: MasternodeVotingKeyCheck,
        slotBefore: MasternodeVotingKeyRecords.SlotBefore,
    ): MasternodeVotingKeyCheckOutcome?
}

/** [VotingKeyEntries] over [MasternodeVotingKeyStore] and the identities of [database]. */
internal class StoredVotingKeyEntries(
    private val keys: MasternodeVotingKeyStore,
    private val database: DashDatabase,
) : VotingKeyEntries {
    override fun identifierFor(identityId: ByteArray, votingKey: ByteArray): String =
        MasternodeVotingKeyStore.identifierFor(identityId, keys.votingAddress(votingKey))

    override suspend fun assign(
        identityId: ByteArray,
        networkRaw: Int,
        votingKey: ByteArray,
        expectedIdentifier: String?,
        nowMillis: Long,
    ): VotingKeyAssignment = keys.assignToIdentity(
        database, identityId, networkRaw, votingKey, expectedIdentifier, nowMillis,
    )

    override suspend fun removeStaleEntries(identityId: ByteArray): Boolean =
        keys.removeStaleEntries(database, identityId)

    override suspend fun recordCheck(
        identityId: ByteArray,
        networkRaw: Int,
        identifier: String,
        check: MasternodeVotingKeyCheck,
        slotBefore: MasternodeVotingKeyRecords.SlotBefore,
    ): MasternodeVotingKeyCheckOutcome? = MasternodeVotingKeyRecords.record(
        database, identityId, networkRaw, identifier, check, slotBefore,
    )
}

/** What [checkEnteredVotingKey] decided for an entered voting key. */
internal sealed interface EnteredVotingKeyCheck {
    /** Rust says to store the key: go on with the load. */
    data object Proceed : EnteredVotingKeyCheck

    /** Rust refused the key; show [message], nothing entered is stored. */
    data class Refused(val message: String) : EnteredVotingKeyCheck
}

/**
 * What a load does with an entered voting key after its [check], before anything is saved
 * (Swift `LoadIdentityView.swift`, the check block before the upsert). The slot is read from
 * the identity's row on [networkRaw]: empty, the same key (the row names [enteredIdentifier],
 * with the record it holds), or a different key. [rules] decide whether the key is stored.
 * When they refuse it and say to record the refusal on the same stored key, that record is
 * written on the row through [VotingKeyEntries.recordCheck]. [refusalText] replaces the
 * default wording, for a refusal Rust already worded (a load refused for the voter identity).
 */
internal suspend fun checkEnteredVotingKey(
    dao: IdentityDao,
    entries: VotingKeyEntries,
    rules: MasternodeVotingKeyRules,
    identityId: ByteArray,
    networkRaw: Int,
    enteredIdentifier: String,
    check: MasternodeVotingKeyCheck,
    refusalText: String? = null,
): EnteredVotingKeyCheck {
    val row = dao.getByIdentityId(identityId)?.takeIf { it.networkRaw == networkRaw }
    val stored = row?.votingPrivateKeyIdentifier
    val slot = when {
        stored == null -> MasternodeVotingKeySlot.Empty
        stored.equals(enteredIdentifier, ignoreCase = true) ->
            MasternodeVotingKeySlot.SameKey(row.recordedVotingKey)
        else -> MasternodeVotingKeySlot.DifferentKey
    }
    val address = MasternodeVotingKeyAddress.ofIdentifier(enteredIdentifier)
    val outcome = rules.outcome(check, address, slot)
    if (outcome.storeKey) return EnteredVotingKeyCheck.Proceed
    var text = refusalText ?: votingKeyNotOfMasternodeMessage(identityId.toHex(), reasonOf(check))
    if (outcome.record != null && stored != null && slot is MasternodeVotingKeySlot.SameKey) {
        // Nothing entered is stored. What Rust says to record is for the key already stored.
        try {
            entries.recordCheck(
                identityId, networkRaw, stored, check, MasternodeVotingKeyRecords.SlotBefore.SAME_KEY,
            )
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            text += " " + verdictNotSavedMessage(e.message)
        }
    }
    return EnteredVotingKeyCheck.Refused(text)
}

/** The load error for a voting key Platform proves cannot vote for the masternode. */
internal fun votingKeyNotOfMasternodeMessage(identityIdHex: String, reason: String): String =
    "The voting key does not belong to masternode $identityIdHex, so the identity was not " +
        "saved: $reason"

/** Added to a refusal when the refusal of the stored key could not be recorded. */
internal fun verdictNotSavedMessage(reason: String?): String =
    "The verdict on the stored voting key could not be saved: ${reason ?: "unknown error"}. " +
        "Load the identity again to retry."

/** The message a voting key check carries, empty for a valid one. */
internal fun reasonOf(check: MasternodeVotingKeyCheck): String = when (check) {
    MasternodeVotingKeyCheck.Valid -> ""
    is MasternodeVotingKeyCheck.Invalid -> check.message
    is MasternodeVotingKeyCheck.Undetermined -> check.message
}

/**
 * What the load reports when the stored voting key could not be checked against Platform,
 * worded from Rust's [outcome] for it: whether the earlier record is kept. Null when the key
 * was checked or no outcome was recorded. Swift `LoadIdentityView.uncheckedVotingKeyNote`.
 */
internal fun uncheckedVotingKeyNote(
    check: MasternodeVotingKeyCheck?,
    outcome: MasternodeVotingKeyCheckOutcome?,
): String? {
    if (check !is MasternodeVotingKeyCheck.Undetermined || outcome == null) return null
    return if (outcome.record == null) {
        check.message + " The result of its earlier check is kept."
    } else {
        check.message + " It is checked again when you pick this identity to vote."
    }
}

/**
 * Record [check] of the key the row now names ([identifier]) after it was stored, and return
 * what the load must say about it: the unchecked note, the failure to record it, or null.
 */
private suspend fun recordStoredVotingKeyCheck(
    entries: VotingKeyEntries,
    identityId: ByteArray,
    networkRaw: Int,
    identifier: String,
    check: MasternodeVotingKeyCheck?,
    slotBefore: MasternodeVotingKeyRecords.SlotBefore,
): String? {
    check ?: return null
    val outcome = try {
        entries.recordCheck(identityId, networkRaw, identifier, check, slotBefore)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        return votingKeyCheckNotSavedMessage(e.message)
    }
    return uncheckedVotingKeyNote(check, outcome)
}

/** Shown when the stored key's check could not be written; it is checked again when picked. */
internal fun votingKeyCheckNotSavedMessage(reason: String?): String =
    "The voting key check could not be saved: ${reason ?: "unknown error"}. It is checked " +
        "again when you pick this identity to vote."

/**
 * [result] with [note] about the stored voting key: a plain success becomes a warning that
 * opens with [lead], and a warning gains the note after its own text.
 */
private fun withVotingKeyNote(result: LoadSaveResult, lead: String, note: String?): LoadSaveResult {
    note ?: return result
    return when (result) {
        LoadSaveResult.Saved -> LoadSaveResult.SavedWithWarning("$lead $note")
        is LoadSaveResult.SavedWithWarning -> LoadSaveResult.SavedWithWarning("${result.message} $lead $note")
        else -> result
    }
}

/**
 * Save a fetched identity and, when [votingKey] is given, its voting key.
 *
 * The row is inserted when absent; otherwise only the fields a load owns change on the
 * current row, which must be on [networkRaw]; neither touches the voting key association.
 * A voting key is then stored and recorded by [MasternodeVotingKeyStore.assignToIdentity]
 * against the identifier the row names now, so a write made meanwhile is never overwritten:
 * the load reports the change instead. After a successful save, any other voting entry of
 * the identity (a replaced key, or one an earlier cleanup could not delete) is deleted by
 * [MasternodeVotingKeyStore.removeStaleEntries]; when that fails the result is
 * [LoadSaveResult.SavedWithWarning] and the next load tries again. [votingKey] stays the
 * caller's to zero.
 *
 * [masternodeProTxHash], the wire-order pro_tx_hash of a masternode or evonode loaded for
 * voting, is recorded on the row with its own fields: the row's id is the voter identity's for
 * a shared masternode, so votes read the pro_tx_hash from here (#6099).
 *
 * Once the key is stored, what Rust says to record for [votingKeyCheck] (its check before the
 * save, see [checkEnteredVotingKey]) is written on the row, from an empty slot for a new key
 * or with the row's record for the same key. A key that could not be checked is reported in a
 * warning saying what happens next.
 */
internal suspend fun saveLoadedIdentity(
    dao: IdentityDao,
    entries: VotingKeyEntries,
    identityId: ByteArray,
    networkRaw: Int,
    balance: Long,
    alias: String?,
    pickedType: LoadIdentityType,
    typePicked: Boolean,
    votingKey: ByteArray?,
    votingKeyCheck: MasternodeVotingKeyCheck? = null,
    masternodeProTxHash: ByteArray? = null,
    nowMillis: Long = System.currentTimeMillis(),
): LoadSaveResult {
    saveLoadedFields(dao, identityId, networkRaw, balance, alias, pickedType, typePicked, nowMillis)
        ?.let { return it }
    // A masternode or evonode row records the pro_tx_hash its votes name, apart from its id.
    if (masternodeProTxHash != null &&
        dao.setMasternodeProTxHash(identityId, networkRaw, masternodeProTxHash) == 0
    ) {
        return LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE)
    }
    if (votingKey == null) {
        return savedAfterCleanup(entries, identityId, STALE_VOTING_KEY_NOT_REMOVED_MESSAGE)
    }

    // The association as it is now, after the identity's own fields were saved; the
    // assignment below only records the key while the row still names it.
    val current = dao.getByIdentityId(identityId)
        ?: return LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE)
    val entered = entries.identifierFor(identityId, votingKey)
    val expected = when (val outcome = decideVotingKeyOutcome(current.votingPrivateKeyIdentifier, entered)) {
        // Written again from the entered bytes: repairs an entry that no longer reads.
        VotingKeyOutcome.AlreadyStored -> current.votingPrivateKeyIdentifier
        VotingKeyOutcome.StoreNew -> null
        is VotingKeyOutcome.Conflicting ->
            return LoadSaveResult.ConfirmReplacement(outcome.storedIdentifier)
    }
    val assignment = assignOrFail(entries, identityId, networkRaw, votingKey, expected, nowMillis) { reason ->
        return LoadSaveResult.Failed(votingKeyNotStoredMessage(reason))
    }
    return when (assignment) {
        is VotingKeyAssignment.Assigned -> {
            val note = recordStoredVotingKeyCheck(
                entries, identityId, networkRaw, assignment.identifier, votingKeyCheck,
                if (expected == null) {
                    MasternodeVotingKeyRecords.SlotBefore.EMPTY
                } else {
                    MasternodeVotingKeyRecords.SlotBefore.SAME_KEY
                },
            )
            withVotingKeyNote(
                savedAfterCleanup(entries, identityId, STALE_VOTING_KEY_NOT_REMOVED_MESSAGE),
                lead = "The identity was loaded and its voting key was stored.",
                note = note,
            )
        }
        is VotingKeyAssignment.Changed -> LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE)
    }
}

/** Insert or update the identity's own fields; null on success, else the failure. */
private suspend fun saveLoadedFields(
    dao: IdentityDao,
    identityId: ByteArray,
    networkRaw: Int,
    balance: Long,
    alias: String?,
    pickedType: LoadIdentityType,
    typePicked: Boolean,
    nowMillis: Long,
): LoadSaveResult.Failed? {
    val existing = dao.getByIdentityId(identityId)
    if (existing != null && existing.networkRaw != networkRaw) {
        return LoadSaveResult.Failed(networkMismatchMessage(existing.networkRaw))
    }
    val inserted = existing == null && dao.insertIfAbsent(
        IdentityEntity(
            identityId = identityId,
            balance = balance,
            // A manual add is a tracked identity under the owner's semantics; see the
            // LoadIdentityScreen header.
            isLocal = true,
            alias = alias,
            identityType = identityTypeToSave(null, pickedType, typePicked),
            networkRaw = networkRaw,
        ),
    ) != -1L
    if (inserted) return null
    // Either the row was there, or another write inserted it after the read above.
    val current = existing ?: dao.getByIdentityId(identityId)
        ?: return LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE)
    if (current.networkRaw != networkRaw) {
        return LoadSaveResult.Failed(networkMismatchMessage(current.networkRaw))
    }
    val changed = dao.updateLoadedFields(
        identityId = identityId,
        networkRaw = networkRaw,
        balance = balance,
        alias = alias,
        isLocal = true,
        // Null keeps the stored type: an unpicked type never overwrites a newer one written
        // since [current] was read.
        identityType = if (typePicked) pickedType.rawValue else null,
        nowMillis = nowMillis,
    )
    return if (changed == 0) LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE) else null
}

/**
 * Replace the identity's voting key with [votingKey] after the user confirmed replacing
 * [expectedIdentifier], the key the question showed, through
 * [MasternodeVotingKeyStore.assignToIdentity]: the new entry is recorded only while the row
 * still names [expectedIdentifier]; then [MasternodeVotingKeyStore.removeStaleEntries]
 * deletes the identity's other entries, the replaced one among them; when that fails, the
 * result says so and the next load deletes them. When the row changed meanwhile, the current association is kept and the result says
 * to load again, which asks again. [votingKey] stays the caller's to zero.
 *
 * Once the new key is stored, what Rust says to record for [votingKeyCheck] (the entered key's
 * check at the load) is written on the row, the slot having held a different key.
 */
internal suspend fun replaceVotingKey(
    dao: IdentityDao,
    entries: VotingKeyEntries,
    identityId: ByteArray,
    networkRaw: Int,
    expectedIdentifier: String,
    votingKey: ByteArray,
    votingKeyCheck: MasternodeVotingKeyCheck? = null,
    nowMillis: Long = System.currentTimeMillis(),
): LoadSaveResult {
    val assignment =
        assignOrFail(entries, identityId, networkRaw, votingKey, expectedIdentifier, nowMillis) { reason ->
            return LoadSaveResult.Failed(votingKeyNotReplacedMessage(reason))
        }
    return when (assignment) {
        is VotingKeyAssignment.Assigned -> {
            val note = recordStoredVotingKeyCheck(
                entries, identityId, networkRaw, assignment.identifier, votingKeyCheck,
                MasternodeVotingKeyRecords.SlotBefore.DIFFERENT_KEY,
            )
            withVotingKeyNote(
                savedAfterCleanup(entries, identityId, PREVIOUS_VOTING_KEY_NOT_REMOVED_MESSAGE),
                lead = "The voting key was replaced.",
                note = note,
            )
        }
        is VotingKeyAssignment.Changed -> LoadSaveResult.Failed(VOTING_KEY_CHANGED_MESSAGE)
    }
}

/**
 * The scope a voting key draft belongs to: the decoded identity id (so hex and Base58 of one
 * id match) on [networkRaw], or null while the text is not an identity id.
 */
internal fun votingKeyDraftScope(identityIdText: String, networkRaw: Int): String? =
    Base58.decodeIdentifier(identityIdText.trim())?.let { "${it.toHex()}@$networkRaw" }

/**
 * Whether a voting key draft typed for [lastScope] must be cleared now that the form names
 * [newScope]: when both name an identity and they differ. Text that is not an id yet (a
 * half-typed edit) clears nothing; the draft is kept for the next id only when it is the
 * same one.
 */
internal fun shouldClearVotingKeyDraft(lastScope: String?, newScope: String?): Boolean =
    lastScope != null && newScope != null && lastScope != newScope

/**
 * The user kept the stored key instead of replacing it: the load is complete, so the
 * identity's stale voting entries are cleaned up as after any load (see [savedAfterCleanup]).
 */
internal suspend fun keepStoredVotingKey(
    entries: VotingKeyEntries,
    identityId: ByteArray,
): LoadSaveResult = savedAfterCleanup(entries, identityId, STALE_VOTING_KEY_NOT_REMOVED_MESSAGE)

/**
 * [LoadSaveResult.Saved] once [MasternodeVotingKeyStore.removeStaleEntries] deleted the
 * identity's other voting entries, else [LoadSaveResult.SavedWithWarning] with
 * [warning]: the save stands and the next load retries. Cancellation propagates.
 */
private suspend fun savedAfterCleanup(
    entries: VotingKeyEntries,
    identityId: ByteArray,
    warning: String,
): LoadSaveResult {
    val removed = try {
        entries.removeStaleEntries(identityId)
    } catch (e: CancellationException) {
        throw e
    } catch (_: Exception) {
        false
    }
    return if (removed) LoadSaveResult.Saved else LoadSaveResult.SavedWithWarning(warning)
}

/**
 * [VotingKeyEntries.assign]; when the key cannot be written (a failure other than
 * cancellation, which propagates), hand [onFailure] the reason.
 */
private suspend inline fun assignOrFail(
    entries: VotingKeyEntries,
    identityId: ByteArray,
    networkRaw: Int,
    votingKey: ByteArray,
    expectedIdentifier: String?,
    nowMillis: Long,
    onFailure: (reason: String?) -> Nothing,
): VotingKeyAssignment = try {
    entries.assign(identityId, networkRaw, votingKey, expectedIdentifier, nowMillis)
} catch (e: CancellationException) {
    throw e
} catch (e: Exception) {
    onFailure(e.message)
}
