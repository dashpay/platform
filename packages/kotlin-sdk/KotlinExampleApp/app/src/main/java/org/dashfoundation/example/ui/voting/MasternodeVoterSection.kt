package org.dashfoundation.example.ui.voting

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
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
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.flow.map
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyNotStoredException
import org.dashfoundation.dashsdk.voting.MasternodeRecordedVotingKey
import org.dashfoundation.dashsdk.voting.MasternodeRecordedVotingKeyUse
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyAddress
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyCheck
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyRecords
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyRules
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeySlot
import org.dashfoundation.dashsdk.voting.MasternodeVotingSigner
import org.dashfoundation.dashsdk.voting.recordedVotingKey
import org.dashfoundation.dashsdk.voting.storedVotingKeyAddress
import org.dashfoundation.dashsdk.voting.storedVotingKeyUse
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
     * Identity), and the identifier of the stored key the section decided on
     * ([votingKeyIdentifier]): the vote signs with that key only, so a key replaced after the
     * pick is never used unchecked. The key stays in `WalletStorage` and signs through the
     * wallet manager's signer.
     */
    class StoredKey(
        identityId: ByteArray,
        val votingKeyIdentifier: String,
        /** The record revision the section cleared the key at. */
        val clearedRevision: Long = 0,
        /** The record the section cleared the key on. */
        val clearedRecord: MasternodeRecordedVotingKey? = null,
    ) : MasternodeVoter {
        private val id = identityId.copyOf()
        val identityId: ByteArray get() = id.copyOf()
        override val canCast: Boolean get() = true
        override val wireOrderProTxHash: ByteArray get() = wireOrderProTxHash(id)
        override fun equals(other: Any?): Boolean =
            other is StoredKey && id.contentEquals(other.id) && votingKeyIdentifier == other.votingKeyIdentifier &&
                clearedRevision == other.clearedRevision && clearedRecord == other.clearedRecord
        override fun hashCode(): Int =
            (31 * id.contentHashCode() + votingKeyIdentifier.hashCode()) * 31 + clearedRevision.hashCode()
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
internal fun voterLabel(
    identity: IdentityEntity,
    candidates: List<IdentityEntity>,
    decision: StoredVoterDecision? = null,
): String {
    val name = voterDisplayName(identity)
    val shared = candidates.any {
        !it.identityId.contentEquals(identity.identityId) && voterDisplayName(it) == name
    }
    val suffix = if (shared) ", ${identity.identityId.toHex().take(8)}" else ""
    // A Platform refusal and a key this device cannot read are told apart.
    val note = when (decision) {
        StoredVoterDecision.REFUSED -> ", voting key does not match"
        StoredVoterDecision.UNREADABLE -> ", voting key cannot be read"
        else -> ""
    }
    return "$name (${identity.identityType}$suffix$note)"
}

/**
 * The identity the picker shows: [selectedIdentityHex] while it is still a candidate, else
 * the first candidate whose voting key may vote ([isRefused] false), else the first, null
 * when there is none (Swift `selectDefaultIfNeeded`).
 */
internal fun effectiveVoterSelection(
    selectedIdentityHex: String?,
    candidates: List<IdentityEntity>,
    isRefused: (IdentityEntity) -> Boolean = { false },
): IdentityEntity? =
    candidates.firstOrNull { it.identityId.toHex() == selectedIdentityHex }
        ?: candidates.firstOrNull { !isRefused(it) }
        ?: candidates.firstOrNull()

/**
 * What the voter section reads of a candidate's stored voting key: the identifier the row
 * names and the check recorded for it. Compared by value, unlike [IdentityEntity] (equal by
 * id), so a new record recomposes the section.
 */
internal data class VoterKeyState(
    val identifier: String?,
    val storedAddress: MasternodeVotingKeyAddress?,
    val recorded: MasternodeRecordedVotingKey?,
    /** The row's `votingKeyCheckRevision`: how many checks were recorded on it. */
    val revision: Long = 0,
)

/** [VoterKeyState] of [identity]'s row. */
internal fun voterKeyState(identity: IdentityEntity): VoterKeyState = VoterKeyState(
    identifier = identity.votingPrivateKeyIdentifier,
    storedAddress = identity.storedVotingKeyAddress,
    recorded = identity.recordedVotingKey,
    revision = identity.votingKeyCheckRevision ?: 0,
)

/**
 * A candidate row with its key state, from ONE Room snapshot, so a candidate is never judged
 * against a record from another moment. Equal by value through [state].
 */
internal data class VoterRow(val identity: IdentityEntity, val state: VoterKeyState)

/** [VoterRow]s of [rows] (one Room emission). */
internal fun voterRows(rows: List<IdentityEntity>): List<VoterRow> =
    rows.map { VoterRow(it, voterKeyState(it)) }

/**
 * What this dialog knows of an identity's stored key, [checkedIdentifier]: [known] is what Rust
 * said to record for it from a check made here; [savedRevision] the row's revision once that
 * record was written (null when it could not be written); and [finished] whether a check of it
 * has finished since it was last picked. Picking the identity again starts a new check with
 * [finished] false. How [known] counts against the row's record is [knownOverride]'s rule.
 */
internal data class PickedKeyCheckState(
    val checkedIdentifier: String,
    val known: MasternodeRecordedVotingKey? = null,
    val finished: Boolean = true,
    val savedRevision: Long? = null,
    /**
     * The row's revision the finished check was made against (or wrote). A finished check
     * counts only while the row is at that revision or earlier: a record written since (by
     * another check or a load) needs a check of its own (Swift, after #6145).
     */
    val checkedRevision: Long = 0,
)

/**
 * The record this dialog learned that the row at [revision] does not show yet, else null. A
 * record that could not be saved stands for the dialog's life (Room will never show it); a saved
 * one stands until the row's revision reaches the one its write left, after which Room, holding
 * that write or a later one, decides. A record learned for another key never counts.
 */
internal fun knownOverride(
    identifier: String?,
    revision: Long,
    picked: PickedKeyCheckState?,
): MasternodeRecordedVotingKey? {
    val sameKey = picked?.takeIf { it.checkedIdentifier == identifier } ?: return null
    val known = sameKey.known ?: return null
    val saved = sameKey.savedRevision ?: return known
    return if (revision < saved) known else null
}

/** The record that speaks for [state]'s stored key: [knownOverride], else the row's. */
internal fun effectiveRecord(state: VoterKeyState, picked: PickedKeyCheckState?): MasternodeRecordedVotingKey? =
    knownOverride(state.identifier, state.revision, picked) ?: state.recorded

/** Whether a stored key may be published as the voter, and if not, why. */
internal enum class StoredVoterDecision {
    /** The key may vote now. */
    CASTABLE,

    /** Rust says the key may not vote (a refusal recorded for it). */
    REFUSED,

    /** Rust says to check the key before use, and no check for it has finished yet. */
    AWAITING_CHECK,

    /** The stored identifier cannot be read, so no key can be found to sign. */
    UNREADABLE,
}

/**
 * The decision for a candidate's stored key, as Rust decides it
 * (`MasternodeVotingKeyRules.use`) from the record that applies: what this dialog learned for
 * the same key ([picked]'s [PickedKeyCheckState.known]) when it learned something, else the
 * row's. The stored key's address comes from its identifier, so no key is read. A key Rust says
 * to check first is castable only once a check of that same key finished since it was picked;
 * an unanswered check still lets it vote, as Swift does.
 */
internal fun storedVoterDecision(
    state: VoterKeyState?,
    picked: PickedKeyCheckState?,
    rules: MasternodeVotingKeyRules,
): StoredVoterDecision {
    val identifier = state?.identifier ?: return StoredVoterDecision.UNREADABLE
    val address = state.storedAddress ?: return StoredVoterDecision.UNREADABLE
    val sameKey = picked?.takeIf { it.checkedIdentifier == identifier }
    val keyUse = rules.use(effectiveRecord(state, picked), address)
    return when {
        !keyUse.canVote -> StoredVoterDecision.REFUSED
        keyUse.checkBeforeUse && !checkFinishedFor(state, sameKey) -> StoredVoterDecision.AWAITING_CHECK
        else -> StoredVoterDecision.CASTABLE
    }
}

/**
 * Whether a check of [state]'s key finished since it was picked, against the record the row
 * holds now: one made against an older revision does not count.
 */
internal fun checkFinishedFor(state: VoterKeyState, picked: PickedKeyCheckState?): Boolean =
    picked != null && picked.checkedIdentifier == state.identifier && picked.finished &&
        (picked.checkedRevision >= state.revision || sameVerdict(state.recorded, picked.known))

/**
 * Whether two records say the same thing about the same key: the same verdict for the same
 * voting address (a message may differ). A newer write of the same verdict (another check that
 * was also inconclusive) is not a new record to check, so two open pickers settle.
 */
internal fun sameVerdict(a: MasternodeRecordedVotingKey?, b: MasternodeRecordedVotingKey?): Boolean =
    a?.votingAddress == b?.votingAddress && a?.check?.let { it::class } == b?.check?.let { it::class }

/**
 * Whether the stored key's row at [rowRevision] with [rowRecord] still matches what the
 * section cleared the [voter] at: the same revision or earlier, or the same verdict. When Rust
 * says to check the key before use and the row moved past that, the vote waits for a new check.
 */
internal fun clearedRecordStillHolds(
    voter: MasternodeVoter.StoredKey,
    rowRevision: Long,
    rowRecord: MasternodeRecordedVotingKey?,
): Boolean = rowRevision <= voter.clearedRevision || sameVerdict(rowRecord, voter.clearedRecord)

/**
 * The picker's state for [identifier] after a check of it finished with [result], started
 * against the row at [revisionAtStart]; null leaves it as it was ([PickedVotingKeyCheck.Dropped],
 * [PickedVotingKeyCheck.NotResolved]). What Rust said to record, or what the helper's own read
 * found, stands until Room delivers that revision; a refusal that could not be saved stands for
 * good, so a refusal is never lost in between.
 */
internal fun nextPickedState(
    previous: PickedKeyCheckState?,
    identifier: String,
    result: PickedVotingKeyCheck,
    revisionAtStart: Long,
): PickedKeyCheckState? = when (result) {
    is PickedVotingKeyCheck.Failed -> if (result.unsaved != null) {
        PickedKeyCheckState(identifier, result.unsaved, savedRevision = null, checkedRevision = revisionAtStart)
    } else {
        (previous ?: PickedKeyCheckState(identifier)).copy(finished = true, checkedRevision = revisionAtStart)
    }
    is PickedVotingKeyCheck.Recorded -> PickedKeyCheckState(
        identifier, result.record, savedRevision = result.revision, checkedRevision = result.revision,
    )
    is PickedVotingKeyCheck.NotNeeded -> if (previous?.known != null && previous.savedRevision == null) {
        previous.copy(finished = true, checkedRevision = maxOf(revisionAtStart, result.revision))
    } else {
        PickedKeyCheckState(
            identifier, result.record, savedRevision = result.revision, checkedRevision = result.revision,
        )
    }
    else -> null
}

/**
 * Whether the stored voting key is known not to vote: refused by Rust, or unreadable. Used for
 * the label and the default pick (Swift `MasternodeVoterSection.isRefused`, which reads no
 * address and takes the record as it is).
 */
internal fun isVotingKeyRefused(state: VoterKeyState?, rules: MasternodeVotingKeyRules): Boolean =
    storedVoterDecision(state, null, rules).let {
        it == StoredVoterDecision.REFUSED || it == StoredVoterDecision.UNREADABLE
    }

/** How the check of the picked identity's stored voting key went. */
internal sealed interface PickedVotingKeyCheck {
    /**
     * Rust says the key needs no check before use, from [record], the record the helper's own
     * read found (or the dialog's override), at [revision]: the picker holds it until Room
     * delivers that revision, so an older displayed record never stands in for it.
     */
    data class NotNeeded(
        val record: MasternodeRecordedVotingKey? = null,
        val revision: Long = 0,
    ) : PickedVotingKeyCheck

    /**
     * Checked and recorded what Rust said to record: [record], null when Rust said to keep the
     * record as it was.
     */
    data class Recorded(
        val check: MasternodeVotingKeyCheck,
        val record: MasternodeRecordedVotingKey? = null,
        /** The row's revision after the write. */
        val revision: Long = 0,
    ) : PickedVotingKeyCheck

    /**
     * Checked, but the answer was dropped: the row is gone, moved network or names another
     * key by now, or the check was superseded.
     */
    data object Dropped : PickedVotingKeyCheck

    /**
     * The check could not run, or its answer could not be saved; [message] says why. For an
     * answer that could not be saved, [unsaved] is what Rust said to record for it, which the
     * section applies until it is saved, so a refusal is never lost to a failed write.
     */
    data class Failed(
        val message: String,
        val unsaved: MasternodeRecordedVotingKey? = null,
    ) : PickedVotingKeyCheck
}

/**
 * Check the picked identity's stored voting key against Platform when Rust says to check it
 * before use, and record on its row what Rust says to record. Port of Swift
 * `MasternodeVoterSection.checkSelectedVotingKeyIfNeeded`.
 *
 * The stored key's voting address comes from its identifier, so no key is read and the key is
 * never made signable. The answer is recorded only while the row still names [identifier], in
 * one transaction ([MasternodeVotingKeyRecords.record]): an answer about a key replaced during
 * the check is dropped. Nothing is written before the answer, so a check cut short by process
 * death leaves the row unchecked, and it is checked again when picked.
 *
 * @param picked what this dialog already learned for the key, which Rust weighs in place of
 *   the row's record as this read finds it ([knownOverride]): a refusal whose write failed is
 *   never checked away by a retry that learns nothing, and a saved result the row has since
 *   moved past does not hide the row's newer record.
 * @param check the Platform check (`Voting.checkMasternodeVotingKey`) of the wire-order
 *   pro_tx_hash and the address.
 * @param stillCurrent whether the answer may still be recorded: the same SDK and network.
 */
internal suspend fun checkPickedVotingKeyIfNeeded(
    database: DashDatabase,
    identityId: ByteArray,
    networkRaw: Int,
    identifier: String,
    rules: MasternodeVotingKeyRules,
    picked: PickedKeyCheckState? = null,
    check: suspend (wireOrderProTxHash: ByteArray, address: MasternodeVotingKeyAddress) -> MasternodeVotingKeyCheck,
    stillCurrent: () -> Boolean,
): PickedVotingKeyCheck {
    val row = database.identityDao().getByIdentityId(identityId)
        ?.takeIf { it.networkRaw == networkRaw && it.votingPrivateKeyIdentifier == identifier }
        ?: return PickedVotingKeyCheck.Dropped
    // An identifier this build cannot read names no key to check; the picker shows it as such.
    val address = row.storedVotingKeyAddress ?: return PickedVotingKeyCheck.Dropped
    val recorded = knownOverride(identifier, row.votingKeyCheckRevision ?: 0, picked)
        ?.takeIf { it.votingAddress == address }
        ?: row.recordedVotingKey
    val rowRevision = row.votingKeyCheckRevision ?: 0
    if (!rules.use(recorded, address).checkBeforeUse) {
        return PickedVotingKeyCheck.NotNeeded(recorded, rowRevision)
    }
    val answer = try {
        check(wireOrderProTxHash(identityId), address)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        // Nothing was learned about the key; the row is left as it was.
        return PickedVotingKeyCheck.Failed("The voting key could not be checked: ${e.message}")
    }
    // A superseded check (the pick changed) is not recorded: it could finish out of order.
    currentCoroutineContext().ensureActive()
    if (!stillCurrent()) return PickedVotingKeyCheck.Dropped
    val written = try {
        MasternodeVotingKeyRecords.recordWithRevision(database, identityId, networkRaw, identifier, answer, rules)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        // The answer stands even though it was not saved: what Rust says to record for it,
        // against the record read before the check, is held by the section instead.
        val unsaved = rules.outcome(answer, address, MasternodeVotingKeySlot.SameKey(recorded)).record
        return PickedVotingKeyCheck.Failed("The voting key check could not be saved: ${e.message}", unsaved)
    }
    return if (written == null) {
        PickedVotingKeyCheck.Dropped
    } else {
        // The record the row holds after the write: the one written, or the one Rust kept.
        PickedVotingKeyCheck.Recorded(answer, written.record, written.revision)
    }
}

/**
 * A 32-byte value typed as exactly 64 hex characters (surrounding whitespace ignored);
 * null for anything else.
 */
internal fun decodeTyped32Bytes(text: String): ByteArray? {
    val trimmed = text.trim()
    // ASCII hex only: Char.isDigit() also accepts other scripts' digits.
    if (trimmed.length != 64 || !trimmed.all { it in '0'..'9' || it in 'a'..'f' || it in 'A'..'F' }) {
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
    selectedCastable: Boolean = true,
    clearedRevision: Long = 0,
    clearedRecord: MasternodeRecordedVotingKey? = null,
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
    // A stored key is published only once it may vote now (see [storedVoterDecision]).
    if (!selectedCastable) return null
    val identity = selected ?: return null
    val identifier = identity.votingPrivateKeyIdentifier ?: return null
    return MasternodeVoter.StoredKey(identity.identityId, identifier, clearedRevision, clearedRecord)
}

// ── Casting ──────────────────────────────────────────────────────────

/** Why a vote could not be cast before anything reached Platform. */
class MasternodeVoteCallException(message: String) : Exception(message)

/** Shown when a picked identity has no readable stored voting key. */
internal const val NO_STORED_VOTING_KEY_MESSAGE =
    "No voting key is stored for this masternode identity. Load it again with its voting " +
        "private key."

/** Shown under the picker for a stored voting key identifier this build cannot read. */
internal const val UNREADABLE_VOTING_KEY_IDENTIFIER_MESSAGE =
    "This identity's stored voting key cannot be found on this device, so it cannot vote. " +
        "Load the identity again with its voting key under Identities, Load Identity."

/** Shown when the picked identity's voting key was replaced after the section decided on it. */
internal const val VOTING_KEY_CHANGED_SINCE_PICK_MESSAGE =
    "This masternode's voting key changed since it was picked. Pick it again to vote."

/** Shown when a refusal of the picked key was recorded after the section decided on it. */
internal const val VOTING_KEY_REFUSED_SINCE_PICK_MESSAGE =
    "Platform refused this masternode's voting key since it was picked, so the vote was not " +
        "cast. Load the identity again with its voting key under Identities, Load Identity."

/** Shown when a newer record of the picked key, to be checked first, arrived after the pick. */
internal const val VOTING_KEY_RECHECK_SINCE_PICK_MESSAGE =
    "This masternode's voting key has a newer record that must be checked first. Wait for the " +
        "check under the picker, then vote again."

/** Shown when there is no wallet manager whose signer could sign the vote. */
internal const val NO_WALLET_MANAGER_MESSAGE =
    "No wallet manager is active on this network, so the stored voting key cannot sign. " +
        "Open the Wallets tab and wait for the wallets to load, then vote again."

/**
 * One masternode vote, as each dialog's own SDK call makes it. Port of Swift
 * `MasternodeVoteCall`: [run] picks how the vote is signed, so the dialogs do not repeat
 * the dispatch.
 *
 * [withSigner] casts with a signer: the wire-order pro_tx_hash, the vote's
 * [MasternodeVotingSigner], which the lambda passes to the SDK's signer overload, and the
 * active wallet manager. [run] calls
 * it inside [PlatformWalletManager.withStoredMasternodeVotingKey], which builds that signer
 * for this vote only, holding just the identity's stored voting key, and frees it after.
 * The manager is read when [run] is called, never held in composition.
 */
class MasternodeVoteCall(
    private val withSigner: suspend (
        proTxHash: ByteArray,
        signer: MasternodeVotingSigner,
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
     * Cast as [voter]. A stored key signs through the active wallet manager's signer, with the
     * key the section decided on ([MasternodeVoter.StoredKey.votingKeyIdentifier]): when the
     * identity's row names another key by now, the vote is refused rather than signed with a
     * key the section never judged, and the manager refuses it when no readable key is stored.
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
                if (identifier != voter.votingKeyIdentifier) {
                    throw MasternodeVoteCallException(VOTING_KEY_CHANGED_SINCE_PICK_MESSAGE)
                }
                // A refusal recorded since the pick is honoured before the signer is opened:
                // Rust decides from the row as read now.
                val keyUse = identity.storedVotingKeyUse()
                if (keyUse?.canVote == false) {
                    throw MasternodeVoteCallException(VOTING_KEY_REFUSED_SINCE_PICK_MESSAGE)
                }
                // A newer record that Rust says to check first was not cleared by the section.
                if (keyUse?.checkBeforeUse == true && !clearedRecordStillHolds(
                        voter, identity.votingKeyCheckRevision ?: 0, identity.recordedVotingKey,
                    )
                ) {
                    throw MasternodeVoteCallException(VOTING_KEY_RECHECK_SINCE_PICK_MESSAGE)
                }
                val manager = container.walletManagerStore.activeManager.value
                    ?: throw MasternodeVoteCallException(NO_WALLET_MANAGER_MESSAGE)
                try {
                    manager.withStoredMasternodeVotingKey(identifier) { signer ->
                        withSigner(voter.wireOrderProTxHash, signer, manager)
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
 *
 * Rust decides from the check recorded on a candidate, with the address of the key it was made
 * of, whether it may vote and whether its key is checked when picked
 * (`MasternodeVotingKeyRules.use`). A candidate that may not vote is labelled and never
 * published. A picked candidate whose key is to be checked first is checked once when picked
 * (`<prefix>.voterKeyCheck` shows how it went), and what Rust says to record is written on its
 * row ([checkPickedVotingKeyIfNeeded]).
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
    // The candidates and their stored keys and records from ONE Room snapshot, compared by
    // value, so a new record (this section's own check, or a load elsewhere) recomposes the
    // section and a candidate is never judged against a record from another moment.
    val rows by remember(network) {
        container.database.identityDao().observeByNetwork(network.ffiValue).map(::voterRows)
    }.collectAsStateWithLifecycle(initialValue = emptyList())
    val candidates = remember(rows, network) { votingCandidates(rows.map { it.identity }, network.ffiValue) }
    val keyStates = remember(rows) { rows.associate { it.identity.identityId.toHex() to it.state } }
    val rules = MasternodeVotingKeyRules.Native
    fun isRefused(identity: IdentityEntity): Boolean =
        isVotingKeyRefused(keyStates[identity.identityId.toHex()], rules)
    // The identity whose stored voting key is being checked right now; why the last check of
    // a picked identity did not happen or was not saved, by identity hex; and what this dialog
    // knows of each identity's key from a finished check, by identity hex.
    var checkingIdentityHex by remember { mutableStateOf<String?>(null) }
    var checkFailure by remember { mutableStateOf<Pair<String, String>?>(null) }
    var pickedChecks by remember { mutableStateOf<Map<String, PickedKeyCheckState>>(emptyMap()) }
    // Counts explicit picks, so picking the shown identity again starts its check again.
    var pickRequest by remember { mutableIntStateOf(0) }

    var selectedIdentityHex by rememberSaveable { mutableStateOf<String?>(null) }
    var useTypedProTxHash by rememberSaveable { mutableStateOf(false) }
    // Shared by the typed pro_tx_hash and, in debug builds, the typed credentials; their
    // toggles are mutually exclusive.
    var proTxHashText by rememberSaveable { mutableStateOf("") }
    // Never saved into the instance state bundle: a key typed here stays in memory only.
    var useTypedCredentials by remember { mutableStateOf(false) }
    var votingKeyText by remember { mutableStateOf("") }

    val selected = effectiveVoterSelection(selectedIdentityHex, candidates, ::isRefused)
    val selectedHex = selected?.identityId?.toHex()
    val selectedState = selectedHex?.let { keyStates[it] }
    val selectedPicked = selectedHex?.let { pickedChecks[it] }
        ?.takeIf { it.checkedIdentifier == selectedState?.identifier }
    // The default pick is kept once shown: a refusal that arrives later labels it and keeps it
    // from voting, but never swaps the vote to another masternode the operator did not pick.
    if (selectedIdentityHex == null && selectedHex != null) {
        SideEffect { selectedIdentityHex = selectedHex }
    }
    val usesTypedInput = (BuildConfig.DEBUG && useTypedCredentials) ||
        (allowsTypedProTxHash && useTypedProTxHash)

    // The voter the inputs name now. Event handlers call [publish] right after their state
    // writes, so the dialog's voter changes in the same event as the mode and inputs: it
    // never casts with a voter the section no longer shows.
    fun currentVoter(): MasternodeVoter? {
        val pick = effectiveVoterSelection(selectedIdentityHex, candidates, ::isRefused)
        val hex = pick?.identityId?.toHex()
        val state = hex?.let { keyStates[it] }
        val picked = hex?.let { pickedChecks[it] }
        return publishVoter(
        selected = pick,
        selectedCastable = pick != null && storedVoterDecision(state, picked, rules) == StoredVoterDecision.CASTABLE,
        // What the vote carries of the record it was cleared on, for the check at vote time.
        clearedRevision = maxOf(state?.revision ?: 0, picked?.takeIf { it.checkedIdentifier == state?.identifier }?.checkedRevision ?: 0),
        clearedRecord = state?.let { effectiveRecord(it, picked) },
        allowsTypedProTxHash = allowsTypedProTxHash,
        useTypedProTxHash = useTypedProTxHash,
        proTxHashText = proTxHashText,
        debugBuild = BuildConfig.DEBUG,
        useTypedCredentials = useTypedCredentials,
        votingKeyText = votingKeyText,
    )
    }
    fun publish() = onVoterChange(currentVoter())
    val voter = currentVoter()
    // Changes no handler makes (the identity list, the first composition) are published by
    // SideEffect, which runs when the composition that shows them is applied, in that same
    // frame and before any further input is handled, so a tap can never cast with a voter
    // the section stopped showing. Publishing an unchanged voter is a no-op for the dialog's
    // state (voters compare by value).
    SideEffect { onVoterChange(voter) }

    // A record written since the picked key's check finished (another check, or a load) needs
    // a check of its own when Rust says so: a new check request, as Swift reruns its check
    // when the record changes.
    var recheckRequest by remember { mutableIntStateOf(0) }
    LaunchedEffect(selectedState?.revision, selectedPicked) {
        val state = selectedState ?: return@LaunchedEffect
        val picked = selectedPicked ?: return@LaunchedEffect
        if (picked.finished && picked.checkedRevision < state.revision && !sameVerdict(state.recorded, picked.known)) {
            recheckRequest++
        }
    }

    // Once per pick of an identity and its stored key: the key is checked when Rust says to
    // check it before use. The record it writes recomposes the section through [keyStates].
    LaunchedEffect(selectedHex, selectedState?.identifier, network, pickRequest, recheckRequest) {
        val identity = selected ?: return@LaunchedEffect
        val identifier = selectedState?.identifier ?: return@LaunchedEffect
        // An unreadable identifier names no key to check; the status line says so.
        if (selectedState.storedAddress == null) return@LaunchedEffect
        val sdk = appState.sdk.value ?: return@LaunchedEffect
        val hex = identity.identityId.toHex()
        val networkAtStart = network
        // A new pick starts a new check: until it finishes, the key is not castable on the
        // strength of an earlier check. What this dialog learned of the same key is kept.
        val previous = pickedChecks[hex]?.takeIf { it.checkedIdentifier == identifier }
        // The revision of the row this check is made against.
        val revisionAtStart = selectedState.revision
        pickedChecks = if (previous != null) {
            pickedChecks + (hex to previous.copy(finished = false))
        } else {
            pickedChecks - hex
        }
        checkingIdentityHex = hex
        val result = try {
            checkPickedVotingKeyIfNeeded(
                database = container.database,
                identityId = identity.identityId,
                networkRaw = networkAtStart.ffiValue,
                identifier = identifier,
                rules = rules,
                picked = previous,
                check = { proTxHash, address -> sdk.voting.checkMasternodeVotingKey(proTxHash, address) },
                stillCurrent = {
                    appState.sdk.value === sdk && appState.currentNetwork.value == networkAtStart
                },
            )
        } finally {
            if (checkingIdentityHex == hex) checkingIdentityHex = null
        }
        when (result) {
            is PickedVotingKeyCheck.Failed -> checkFailure = hex to result.message
            is PickedVotingKeyCheck.Recorded, is PickedVotingKeyCheck.NotNeeded ->
                if (checkFailure?.first == hex) checkFailure = null
            else -> Unit
        }
        nextPickedState(previous, identifier, result, revisionAtStart)?.let {
            pickedChecks = pickedChecks + (hex to it)
        }
    }

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
                optionLabel = {
                    val hex = it.identityId.toHex()
                    voterLabel(it, candidates, storedVoterDecision(keyStates[hex], pickedChecks[hex], rules))
                },
                testTag = "$testTagPrefix.voter",
                enabled = enabled && !usesTypedInput,
                onSelected = {
                    val hex = it.identityId.toHex()
                    // Every explicit pick, of this identity again too, is a new pick: its
                    // earlier check no longer releases it, and the check runs again when Rust
                    // says to check it before use (a retry of an unanswered check).
                    pickedChecks[hex]?.let { picked ->
                        pickedChecks = pickedChecks + (hex to picked.copy(finished = false))
                    }
                    pickRequest++
                    selectedIdentityHex = hex
                    publish()
                },
            )
            if (!usesTypedInput) {
                // A record that could not be saved stands in for the row's until it is.
                val shownState = selectedState?.let { state ->
                    state.copy(recorded = effectiveRecord(state, selectedPicked))
                }
                VotingKeyCheckStatus(
                    state = shownState,
                    keyUse = shownState?.storedAddress?.let { rules.use(shownState.recorded, it) },
                    isChecking = checkingIdentityHex == selectedHex,
                    failure = checkFailure?.takeIf { it.first == selectedHex }?.second,
                    modifier = Modifier.testTag("$testTagPrefix.voterKeyCheck"),
                )
            }
        }

        if (allowsTypedProTxHash) {
            ToggleRow(
                label = "Read another masternode's answer",
                checked = useTypedProTxHash,
                enabled = enabled,
                testTag = "$testTagPrefix.useTypedProTxHash",
                onCheckedChange = { isOn ->
                    useTypedProTxHash = isOn
                    if (isOn) {
                        // The read-only lookup keeps the shared pro_tx_hash, never the key.
                        useTypedCredentials = false
                        votingKeyText = ""
                    }
                    publish()
                },
            )
            if (useTypedProTxHash) {
                ProTxHashField(proTxHashText, enabled, testTagPrefix) {
                    proTxHashText = it
                    publish()
                }
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
                    publish()
                },
            )
            if (useTypedCredentials) {
                ProTxHashField(proTxHashText, enabled, testTagPrefix) {
                    proTxHashText = it
                    publish()
                }
                OutlinedTextField(
                    value = votingKeyText,
                    onValueChange = {
                        votingKeyText = it
                        publish()
                    },
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

/**
 * How the check of the picked identity's stored voting key went, under the picker (Swift
 * `VotingKeyCheckStatus` in `MasternodeVoterSection.swift`). Nothing for a key Platform
 * accepted.
 */
@Composable
private fun VotingKeyCheckStatus(
    state: VoterKeyState?,
    keyUse: MasternodeRecordedVotingKeyUse?,
    isChecking: Boolean,
    failure: String?,
    modifier: Modifier = Modifier,
) {
    when (val text = votingKeyCheckStatusText(state, keyUse, isChecking, failure)) {
        null -> Unit
        else -> Row(
            modifier = modifier,
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (isChecking) CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
            Text(
                text.first,
                style = MaterialTheme.typography.bodySmall,
                color = if (text.second) {
                    MaterialTheme.colorScheme.error
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
            )
        }
    }
}

/**
 * The status line under the picker and whether it reads as an error, or null for none. Which
 * record speaks for the stored key is Rust's word ([keyUse], `MasternodeVotingKeyRules.use`):
 * a refusal shows only when Rust says the key may not vote, and an unanswered check only when
 * Rust says to check it again.
 */
internal fun votingKeyCheckStatusText(
    state: VoterKeyState?,
    keyUse: MasternodeRecordedVotingKeyUse?,
    isChecking: Boolean,
    failure: String?,
): Pair<String, Boolean>? {
    if (isChecking) return "Checking the voting key..." to false
    if (state?.identifier != null && state.storedAddress == null) {
        return UNREADABLE_VOTING_KEY_IDENTIFIER_MESSAGE to true
    }
    val recorded = state?.recorded?.check
    return when {
        keyUse?.canVote == false && recorded is MasternodeVotingKeyCheck.Invalid ->
            "This voting key cannot vote for this masternode: ${recorded.message} Load the " +
                "identity again with its voting key under Identities, Load Identity." to true
        failure != null -> "$failure You can still vote with it." to false
        keyUse?.checkBeforeUse == true && recorded is MasternodeVotingKeyCheck.Undetermined ->
            "${recorded.message} You can still vote with it; it is checked again when you " +
                "pick this identity." to false
        else -> null
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
