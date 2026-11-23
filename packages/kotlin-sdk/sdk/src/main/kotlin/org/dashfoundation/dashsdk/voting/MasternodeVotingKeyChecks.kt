package org.dashfoundation.dashsdk.voting

import androidx.room.withTransaction
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.MasternodeVotingKeyNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.hexToByteArray
import org.dashfoundation.dashsdk.persistence.toHex
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore

// Checking a masternode voting key against Platform, and what a host does with the answer.
// Port of Swift `SDK+MasternodeVotingKeyCheck.swift` (SwiftDashSDK/Voting/) and of the voting
// key check fields of `PersistentIdentity.swift`.
//
// rs-sdk decides everything (`masternode_vote_keys.rs`: `check_masternode_voting_key`,
// `voting_key_check_outcome`, `recorded_voting_key_use`); rs-sdk-ffi words the verdict. This
// file marshals arguments in and results out through `MasternodeVotingKeyNative`, and persists
// on the identity row what Rust returns.

/**
 * Whether a voting key can vote for a masternode, as Platform's state proves it
 * (`org.dashfoundation.dashsdk.queries.Voting.checkMasternodeVotingKey`). Swift
 * `MasternodeVotingKeyCheck`.
 */
sealed interface MasternodeVotingKeyCheck {
    /**
     * The masternode's voter identity holds an enabled voting key for this address, so the key
     * can vote, apart from one history Platform's state cannot rule out (rs-sdk's
     * `VotingKeyCheck`: a masternode that moved its voting address away and back before
     * protocol version 15). Store it as checked.
     */
    data object Valid : MasternodeVotingKeyCheck

    /**
     * Platform proves the key cannot vote for the masternode: the pro_tx_hash is not a
     * masternode, the key is not its voting key, or the key was rotated away. Refuse the key.
     * [message] says which, worded in Rust.
     */
    data class Invalid(val message: String) : MasternodeVotingKeyCheck

    /**
     * The voter identity could not be fetched, so nothing is known about the key. It may be
     * kept, marked unchecked, and checked again later. [message] says why, worded in Rust.
     */
    data class Undetermined(val message: String) : MasternodeVotingKeyCheck
}

/**
 * A masternode voting address: the 20-byte hash160 of the voting public key. Built only from
 * 20 bytes, so the voting key rules always receive a whole address. Swift
 * `MasternodeVotingKeyAddress`.
 */
class MasternodeVotingKeyAddress private constructor(private val value: ByteArray) {
    /** A copy of the 20 address bytes. */
    val bytes: ByteArray get() = value.copyOf()

    /** Lowercase hex, as voting key identifiers name it. */
    val hex: String get() = value.toHex()

    override fun equals(other: Any?): Boolean =
        other is MasternodeVotingKeyAddress && value.contentEquals(other.value)

    override fun hashCode(): Int = value.contentHashCode()

    override fun toString(): String = "MasternodeVotingKeyAddress($hex)"

    companion object {
        /** The address in [bytes]; null unless it is exactly 20 bytes. */
        fun of(bytes: ByteArray): MasternodeVotingKeyAddress? =
            if (bytes.size == 20) MasternodeVotingKeyAddress(bytes.copyOf()) else null

        /**
         * The voting address of the key a `MasternodeVotingKeyStore` identifier names. The
         * identifier carries it, so no key is read.
         *
         * @throws IllegalArgumentException when [identifier] is not a voting key identifier.
         */
        fun ofIdentifier(identifier: String): MasternodeVotingKeyAddress =
            MasternodeVotingKeyAddress(MasternodeVotingKeyStore.votingAddressOfIdentifier(identifier))
    }
}

/**
 * A check a host recorded, with the voting address of the key it was made of. The address
 * ties the record to its key: Rust uses a record only for a key with that address. Swift
 * `MasternodeRecordedVotingKey`.
 */
data class MasternodeRecordedVotingKey(
    val check: MasternodeVotingKeyCheck,
    val votingAddress: MasternodeVotingKeyAddress,
)

