package org.dashfoundation.example.ui.voting

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyNotStoredException
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.example.BuildConfig
import org.dashfoundation.example.di.AppContainer
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.hexToBytes
import org.dashfoundation.example.util.toHex

// Who votes. Port of `MasternodeVoterSection.swift`
// (`SwiftExampleApp/Views/Voting/`): the voter type, the cast dispatch and the picker
// shared by the DPNS contest vote, the decision proposal vote and the multiple choice
// proposal vote.

/**
 * The masternode a vote dialog votes for, and how its vote is signed. Port of Swift's
 * `MasternodeVoter`. The dialogs use the same value to cast and to read "this masternode's
 * answer", so both always name the same masternode.
 */
sealed interface MasternodeVoter {

    /** Whether a vote can be cast as this voter (Swift `canCast`). */
    val canCast: Boolean

    /** The pro_tx_hash in the wire order the vote and the answer reads take. */
    val wireOrderProTxHash: ByteArray

    /**
     * A masternode or evonode identity loaded on this device with its voting key (Load
     * Identity). The key stays in `WalletStorage` and signs through the wallet manager's
     * signer.
     */
    class StoredKey(identityId: ByteArray) : MasternodeVoter {
        private val id = identityId.copyOf()
        val identityId: ByteArray get() = id.copyOf()
        override val canCast: Boolean get() = true
        override val wireOrderProTxHash: ByteArray get() = wireOrderProTxHash(id)
        override fun equals(other: Any?): Boolean = other is StoredKey && id.contentEquals(other.id)
        override fun hashCode(): Int = id.contentHashCode()
        override fun toString(): String = "StoredKey(${id.toHex()})"
    }

    /**
     * A pro_tx_hash (wire order) typed into the dialog, with no key: enough to read any
     * masternode's answer, never to cast. Available in every build.
     */
    class TypedProTxHash(proTxHash: ByteArray) : MasternodeVoter {
        private val hash = proTxHash.copyOf()
        override val canCast: Boolean get() = false
        override val wireOrderProTxHash: ByteArray get() = hash.copyOf()
        override fun equals(other: Any?): Boolean =
            other is TypedProTxHash && hash.contentEquals(other.hash)
        override fun hashCode(): Int = hash.contentHashCode()
        override fun toString(): String = "TypedProTxHash(${hash.toHex()})"
    }

    /**
     * Debug builds only: a pro_tx_hash (wire order) and a voting private key typed into
     * the dialog. Only [publishVoter] with `debugBuild = BuildConfig.DEBUG` builds one, so
     * a release build never holds it. The key is null while its field is empty or not 64
     * hex characters: reading the masternode's answer needs only the pro_tx_hash, casting
     * needs both.
     */
    class TypedCredentials(proTxHash: ByteArray, votingPrivateKey: ByteArray?) : MasternodeVoter {
        private val hash = proTxHash.copyOf()
        private val key = votingPrivateKey?.copyOf()
        override val canCast: Boolean get() = key != null
        override val wireOrderProTxHash: ByteArray get() = hash.copyOf()

        /** A copy of the typed key, the caller's to zero; null without one. */
        fun votingPrivateKeyCopy(): ByteArray? = key?.copyOf()

        override fun equals(other: Any?): Boolean =
            other is TypedCredentials && hash.contentEquals(other.hash) &&
                (key?.let { k -> other.key?.contentEquals(k) == true } ?: (other.key == null))
        override fun hashCode(): Int = 31 * hash.contentHashCode() + (key?.contentHashCode() ?: 0)

        // Never prints the key.
        override fun toString(): String =
            "TypedCredentials(${hash.toHex()}, key=${if (key != null) "set" else "none"})"
    }
}

// ── Pure logic (unit tested) ─────────────────────────────────────────

/**
 * A loaded masternode identity's id is the pro_tx_hash as Platform names it, the hex Core
 * displays. The vote calls take the wire order, its reverse, and Rust reverses it back
 * (see "pro_tx_hash byte order" in `rs-sdk-ffi/src/voting/masternode_vote.rs`). This is
 * the only place the app converts between the two (Swift
 * `MasternodeVoter.wireOrderProTxHash(identityId:)`).
 */
fun wireOrderProTxHash(identityId: ByteArray): ByteArray = identityId.reversedArray()

/** The identity types that carry a masternode voting key (`IdentityType.rawValue`). */
internal val VOTING_IDENTITY_TYPES = setOf("Masternode", "Evonode")

/**
 * Loaded masternode and evonode identities on [networkRaw] that carry a stored voting key,
 * by display name (Swift `votingIdentities`).
 */
