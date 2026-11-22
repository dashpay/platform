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
import java.util.Date
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
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
 * type, `isLocal`, the network and `lastUpdated` change, so a wallet link, key identifiers
 * and DPNS names survive a reload.
 *
 * A masternode or evonode identity can bring its voting private key (hex or WIF, read by
 * Rust through [MasternodeVotingKeyStore.privateKeyBytesFromText]). The key is read before
 * the fetch, so a refused key stops the load with Rust's reason and nothing is written. Once
 * the identity is saved, the key is stored in `WalletStorage` under its voting address, the
 * identifier the row records, and the vote dialogs sign with it through the wallet
 * manager's signer (Swift `storeSpecialKeyIfAbsent`, adapted to address-keyed storage, see
 * [decideVotingKeyOutcome]). A different key already stored for the identity is kept until
 * the user confirms replacing it. Owner and payout keys and identity private keys are not
 * imported: nothing in this app signs with them.
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
    // The typed text, never the decoded bytes: a confirmed replacement reads it again.
    var votingKeyText by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var pendingReplacement by remember { mutableStateOf<PendingVotingKeyReplacement?>(null) }

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

                        val dao = container.database.identityDao()
                        val existing = dao.getByIdentityId(idBytes)
                        var row = existing?.let {
                            it.copy(
                                balance = balance,
                                alias = trimmedAlias ?: it.alias,
                                // A manual add is a tracked identity under the owner's
                                // semantics; see the file header.
                                isLocal = true,
                                identityType = type.rawValue,
                                networkRaw = activeNetwork.ffiValue,
                                lastUpdated = Date(),
                            )
                        } ?: IdentityEntity(
                            identityId = idBytes,
                            balance = balance,
                            isLocal = true,
                            alias = trimmedAlias,
                            identityType = type.rawValue,
                            networkRaw = activeNetwork.ffiValue,
                        )

                        val key = votingKey
                        if (key == null) {
                            dao.upsert(row)
                            showIdentity(idBytes)
                            return@launch
                        }

                        val enteredIdentifier = keys.votingAddress(key).toHex()
                        val outcome =
                            decideVotingKeyOutcome(row.votingPrivateKeyIdentifier, enteredIdentifier)
                        when (outcome) {
                            VotingKeyOutcome.AlreadyStored -> {
                                // Heals a missing ciphertext; a no-op when the key reads.
                                runCatching { keys.store(key) }
                                dao.upsert(row)
                                showIdentity(idBytes)
                            }
                            VotingKeyOutcome.StoreNew -> {
                                val stored = try {
                                    keys.store(key)
                                } catch (e: CancellationException) {
                                    throw e
                                } catch (_: Exception) {
                                    null
                                }
                                if (stored != null) row = row.copy(votingPrivateKeyIdentifier = stored)
                                // The identity itself loaded, so its row is saved even when
                                // the key could not be stored.
                                dao.upsert(row)
                                if (stored != null) {
                                    showIdentity(idBytes)
                                } else {
                                    error = VOTING_KEY_NOT_STORED_MESSAGE
                                }
                            }
                            is VotingKeyOutcome.Conflicting -> {
                                // The stored key and the row's identifier are kept; the
                                // replacement waits for the user's answer.
                                dao.upsert(row)
                                pendingReplacement = PendingVotingKeyReplacement(
                                    identityId = idBytes,
                                    storedIdentifier = outcome.storedIdentifier,
                                )
                            }
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
                                val message = replaceVotingKey(
                                    keys = MasternodeVotingKeyStore(container.walletStorage),
                                    container = container,
                                    pending = pending,
                                    keyText = keyText,
                                )
                                if (message == null) {
                                    showIdentity(pending.identityId)
                                } else {
                                    error = message
                                }
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

    ErrorAlertDialog(message = error, onDismiss = { error = null })
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
    /** The row already names this key: store it again only to heal a missing ciphertext. */
    data object AlreadyStored : VotingKeyOutcome

    /** The row names no key: store it and record its identifier. */
    data object StoreNew : VotingKeyOutcome

    /** The row names a different key, which is kept until the user confirms replacing it. */
    data class Conflicting(val storedIdentifier: String) : VotingKeyOutcome
}

/**
 * Swift `KeychainManager.storeSpecialKeyIfAbsent`, adapted to voting keys stored under their
 * voting address: the identifier [entered] (the entered key's lowercase address hex) against
 * the row's [current] one.
 */
internal fun decideVotingKeyOutcome(current: String?, entered: String): VotingKeyOutcome = when {
    current == null -> VotingKeyOutcome.StoreNew
    current.equals(entered, ignoreCase = true) -> VotingKeyOutcome.AlreadyStored
    else -> VotingKeyOutcome.Conflicting(current)
}

/** Shown when a voting key could not be stored; the identity itself was saved. */
internal const val VOTING_KEY_NOT_STORED_MESSAGE =
    "The voting private key could not be stored. The identity cannot vote until it is " +
        "loaded again."

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
 * Replace the identity's stored voting key with the one typed in [keyText] after the user
 * confirmed it: read the text again, store the new key, point the row at it, and delete the
 * key the row named before only when no identity still names it (several masternodes can
 * share one voting key). Returns null on success, else the message to show; a failure
 * before the row changes leaves the stored key and the row as they were.
 */
private suspend fun replaceVotingKey(
    keys: MasternodeVotingKeyStore,
    container: org.dashfoundation.example.di.AppContainer,
    pending: PendingVotingKeyReplacement,
    keyText: String,
): String? {
    val key = try {
        keys.privateKeyBytesFromText(keyText)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        return votingKeyRefusedMessage(e.message)
    }
    try {
        val newIdentifier = try {
            keys.store(key)
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            return "The voting private key could not be stored, so the stored key was kept."
        }
        val dao = container.database.identityDao()
        val row = dao.getByIdentityId(pending.identityId)
            ?: return "The identity could not be found on this device, so no key was " +
                "replaced. Load it again."
        // The key the row names now, which another load may have changed since the
        // question was asked; it is the one being replaced.
        val replaced = row.votingPrivateKeyIdentifier
        dao.upsert(row.copy(votingPrivateKeyIdentifier = newIdentifier, lastUpdated = Date()))
        if (replaced != null && !replaced.equals(newIdentifier, ignoreCase = true) &&
            dao.countByVotingPrivateKeyIdentifier(replaced) == 0
        ) {
            keys.delete(replaced)
        }
        return null
    } finally {
        key.fill(0)
    }
}