/**
 * What the host's voting key slot for a masternode holds, compared with the key that was
 * checked. The host reads its identity row to say this; Rust decides what follows from it.
 * Swift `MasternodeVotingKeySlot`.
 */
sealed interface MasternodeVotingKeySlot {
    /** No voting key is stored for the masternode. */
    data object Empty : MasternodeVotingKeySlot

    /** Exactly the checked key is stored, with the record held for it, null when none is. */
    data class SameKey(val recorded: MasternodeRecordedVotingKey?) : MasternodeVotingKeySlot

    /** A different key is stored. */
    data object DifferentKey : MasternodeVotingKeySlot
}

/**
 * What the host does with a voting key once checked, as Rust decides it. Swift
 * `MasternodeVotingKeyCheckOutcome`.
 *
 * @property storeKey whether the host stores the checked key.
 * @property record what the host records for the checked key once it is in the slot, or for
 *   the stored key when it is the same one, made of the checked key: persist it as it is.
 *   Null leaves the record unchanged.
 */
data class MasternodeVotingKeyCheckOutcome(
    val storeKey: Boolean,
    val record: MasternodeRecordedVotingKey?,
)

/**
 * How the host may use a stored voting key, as Rust decides it from the record it holds.
 * Swift `MasternodeRecordedVotingKeyUse`.
 *
 * @property canVote whether the key is offered for voting.
 * @property checkBeforeUse whether the host checks the key before it uses it.
 */
data class MasternodeRecordedVotingKeyUse(
    val canVote: Boolean,
    val checkBeforeUse: Boolean,
)

/**
 * rs-sdk's host rules for a masternode voting key, bridged through JNI. An interface only so
 * JVM tests, which cannot load the native library, can hand the app's flows a fixed answer;
 * production code uses [Native], and nothing else decides.
 */
interface MasternodeVotingKeyRules {
    /**
     * What the host does with the key [check] was made of, with [votingAddress], given what
     * its slot holds (`dash_sdk_masternode_voting_key_check_outcome`; Swift
     * `MasternodeVotingKeyCheck.outcome(votingAddress:slot:)`).
     */
    fun outcome(
        check: MasternodeVotingKeyCheck,
        votingAddress: MasternodeVotingKeyAddress,
        slot: MasternodeVotingKeySlot,
    ): MasternodeVotingKeyCheckOutcome

    /**
     * How a stored key may be used, from [recorded], the record held for the masternode, and
     * [storedVotingAddress], the stored key's address when known
     * (`dash_sdk_masternode_recorded_voting_key_use`; Swift
     * `MasternodeRecordedVotingKeyUse.init(recorded:storedVotingAddress:)`).
     */
    fun use(
        recorded: MasternodeRecordedVotingKey?,
        storedVotingAddress: MasternodeVotingKeyAddress?,
    ): MasternodeRecordedVotingKeyUse

    /**
     * What to record for another key stored for the same masternode as the key [loaded] was
     * made of, with [loadedAddress], given the record held for it ([sibling]); null leaves it
     * unchanged (`dash_sdk_masternode_sibling_voting_key_record`; Swift
     * `MasternodeVotingKeyCheck.siblingRecord(votingAddress:sibling:)`).
     */
    fun siblingRecord(
        loaded: MasternodeVotingKeyCheck,
        loadedAddress: MasternodeVotingKeyAddress,
        sibling: MasternodeRecordedVotingKey?,
    ): MasternodeRecordedVotingKey?

    /** The rules in Rust. Each call loads the native library first. */
    object Native : MasternodeVotingKeyRules {
        override fun outcome(
            check: MasternodeVotingKeyCheck,
            votingAddress: MasternodeVotingKeyAddress,
            slot: MasternodeVotingKeySlot,
        ): MasternodeVotingKeyCheckOutcome {
            NativeLoader.ensureLoaded()
            val recorded = (slot as? MasternodeVotingKeySlot.SameKey)?.recorded
            val json = mapNativeErrors {
                MasternodeVotingKeyNative.votingKeyCheckOutcome(
                    MasternodeVotingKeyCheckCodec.verdictOf(check),
                    votingAddress.bytes,
                    MasternodeVotingKeyCheckCodec.slotOf(slot),
                    MasternodeVotingKeyCheckCodec.recordedCheckOf(recorded?.check),
                    recorded?.votingAddress?.bytes,
                )
            }
            return MasternodeVotingKeyCheckCodec.decodeOutcome(json, preferring = check)
        }