internal fun votingCandidates(identities: List<IdentityEntity>, networkRaw: Int): List<IdentityEntity> =
    identities
        .filter { identity ->
            identity.networkRaw == networkRaw &&
                identity.votingPrivateKeyIdentifier != null &&
                identity.identityType in VOTING_IDENTITY_TYPES
        }
        .sortedBy { voterDisplayName(it) }

/**
 * Swift `PersistentIdentity.displayName`: the alias, else the main DPNS name, else the DPNS
 * name, else the start of the base58 id.
 */
internal fun voterDisplayName(identity: IdentityEntity): String =
    identity.alias?.takeIf { it.isNotEmpty() }
        ?: identity.mainDpnsName
        ?: identity.dpnsName?.takeIf { it.isNotEmpty() }
        ?: (Base58.encode(identity.identityId).take(12) + "...")

/**
 * The identity's name and type, plus the start of its id when another candidate shows the
 * same name, since aliases need not be unique (Swift `voterLabel`).
 */
internal fun voterLabel(identity: IdentityEntity, candidates: List<IdentityEntity>): String {
    val name = voterDisplayName(identity)
    val shared = candidates.any {
        !it.identityId.contentEquals(identity.identityId) && voterDisplayName(it) == name
    }
    val suffix = if (shared) ", ${identity.identityId.toHex().take(8)}" else ""
    return "$name (${identity.identityType}$suffix)"
}

/**
 * The identity the picker shows: [selectedIdentityHex] while it is still a candidate, else
 * the first candidate, null when there is none (Swift `selectDefaultIfNeeded`).
 */
internal fun effectiveVoterSelection(
    selectedIdentityHex: String?,
    candidates: List<IdentityEntity>,
): IdentityEntity? =
    candidates.firstOrNull { it.identityId.toHex() == selectedIdentityHex } ?: candidates.firstOrNull()

/**
 * A 32-byte value typed as exactly 64 hex characters (surrounding whitespace ignored);
 * null for anything else.
 */
internal fun decodeTyped32Bytes(text: String): ByteArray? {
    val trimmed = text.trim()
    if (trimmed.length != 64 || !trimmed.all { it.isDigit() || it in 'a'..'f' || it in 'A'..'F' }) {
        return null
    }
    return trimmed.hexToBytes()
}

/**
 * The voter the section's inputs name (Swift `publish()`), null while nothing usable is
 * picked or typed.
 *
 * [debugBuild] is `BuildConfig.DEBUG` in the app: typed credentials are honored only in a
 * debug build, so a release build cannot publish a raw key even if [useTypedCredentials]
 * were set.
 */
internal fun publishVoter(
    selected: IdentityEntity?,
    allowsTypedProTxHash: Boolean,
    useTypedProTxHash: Boolean,
    proTxHashText: String,
    debugBuild: Boolean,
    useTypedCredentials: Boolean,
    votingKeyText: String,
): MasternodeVoter? {
    if (debugBuild && useTypedCredentials) {
        // The pro_tx_hash alone is enough to read the masternode's answer, so the voter is
        // published without a key while the key field is empty or not a key; canCast keeps
        // Submit disabled until it is.
        val proTxHash = decodeTyped32Bytes(proTxHashText) ?: return null
        val key = decodeTyped32Bytes(votingKeyText)
        try {
            return MasternodeVoter.TypedCredentials(proTxHash, key)
        } finally {
            key?.fill(0)
        }
    }
    if (allowsTypedProTxHash && useTypedProTxHash) {
        return decodeTyped32Bytes(proTxHashText)?.let { MasternodeVoter.TypedProTxHash(it) }
    }
    return selected?.let { MasternodeVoter.StoredKey(it.identityId) }
}

// ── Casting ──────────────────────────────────────────────────────────

/** Why a vote could not be cast before anything reached Platform. */
class MasternodeVoteCallException(message: String) : Exception(message)

/** Shown when a picked identity has no readable stored voting key. */
internal const val NO_STORED_VOTING_KEY_MESSAGE =
    "No voting key is stored for this masternode identity. Load it again with its voting " +
        "private key."

/** Shown when there is no wallet manager whose signer could sign the vote. */
internal const val NO_WALLET_MANAGER_MESSAGE =
    "No wallet manager is active on this network, so the stored voting key cannot sign. " +
        "Open the Wallets tab and wait for the wallets to load, then vote again."

/**
 * One masternode vote, as each dialog's own SDK call makes it. Port of Swift
 * `MasternodeVoteCall`: [run] picks how the vote is signed, so the dialogs do not repeat
 * the dispatch.
 *
 * [withSigner] casts with a signer: the wire-order pro_tx_hash, the 20-byte voting address
 * and the active wallet manager, which the lambda passes to the SDK's signer overload. [run]
 * calls it inside [PlatformWalletManager.withStoredMasternodeVotingKey], which makes the
 * identity's stored voting key signable for that call only and keeps the manager from
 * freeing the signer while the vote is in flight. The manager is read when [run] is called,
 * never held in composition.
 */
