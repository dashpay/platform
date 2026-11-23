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
import org.dashfoundation.dashsdk.persistence.dao.IdentityDao
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.IdentityDetail
import org.dashfoundation.example.navigation.QrScanner
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
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

                        val json = activeSdk.identities.fetch(id) ?: run {
                            error = "Identity not found on ${activeNetwork.displayName}"
                            return@launch
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

                        val result = saveLoadedIdentity(
                            dao = container.database.identityDao(),
                            entries = StoredVotingKeyEntries(keys),
                            identityId = idBytes,
                            networkRaw = activeNetwork.ffiValue,
                            balance = balance,
                            alias = trimmedAlias,
                            pickedType = type,
                            typePicked = typePicked,
                            votingKey = votingKey,
                        )
                        when (result) {
                            LoadSaveResult.Saved -> showIdentity(idBytes)
                            is LoadSaveResult.Failed -> error = result.message
                            is LoadSaveResult.SavedWithWarning -> {
                                showAfterError = idBytes
                                error = result.message
                            }
                            is LoadSaveResult.ConfirmReplacement ->
                                pendingReplacement = PendingVotingKeyReplacement(
                                    identityId = idBytes,
                                    storedIdentifier = result.storedIdentifier,
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
                                        entries = StoredVotingKeyEntries(keys),
                                        identityId = pending.identityId,
                                        expectedIdentifier = pending.storedIdentifier,
                                        votingKey = key,
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
                        showIdentity(pending.identityId)
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

/** A conflicting voting key waiting for the user's answer. Holds no key bytes. */
private data class PendingVotingKeyReplacement(
    val identityId: ByteArray,
    val storedIdentifier: String,
) {
    override fun equals(other: Any?): Boolean =
        other is PendingVotingKeyReplacement && identityId.contentEquals(other.identityId) &&
            storedIdentifier == other.storedIdentifier

    override fun hashCode(): Int = 31 * identityId.contentHashCode() + storedIdentifier.hashCode()
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

/** The voting key entries a load writes; [StoredVotingKeyEntries] in the app. */
internal interface VotingKeyEntries {
    /** The identifier [store] would return for [votingKey] and [identityId]. */
    fun identifierFor(identityId: ByteArray, votingKey: ByteArray): String

    /** Write [identityId]'s voting entry from [votingKey] and return its identifier. */
    suspend fun store(identityId: ByteArray, votingKey: ByteArray): String

    /** Delete the entry [identifier]. */
    suspend fun delete(identifier: String)

    /** Delete every entry of [identityId] except [keep] (null: all of them). */
    suspend fun deleteOtherEntriesOf(identityId: ByteArray, keep: String?)
}

/** [VotingKeyEntries] over [MasternodeVotingKeyStore]. */
internal class StoredVotingKeyEntries(private val keys: MasternodeVotingKeyStore) : VotingKeyEntries {
    override fun identifierFor(identityId: ByteArray, votingKey: ByteArray): String =
        MasternodeVotingKeyStore.identifierFor(identityId, keys.votingAddress(votingKey))

    override suspend fun store(identityId: ByteArray, votingKey: ByteArray): String =
        keys.store(identityId, votingKey)

    override suspend fun delete(identifier: String) = keys.delete(identifier)

    override suspend fun deleteOtherEntriesOf(identityId: ByteArray, keep: String?) =
        keys.deleteOtherEntriesOf(identityId, keep)
}

/**
 * Save a fetched identity and, when [votingKey] is given, its voting key.
 *
 * The row is inserted when absent; otherwise only the fields a load owns change on the
 * current row, which must be on [networkRaw]. The voting key association then changes only
 * through a conditional update against the identifier the row names now, so a write made
 * meanwhile is never overwritten: the entry just written is discarded unless the row names
 * it, and the load reports the change. After a successful save, any other voting entry of
 * the identity (left by an earlier replacement whose cleanup failed) is deleted; when that
 * fails the result is [LoadSaveResult.SavedWithWarning] and the next load tries again.
 *
 * The row writes, the key write and the sweep run as one
 * [MasternodeVotingKeyStore.withVotingKeyLock] section, so a confirmed replacement or a
 * clear cannot interleave with them. [votingKey] stays the caller's to zero.
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
    nowMillis: Long = System.currentTimeMillis(),
): LoadSaveResult = MasternodeVotingKeyStore.withVotingKeyLock {
    val result = saveLoadedIdentityRow(
        dao, entries, identityId, networkRaw, balance, alias, pickedType, typePicked, votingKey,
        nowMillis,
    )
    when {
        result != LoadSaveResult.Saved -> result
        sweepStaleEntries(
            entries, identityId, dao.getByIdentityId(identityId)?.votingPrivateKeyIdentifier,
        ) -> result
        else -> LoadSaveResult.SavedWithWarning(STALE_VOTING_KEY_NOT_REMOVED_MESSAGE)
    }
}

private suspend fun saveLoadedIdentityRow(
    dao: IdentityDao,
    entries: VotingKeyEntries,
    identityId: ByteArray,
    networkRaw: Int,
    balance: Long,
    alias: String?,
    pickedType: LoadIdentityType,
    typePicked: Boolean,
    votingKey: ByteArray?,
    nowMillis: Long,
): LoadSaveResult {
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
    if (!inserted) {
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
            identityType = identityTypeToSave(current.identityType, pickedType, typePicked),
            nowMillis = nowMillis,
        )
        if (changed == 0) return LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE)
    }
    if (votingKey == null) return LoadSaveResult.Saved

    // The association as it is now, after the identity's own fields were saved.
    val current = dao.getByIdentityId(identityId)
        ?: return LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE)
    val entered = entries.identifierFor(identityId, votingKey)
    return when (val outcome = decideVotingKeyOutcome(current.votingPrivateKeyIdentifier, entered)) {
        VotingKeyOutcome.AlreadyStored -> {
            // Written again from the entered bytes: repairs an entry that no longer reads.
            val stored = storeEntry(entries, identityId, votingKey) { reason ->
                return LoadSaveResult.Failed(votingKeyNotStoredMessage(reason))
            }
            if (dao.getByIdentityId(identityId)?.votingPrivateKeyIdentifier == stored) {
                LoadSaveResult.Saved
            } else {
                discardUnlessNamed(dao, entries, identityId, stored)
                LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE)
            }
        }
        VotingKeyOutcome.StoreNew -> {
            val stored = storeEntry(entries, identityId, votingKey) { reason ->
                return LoadSaveResult.Failed(votingKeyNotStoredMessage(reason))
            }
            if (dao.replaceVotingPrivateKeyIdentifier(identityId, null, stored, nowMillis) == 1) {
                LoadSaveResult.Saved
            } else {
                discardUnlessNamed(dao, entries, identityId, stored)
                LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE)
            }
        }
        is VotingKeyOutcome.Conflicting -> LoadSaveResult.ConfirmReplacement(outcome.storedIdentifier)
    }
}

/**
 * Replace the identity's voting key with [votingKey] after the user confirmed replacing
 * [expectedIdentifier], the key the question showed: store the new entry, point the row at it
 * only while the row still names [expectedIdentifier], then delete the identity's other
 * entries, the replaced one among them; when that fails, the result says so and the next
 * load deletes them. When the row
 * changed meanwhile, the new entry is discarded unless the row names it, the current
 * association is kept, and the result says to load again, which asks again. An entry's name
 * contains its identity's id, so the replaced entry is only this row's. The whole replacement
 * is one [MasternodeVotingKeyStore.withVotingKeyLock] section. [votingKey] stays the caller's
 * to zero.
 */
internal suspend fun replaceVotingKey(
    dao: IdentityDao,
    entries: VotingKeyEntries,
    identityId: ByteArray,
    expectedIdentifier: String,
    votingKey: ByteArray,
    nowMillis: Long = System.currentTimeMillis(),
): LoadSaveResult = MasternodeVotingKeyStore.withVotingKeyLock {
    replaceVotingKeyLocked(dao, entries, identityId, expectedIdentifier, votingKey, nowMillis)
}

private suspend fun replaceVotingKeyLocked(
    dao: IdentityDao,
    entries: VotingKeyEntries,
    identityId: ByteArray,
    expectedIdentifier: String,
    votingKey: ByteArray,
    nowMillis: Long,
): LoadSaveResult {
    val stored = storeEntry(entries, identityId, votingKey) { reason ->
        return LoadSaveResult.Failed(votingKeyNotReplacedMessage(reason))
    }
    if (dao.replaceVotingPrivateKeyIdentifier(identityId, expectedIdentifier, stored, nowMillis) == 0) {
        discardUnlessNamed(dao, entries, identityId, stored)
        return LoadSaveResult.Failed(VOTING_KEY_CHANGED_MESSAGE)
    }
    // The update succeeded, so the row names [stored]; the sweep deletes the replaced entry
    // with any other leftover of this identity. If it fails, the next load sweeps again.
    return if (sweepStaleEntries(entries, identityId, keep = stored)) {
        LoadSaveResult.Saved
    } else {
        LoadSaveResult.SavedWithWarning(PREVIOUS_VOTING_KEY_NOT_REMOVED_MESSAGE)
    }
}

/**
 * Delete every voting entry of [identityId] but [keep], the one its row names. Returns false
 * when that failed (other than by cancellation, which propagates). Callers hold the voting
 * key lock, so [keep] stays the row's identifier for the whole sweep.
 */
private suspend fun sweepStaleEntries(
    entries: VotingKeyEntries,
    identityId: ByteArray,
    keep: String?,
): Boolean = try {
    entries.deleteOtherEntriesOf(identityId, keep)
    true
} catch (e: CancellationException) {
    throw e
} catch (_: Exception) {
    false
}

/**
 * Write [identityId]'s voting entry; on a failure other than cancellation, hand [onFailure]
 * the reason. Cancellation always propagates.
 */
private suspend inline fun storeEntry(
    entries: VotingKeyEntries,
    identityId: ByteArray,
    votingKey: ByteArray,
    onFailure: (reason: String?) -> Nothing,
): String = try {
    entries.store(identityId, votingKey)
} catch (e: CancellationException) {
    throw e
} catch (e: Exception) {
    onFailure(e.message)
}

/** Delete [identifier] unless the identity's row names it now. */
private suspend fun discardUnlessNamed(
    dao: IdentityDao,
    entries: VotingKeyEntries,
    identityId: ByteArray,
    identifier: String,
) {
    if (dao.getByIdentityId(identityId)?.votingPrivateKeyIdentifier != identifier) {
        entries.delete(identifier)
    }
}