        override fun use(
            recorded: MasternodeRecordedVotingKey?,
            storedVotingAddress: MasternodeVotingKeyAddress?,
        ): MasternodeRecordedVotingKeyUse {
            NativeLoader.ensureLoaded()
            val json = mapNativeErrors {
                MasternodeVotingKeyNative.recordedVotingKeyUse(
                    MasternodeVotingKeyCheckCodec.recordedCheckOf(recorded?.check),
                    recorded?.votingAddress?.bytes,
                    storedVotingAddress?.bytes,
                )
            }
            return MasternodeVotingKeyCheckCodec.decodeUse(json)
        }

        override fun siblingRecord(
            loaded: MasternodeVotingKeyCheck,
            loadedAddress: MasternodeVotingKeyAddress,
            sibling: MasternodeRecordedVotingKey?,
        ): MasternodeRecordedVotingKey? {
            NativeLoader.ensureLoaded()
            val json = mapNativeErrors {
                MasternodeVotingKeyNative.siblingVotingKeyRecord(
                    MasternodeVotingKeyCheckCodec.verdictOf(loaded),
                    loadedAddress.bytes,
                    MasternodeVotingKeyCheckCodec.recordedCheckOf(sibling?.check),
                    sibling?.votingAddress?.bytes,
                )
            }
            return MasternodeVotingKeyCheckCodec.decodeSiblingRecord(json)
        }
    }
}

/**
 * An identity loaded to vote for a masternode (`Voting.loadMasternodeForVoting`). Swift
 * `MasternodeVotingLoadedIdentity` (`SDK+MasternodeVotingLoad.swift`).
 *
 * @property role which identity Rust loaded.
 * @property identityJson the identity as JSON, the shape `Identities.fetch` returns.
 * @property votingKeyCheck the voting key's verdict when one was given; always
 *   [MasternodeVotingKeyCheck.Valid] for [Role.VOTER].
 */
class MasternodeVotingLoadedIdentity(
    val role: Role,
    identityId: ByteArray,
    val identityJson: String,
    val votingKeyCheck: MasternodeVotingKeyCheck?,
) {
    private val id = identityId.copyOf()

    /** A copy of the 32-byte id to store the identity's row under. */
    val identityId: ByteArray get() = id.copyOf()

    /** Which identity Rust loaded for the masternode; [raw] is the stored value (Swift's). */
    enum class Role(val raw: String) {
        /** The owner identity, whose id is the pro_tx_hash in the order Platform names it. */
        OWNER("owner"),

        /**
         * The voter identity of a masternode without an owner identity (a shared masternode):
         * its id is not the pro_tx_hash, so the host keeps the pro_tx_hash beside it.
         */
        VOTER("voter"),
    }
}

/** What `Voting.loadMasternodeForVoting` decided. Swift `MasternodeVotingLoad`. */
sealed interface MasternodeVotingLoad {
    /** An identity to store for the masternode. */
    data class Loaded(val identity: MasternodeVotingLoadedIdentity) : MasternodeVotingLoad

    /**
     * Nothing is loaded; [message] says why, worded in Rust. When the voting key was refused
     * for a voter identity, [votingKeyRefusedFor] is that identity's id: the host records the
     * refusal on a row it already stores for it holding the same key (Rust's
     * `voting_key_check_outcome` with an invalid check), and creates or replaces nothing.
     * Swift `MasternodeVotingLoad.refused(message:votingKeyRefusedFor:)`.
     */
    class Refused(val message: String, votingKeyRefusedFor: ByteArray? = null) : MasternodeVotingLoad {
        private val refusedFor = votingKeyRefusedFor?.copyOf()

        /** A copy of the voter identity id the voting key was refused for, if any. */
        val votingKeyRefusedFor: ByteArray? get() = refusedFor?.copyOf()