class MasternodeVoteCall(
    private val withSigner: suspend (
        proTxHash: ByteArray,
        votingAddress: ByteArray,
        manager: PlatformWalletManager,
    ) -> Unit,
) {
    private var withTypedKey: (suspend (proTxHash: ByteArray, votingPrivateKey: ByteArray) -> Unit)? = null

    /**
     * Debug builds only: this call, also able to cast with typed credentials. Call it only
     * inside `if (BuildConfig.DEBUG)` so a release build has no reachable raw-key call; in a
     * release build it returns the call unchanged.
     */
    fun allowingTypedKey(
        cast: suspend (proTxHash: ByteArray, votingPrivateKey: ByteArray) -> Unit,
    ): MasternodeVoteCall {
        if (BuildConfig.DEBUG) withTypedKey = cast
        return this
    }

    /**
     * Cast as [voter]. A stored key signs through the active wallet manager's signer: the
     * identity's voting key identifier is read from Room now, and the manager refuses the
     * vote when no readable key is stored under it.
     *
     * @throws MasternodeVoteCallException when the vote cannot be cast as [voter]; any SDK
     *   error from the cast itself propagates.
     */
    suspend fun run(voter: MasternodeVoter, container: AppContainer) {
        when (voter) {
            is MasternodeVoter.StoredKey -> {
                val identity = container.database.identityDao().getByIdentityId(voter.identityId)
                val identifier = identity?.votingPrivateKeyIdentifier
                    ?: throw MasternodeVoteCallException(NO_STORED_VOTING_KEY_MESSAGE)
                val manager = container.walletManagerStore.activeManager.value
                    ?: throw MasternodeVoteCallException(NO_WALLET_MANAGER_MESSAGE)
                try {
                    manager.withStoredMasternodeVotingKey(identifier) { votingAddress ->
                        withSigner(voter.wireOrderProTxHash, votingAddress, manager)
                    }
                } catch (_: MasternodeVotingKeyNotStoredException) {
                    throw MasternodeVoteCallException(NO_STORED_VOTING_KEY_MESSAGE)
                }
            }
            is MasternodeVoter.TypedProTxHash -> throw MasternodeVoteCallException(
                "A typed pro_tx_hash only reads a masternode's answer. Pick a loaded " +
                    "masternode to cast a vote.",
            )
            is MasternodeVoter.TypedCredentials -> {
                val cast = withTypedKey.takeIf { BuildConfig.DEBUG }
                    ?: throw MasternodeVoteCallException(
                        "This vote cannot be cast with typed credentials.",
                    )
                val key = voter.votingPrivateKeyCopy()
                    ?: throw MasternodeVoteCallException(
                        "Enter the voting private key to cast a vote with typed credentials.",
                    )
                try {
                    cast(voter.wireOrderProTxHash, key)
                } finally {
                    key.fill(0)
                }
            }
        }
    }
}

// ── Picking the voter ────────────────────────────────────────────────

/**
 * The form section a vote dialog uses to pick who votes: a loaded masternode or evonode
 * identity on the current network whose voting key is stored. Port of Swift
 * `MasternodeVoterSection`. Reports the choice through [onVoterChange], null while nothing
 * usable is picked.
 *
 * [testTagPrefix] names the dialog (`contestVote`, `decisionProposalVote`,
 * `multipleChoiceProposalVote`); the picker is `<prefix>.voter`, the empty-state note
 * `<prefix>.noVoter`.
 *
 * With [allowsTypedProTxHash], a dialog that reads a masternode's answer also offers, in
 * every build, a typed pro_tx_hash with no key (`<prefix>.useTypedProTxHash`, field
 * `<prefix>.proTxHash`): it reads any masternode's answer and casts nothing.
 *
 * Debug builds only: typed test credentials (`<prefix>.useTypedCredentials`, fields
 * `<prefix>.proTxHash` and `<prefix>.votingKey`). The toggle and the key field are composed
 * only when `BuildConfig.DEBUG`, so a release build cannot be used to paste a raw voting
 * private key, bypassing the signer boundary the stored-key path keeps.
 */