        override fun equals(other: Any?): Boolean =
            other is Refused && message == other.message &&
                (refusedFor?.contentEquals(other.refusedFor) ?: (other.refusedFor == null))

        override fun hashCode(): Int = 31 * message.hashCode() + (refusedFor?.contentHashCode() ?: 0)

        override fun toString(): String = "Refused($message)"
    }
}

/**
 * The C enum values and the JSON the JNI exports write (`rs-unified-sdk-jni`'s
 * `masternode_voting_key.rs`), both ways. Internal and pure, so JVM tests cover it.
 */
internal object MasternodeVotingKeyCheckCodec {
    // DashSDKVotingKeyVerdict
    const val VERDICT_VALID = 0
    const val VERDICT_INVALID = 1
    const val VERDICT_UNDETERMINED = 2

    // DashSDKVotingKeySlot
    const val SLOT_EMPTY = 0
    const val SLOT_SAME_KEY = 1
    const val SLOT_DIFFERENT_KEY = 2

    // DashSDKRecordedVotingKeyCheck
    const val RECORDED_NONE = 0
    const val RECORDED_VALID = 1
    const val RECORDED_INVALID = 2
    const val RECORDED_UNDETERMINED = 3

    private val json = Json { ignoreUnknownKeys = true }

    fun verdictOf(check: MasternodeVotingKeyCheck): Int = when (check) {
        MasternodeVotingKeyCheck.Valid -> VERDICT_VALID
        is MasternodeVotingKeyCheck.Invalid -> VERDICT_INVALID
        is MasternodeVotingKeyCheck.Undetermined -> VERDICT_UNDETERMINED
    }

    fun slotOf(slot: MasternodeVotingKeySlot): Int = when (slot) {
        MasternodeVotingKeySlot.Empty -> SLOT_EMPTY
        is MasternodeVotingKeySlot.SameKey -> SLOT_SAME_KEY
        MasternodeVotingKeySlot.DifferentKey -> SLOT_DIFFERENT_KEY
    }

    fun recordedCheckOf(check: MasternodeVotingKeyCheck?): Int = when (check) {
        null -> RECORDED_NONE
        MasternodeVotingKeyCheck.Valid -> RECORDED_VALID
        is MasternodeVotingKeyCheck.Invalid -> RECORDED_INVALID
        is MasternodeVotingKeyCheck.Undetermined -> RECORDED_UNDETERMINED
    }

    /** `{"verdict":0|1|2,"message":string|null}` from `checkVotingKey`. */
    fun decodeCheck(text: String?): MasternodeVotingKeyCheck {
        val root = parse("dash_sdk_masternode_check_voting_key", text)
        // JsonNull is a primitive whose content is null.
        val message = root["message"]?.jsonPrimitive?.contentOrNull
        return when (val verdict = root["verdict"]?.jsonPrimitive?.intOrNull) {
            VERDICT_VALID -> MasternodeVotingKeyCheck.Valid
            VERDICT_INVALID -> MasternodeVotingKeyCheck.Invalid(message.orEmpty())
            VERDICT_UNDETERMINED -> MasternodeVotingKeyCheck.Undetermined(message.orEmpty())
            else -> throw DashSdkError.SerializationError(
                "dash_sdk_masternode_check_voting_key returned an unknown verdict $verdict",
            )
        }
    }

    /**
     * `{"storeKey":bool,"record":null|{"check":1|2|3,"votingAddress":hex}}` from
     * `votingKeyCheckOutcome`. A record of [preferring]'s own kind carries [preferring], so
     * its message is kept (Swift `MasternodeVotingKeyCheckFFI.recordedKey(_:preferring:)`).
     */
    fun decodeOutcome(text: String?, preferring: MasternodeVotingKeyCheck): MasternodeVotingKeyCheckOutcome {
        val function = "dash_sdk_masternode_voting_key_check_outcome"
        val root = parse(function, text)
        val storeKey = root["storeKey"]?.jsonPrimitive?.booleanOrNull
            ?: throw DashSdkError.SerializationError("$function returned no storeKey")
        val record = decodeRecord(function, root["record"], preferring)
        return MasternodeVotingKeyCheckOutcome(storeKey, record)
    }

    // DashSDKMasternodeVotingLoadKind
    const val LOAD_OWNER_IDENTITY = 0
    const val LOAD_VOTER_IDENTITY = 1
    const val LOAD_REFUSED = 2

    /**
     * `{"kind":0|1|2,"identityId":hex|null,"identityJson":string|null,
     * "votingKeyVerdict":0|1|2|null,"message":string|null}` from `loadForVoting`.
     */
    fun decodeLoad(text: String?): MasternodeVotingLoad {
        val function = "dash_sdk_masternode_load_for_voting"
        val root = parse(function, text)
        val message = root["message"]?.jsonPrimitive?.contentOrNull
        val role = when (val kind = root["kind"]?.jsonPrimitive?.intOrNull) {
            LOAD_OWNER_IDENTITY -> MasternodeVotingLoadedIdentity.Role.OWNER
            LOAD_VOTER_IDENTITY -> MasternodeVotingLoadedIdentity.Role.VOTER
            LOAD_REFUSED -> return decodeRefusal(function, root, message.orEmpty())
            else -> throw DashSdkError.SerializationError("$function returned an unknown kind $kind")
        }
        val identityId = root["identityId"]?.jsonPrimitive?.contentOrNull
            ?.let { runCatching { it.hexToByteArray() }.getOrNull() }
            ?.takeIf { it.size == 32 }
            ?: throw DashSdkError.SerializationError("$function returned no 32-byte identity id")
        val identityJson = root["identityJson"]?.jsonPrimitive?.contentOrNull
            ?: throw DashSdkError.SerializationError("$function returned no identity")
        val check = when (val verdict = root["votingKeyVerdict"]?.jsonPrimitive?.intOrNull) {
            null -> null
            VERDICT_VALID -> MasternodeVotingKeyCheck.Valid
            VERDICT_INVALID -> MasternodeVotingKeyCheck.Invalid(message.orEmpty())
            VERDICT_UNDETERMINED -> MasternodeVotingKeyCheck.Undetermined(message.orEmpty())
            else -> throw DashSdkError.SerializationError("$function returned an unknown verdict $verdict")
        }
        return MasternodeVotingLoad.Loaded(
            MasternodeVotingLoadedIdentity(role, identityId, identityJson, check),
        )
    }

    /** A refused load; with a verdict (always invalid) it names the refused voter identity. */
    private fun decodeRefusal(function: String, root: JsonObject, message: String): MasternodeVotingLoad.Refused {
        val verdict = root["votingKeyVerdict"]?.jsonPrimitive?.intOrNull
            ?: return MasternodeVotingLoad.Refused(message)
        if (verdict != VERDICT_INVALID) {
            throw DashSdkError.SerializationError("$function refused with verdict $verdict, not invalid")
        }
        val refusedFor = root["identityId"]?.jsonPrimitive?.contentOrNull
            ?.let { runCatching { it.hexToByteArray() }.getOrNull() }
            ?.takeIf { it.size == 32 }
            ?: throw DashSdkError.SerializationError("$function refused a voting key without its 32-byte voter identity")
        return MasternodeVotingLoad.Refused(message, refusedFor)
    }