@Composable
fun MasternodeVoterSection(
    testTagPrefix: String,
    onVoterChange: (MasternodeVoter?) -> Unit,
    modifier: Modifier = Modifier,
    allowsTypedProTxHash: Boolean = false,
    enabled: Boolean = true,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val identities by remember(network) {
        container.database.identityDao().observeByNetwork(network.ffiValue)
    }.collectAsStateWithLifecycle(initialValue = emptyList())
    val candidates = remember(identities, network) { votingCandidates(identities, network.ffiValue) }

    var selectedIdentityHex by rememberSaveable { mutableStateOf<String?>(null) }
    var useTypedProTxHash by rememberSaveable { mutableStateOf(false) }
    // Shared by the typed pro_tx_hash and, in debug builds, the typed credentials; their
    // toggles are mutually exclusive.
    var proTxHashText by rememberSaveable { mutableStateOf("") }
    // Never saved into the instance state bundle: a key typed here stays in memory only.
    var useTypedCredentials by remember { mutableStateOf(false) }
    var votingKeyText by remember { mutableStateOf("") }

    val selected = effectiveVoterSelection(selectedIdentityHex, candidates)
    val usesTypedInput = (BuildConfig.DEBUG && useTypedCredentials) ||
        (allowsTypedProTxHash && useTypedProTxHash)
    val voter = publishVoter(
        selected = selected,
        allowsTypedProTxHash = allowsTypedProTxHash,
        useTypedProTxHash = useTypedProTxHash,
        proTxHashText = proTxHashText,
        debugBuild = BuildConfig.DEBUG,
        useTypedCredentials = useTypedCredentials,
        votingKeyText = votingKeyText,
    )
    LaunchedEffect(voter) { onVoterChange(voter) }

    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            "Voting Masternode",
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold,
        )
        if (candidates.isEmpty()) {
            Text(
                "No masternode or evonode identity with a voting key is loaded on this " +
                    "network. Load one with its voting private key under Identities, Load " +
                    "Identity.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.testTag("$testTagPrefix.noVoter"),
            )
        } else if (selected != null) {
            AccessiblePicker(
                label = "Masternode",
                options = candidates,
                selected = selected,
                optionLabel = { voterLabel(it, candidates) },
                testTag = "$testTagPrefix.voter",
                enabled = enabled && !usesTypedInput,
                onSelected = { selectedIdentityHex = it.identityId.toHex() },
            )
        }

        if (allowsTypedProTxHash) {
            ToggleRow(
                label = "Read another masternode's answer",
                checked = useTypedProTxHash,
                enabled = enabled,
                testTag = "$testTagPrefix.useTypedProTxHash",
                onCheckedChange = { isOn ->
                    useTypedProTxHash = isOn
                    if (isOn) useTypedCredentials = false
                },
            )
            if (useTypedProTxHash) {
                ProTxHashField(proTxHashText, enabled, testTagPrefix) { proTxHashText = it }
                Caption(
                    "Reads the answer of the masternode with this pro_tx_hash; no key is " +
                        "needed and no vote can be cast. The pro_tx_hash goes in wire order, " +
                        "the reverse of the hex Core displays.",
                )
            }
        }

        if (BuildConfig.DEBUG) {
            // Debug-only raw credential entry, as Swift gates it behind `#if DEBUG`.
            ToggleRow(
                label = "Use typed test credentials (debug only)",
                checked = useTypedCredentials,
                enabled = enabled,
                testTag = "$testTagPrefix.useTypedCredentials",
                onCheckedChange = { isOn ->
                    useTypedCredentials = isOn
                    if (isOn) useTypedProTxHash = false
                    if (!isOn) votingKeyText = ""
                },
            )
            if (useTypedCredentials) {
                ProTxHashField(proTxHashText, enabled, testTagPrefix) { proTxHashText = it }
                OutlinedTextField(
                    value = votingKeyText,
                    onValueChange = { votingKeyText = it },
                    label = { Text("voting private key (64 hex chars)") },
                    singleLine = true,
                    enabled = enabled,
                    textStyle = MaterialTheme.typography.bodySmall
                        .copy(fontFamily = FontFamily.Monospace),
                    // KeyboardType.Password only configures the keyboard; the
                    // transformation masks what the field shows.
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                    modifier = Modifier.fillMaxWidth().testTag("$testTagPrefix.votingKey"),
                )
                Caption(
                    "Debug builds only. The pro_tx_hash goes in wire order, the reverse of " +
                        "the hex Core displays.",
                )
            }
        }

        Caption(
            "When you vote with a loaded identity, the signer uses its stored voting key. " +
                "You don't need to enter the private key here.",
        )
    }
}

@Composable
private fun ToggleRow(
    label: String,
    checked: Boolean,
    enabled: Boolean,
    testTag: String,
    onCheckedChange: (Boolean) -> Unit,
) {
    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            enabled = enabled,
            modifier = Modifier.testTag(testTag),
        )
    }
}

@Composable
private fun ProTxHashField(
    value: String,
    enabled: Boolean,
    testTagPrefix: String,
    onValueChange: (String) -> Unit,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text("pro_tx_hash, wire order (64 hex chars)") },
        singleLine = true,
        enabled = enabled,
        textStyle = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace),
        modifier = Modifier.fillMaxWidth().testTag("$testTagPrefix.proTxHash"),
    )
}

@Composable
private fun Caption(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}