    /**
     * A record in the JNI JSON: null (or absent) for none, else its check and 20-byte address.
     * A record of [preferring]'s own kind carries [preferring], so its message is kept (Swift
     * `MasternodeVotingKeyCheckFFI.recordedKey(_:preferring:)`).
     */
    private fun decodeRecord(
        function: String,
        element: kotlinx.serialization.json.JsonElement?,
        preferring: MasternodeVotingKeyCheck,
    ): MasternodeRecordedVotingKey? {
        if (element == null || element == JsonNull) return null
        val obj = element.jsonObject
        val kind = when (val check = obj["check"]?.jsonPrimitive?.intOrNull) {
            RECORDED_VALID -> MasternodeVotingKeyCheck.Valid
            RECORDED_INVALID -> MasternodeVotingKeyCheck.Invalid("")
            RECORDED_UNDETERMINED -> MasternodeVotingKeyCheck.Undetermined("")
            else -> throw DashSdkError.SerializationError("$function returned an unknown recorded check $check")
        }
        val address = obj["votingAddress"]?.jsonPrimitive?.contentOrNull
            ?.let { runCatching { it.hexToByteArray() }.getOrNull() }
            ?.let { MasternodeVotingKeyAddress.of(it) }
            ?: throw DashSdkError.SerializationError("$function returned a record without a 20-byte address")
        return MasternodeRecordedVotingKey(
            check = if (verdictOf(kind) == verdictOf(preferring)) preferring else kind,
            votingAddress = address,
        )
    }

    /**
     * The message an undetermined record carries when another key was loaded for the same
     * masternode. Swift `MasternodeVotingKeyCheck.checkAgainAfterAnotherKeyMessage`.
     */
    const val CHECK_AGAIN_AFTER_ANOTHER_KEY_MESSAGE =
        "Another voting key was loaded for this masternode, so this one is checked again before it votes."

    /**
     * `{"record":null|{"check":1|2|3,"votingAddress":hex}}` from `siblingVotingKeyRecord`. An
     * undetermined record carries [CHECK_AGAIN_AFTER_ANOTHER_KEY_MESSAGE].
     */
    fun decodeSiblingRecord(text: String?): MasternodeRecordedVotingKey? {
        val function = "dash_sdk_masternode_sibling_voting_key_record"
        return decodeRecord(
            function,
            parse(function, text)["record"],
            preferring = MasternodeVotingKeyCheck.Undetermined(CHECK_AGAIN_AFTER_ANOTHER_KEY_MESSAGE),
        )
    }

    /** `{"canVote":bool,"checkBeforeUse":bool}` from `recordedVotingKeyUse`. */
    fun decodeUse(text: String?): MasternodeRecordedVotingKeyUse {
        val function = "dash_sdk_masternode_recorded_voting_key_use"
        val root = parse(function, text)
        return MasternodeRecordedVotingKeyUse(
            canVote = root["canVote"]?.jsonPrimitive?.booleanOrNull
                ?: throw DashSdkError.SerializationError("$function returned no canVote"),
            checkBeforeUse = root["checkBeforeUse"]?.jsonPrimitive?.booleanOrNull
                ?: throw DashSdkError.SerializationError("$function returned no checkBeforeUse"),
        )
    }

    private fun parse(function: String, text: String?): JsonObject {
        if (text == null) throw DashSdkError.SerializationError("$function returned no answer")
        return try {
            json.parseToJsonElement(text).jsonObject
        } catch (e: SerializationException) {
            throw DashSdkError.SerializationError("$function returned malformed JSON", e)
        } catch (e: IllegalArgumentException) {
            throw DashSdkError.SerializationError("$function returned malformed JSON", e)
        }
    }

    // The values `IdentityEntity.votingKeyCheckRaw` holds, as Swift's
    // `PersistentIdentity.votingKeyCheckRaw` does.
    const val RAW_VALID = "valid"
    const val RAW_INVALID = "invalid"
    const val RAW_UNDETERMINED = "undetermined"
}

/**
 * The last check of the row's stored voting key, for display: null when it was never checked
 * or the stored value is not one this build knows. Swift `PersistentIdentity.votingKeyCheck`.
 */
val IdentityEntity.votingKeyCheck: MasternodeVotingKeyCheck?
    get() = when (votingKeyCheckRaw) {
        MasternodeVotingKeyCheckCodec.RAW_VALID -> MasternodeVotingKeyCheck.Valid
        MasternodeVotingKeyCheckCodec.RAW_INVALID ->
            MasternodeVotingKeyCheck.Invalid(votingKeyCheckMessage.orEmpty())
        MasternodeVotingKeyCheckCodec.RAW_UNDETERMINED ->
            MasternodeVotingKeyCheck.Undetermined(votingKeyCheckMessage.orEmpty())
        else -> null
    }

/**
 * The recorded check with the address of the key it was made of, as the voting key rules take
 * it; null when nothing is recorded or it was recorded without a whole address. Swift
 * `PersistentIdentity.recordedVotingKey`.
 */
val IdentityEntity.recordedVotingKey: MasternodeRecordedVotingKey?
    get() {
        val check = votingKeyCheck ?: return null
        val address = votingKeyCheckAddress?.let { MasternodeVotingKeyAddress.of(it) } ?: return null
        return MasternodeRecordedVotingKey(check, address)
    }

/**
 * Which of a masternode's identities this row is, as Rust's load rule returned it
 * ([IdentityEntity.masternodeRoleRaw]); null for a row not loaded through it. Swift
 * `PersistentIdentity.masternodeRole`.
 */
val IdentityEntity.masternodeRole: MasternodeVotingLoadedIdentity.Role?
    get() = MasternodeVotingLoadedIdentity.Role.entries.firstOrNull { it.raw == masternodeRoleRaw }

/**
 * Whether this row is a masternode's voter identity, which holds only a voting key: credits or
 * tokens sent to it could not be spent, so it is never offered or accepted as a recipient.
 * Swift `PersistentIdentity.isVotingOnlyIdentity`.
 */
val IdentityEntity.isVotingOnlyIdentity: Boolean
    get() = masternodeRole == MasternodeVotingLoadedIdentity.Role.VOTER

/**
 * The voting address of the key the row names (`votingPrivateKeyIdentifier`), read from the
 * identifier, so no key is read; null when the row names none or an identifier this build
 * cannot read.
 */
val IdentityEntity.storedVotingKeyAddress: MasternodeVotingKeyAddress?
    get() = votingPrivateKeyIdentifier?.let {
        try {
            MasternodeVotingKeyAddress.ofIdentifier(it)
        } catch (_: IllegalArgumentException) {
            null
        }
    }

/**
 * How the row's stored voting key may be used, as [rules] decide from its record and the
 * stored key's address (read from its identifier). Null when the row names no voting key.
 */
fun IdentityEntity.storedVotingKeyUse(
    rules: MasternodeVotingKeyRules = MasternodeVotingKeyRules.Native,
): MasternodeRecordedVotingKeyUse? {
    val address = storedVotingKeyAddress ?: return null
    return rules.use(recordedVotingKey, address)
}

/**
 * Persisting what Rust says to record for a checked voting key on its identity row. Port of
 * the record writes in Swift `LoadIdentityView.swift` and `MasternodeVoterSection.swift`.
 */
object MasternodeVotingKeyRecords {

    /**
     * Record [check] of the voting key [storedIdentifier] names on [identityId]'s row on
     * [networkRaw], as [rules] decide. One Room transaction reads the row, asks Rust and writes
     * the record, and only while the row still names [storedIdentifier]: an answer about a key
     * replaced meanwhile is dropped. The slot Rust is given is the row as that transaction reads
     * it: the checked key, with the record the row holds now. A record written by another check
     * after the key was stored (a newer refusal of the same key) is therefore weighed by Rust,
     * never overwritten unseen; a record left by another key is ignored by Rust, so a key just
     * stored into an empty slot or over another key reads as never checked. Nothing is written
     * before the check's answer exists, so a check that never finishes (process death) leaves
     * the row as it was.
     *
     * @return the outcome Rust returned, or null when the row is gone, on another network or
     *   names another key, so nothing was written.
     * @throws Exception when the row cannot be read or written; cancellation propagates.
     */
    suspend fun record(
        database: DashDatabase,
        identityId: ByteArray,
        networkRaw: Int,
        storedIdentifier: String,
        check: MasternodeVotingKeyCheck,
        rules: MasternodeVotingKeyRules = MasternodeVotingKeyRules.Native,
    ): MasternodeVotingKeyCheckOutcome? =
        recordWithRevision(database, identityId, networkRaw, storedIdentifier, check, rules)?.outcome

    /** What [recordWithRevision] did: Rust's [outcome], and the row's revision after it. */
    data class Recorded(val outcome: MasternodeVotingKeyCheckOutcome, val revision: Long)

    /**
     * [record], also returning the row's `votingKeyCheckRevision` as the same transaction left
     * it: a host holding the written record can tell from a later Room emission whether it
     * already shows this write or a later one (its revision is at least this one).
     */
    suspend fun recordWithRevision(
        database: DashDatabase,
        identityId: ByteArray,
        networkRaw: Int,
        storedIdentifier: String,
        check: MasternodeVotingKeyCheck,
        rules: MasternodeVotingKeyRules = MasternodeVotingKeyRules.Native,
    ): Recorded? {
        val address = MasternodeVotingKeyAddress.ofIdentifier(storedIdentifier)
        return database.withTransaction {
            val dao = database.identityDao()
            val row = dao.getByIdentityId(identityId)
                ?.takeIf { it.networkRaw == networkRaw }
                ?: return@withTransaction null
            val identifier = row.votingPrivateKeyIdentifier
                ?.takeIf { it.equals(storedIdentifier, ignoreCase = true) }
                ?: return@withTransaction null
            val outcome = rules.outcome(check, address, MasternodeVotingKeySlot.SameKey(row.recordedVotingKey))
            outcome.record?.let { record ->
                val (raw, message) = rawOf(record.check)
                dao.recordVotingKeyCheck(
                    identityId = identityId,
                    networkRaw = networkRaw,
                    identifier = identifier,
                    raw = raw,
                    message = message,
                    address = record.votingAddress.bytes,
                )
            }
            val revision = dao.getByIdentityId(identityId)?.votingKeyCheckRevision ?: 0L
            Recorded(outcome, revision)
        }
    }

    /**
     * After the key [loadedIdentifier] names was stored for [loadedIdentityId] and checked
     * ([loaded]), record on every other row on [networkRaw] for the same masternode
     * ([proTxHash]) what [rules] say for its key (a valid record of a different key becomes
     * undetermined, so it is checked again before it votes). One Room transaction; each write is
     * conditional on the row still naming the key it was read with. Port of the sibling loop in
     * Swift `LoadIdentityView.swift`.
     *
     * @return how many rows were written.
     * @throws Exception when the rows cannot be read or written; cancellation propagates.
     */
    suspend fun recordSiblings(
        database: DashDatabase,
        networkRaw: Int,
        proTxHash: ByteArray,
        loadedIdentityId: ByteArray,
        loadedIdentifier: String,
        loaded: MasternodeVotingKeyCheck,
        rules: MasternodeVotingKeyRules = MasternodeVotingKeyRules.Native,
    ): Int {
        val loadedAddress = MasternodeVotingKeyAddress.ofIdentifier(loadedIdentifier)
        return database.withTransaction {
            val dao = database.identityDao()
            var written = 0
            for (sibling in dao.getByMasternodeProTxHash(networkRaw, proTxHash)) {
                if (sibling.identityId.contentEquals(loadedIdentityId)) continue
                val identifier = sibling.votingPrivateKeyIdentifier ?: continue
                val record = rules.siblingRecord(loaded, loadedAddress, sibling.recordedVotingKey) ?: continue
                val (raw, message) = rawOf(record.check)
                written += dao.recordVotingKeyCheck(
                    identityId = sibling.identityId,
                    networkRaw = networkRaw,
                    identifier = identifier,
                    raw = raw,
                    message = message,
                    address = record.votingAddress.bytes,
                )
            }
            written
        }
    }

    /** The `votingKeyCheckRaw` and `votingKeyCheckMessage` values of [check]. */
    internal fun rawOf(check: MasternodeVotingKeyCheck): Pair<String, String?> = when (check) {
        MasternodeVotingKeyCheck.Valid -> MasternodeVotingKeyCheckCodec.RAW_VALID to null
        is MasternodeVotingKeyCheck.Invalid -> MasternodeVotingKeyCheckCodec.RAW_INVALID to check.message
        is MasternodeVotingKeyCheck.Undetermined ->
            MasternodeVotingKeyCheckCodec.RAW_UNDETERMINED to check.message
    }
}
