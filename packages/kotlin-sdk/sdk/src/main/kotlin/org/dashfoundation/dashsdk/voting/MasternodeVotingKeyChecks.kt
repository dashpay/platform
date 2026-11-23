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
        val record = when (val element = root["record"]) {
            null, JsonNull -> null
            else -> {
                val obj = element.jsonObject
                val kind = when (val check = obj["check"]?.jsonPrimitive?.intOrNull) {
                    RECORDED_VALID -> MasternodeVotingKeyCheck.Valid
                    RECORDED_INVALID -> MasternodeVotingKeyCheck.Invalid("")
                    RECORDED_UNDETERMINED -> MasternodeVotingKeyCheck.Undetermined("")
                    else -> throw DashSdkError.SerializationError(
                        "$function returned an unknown recorded check $check",
                    )
                }
                val address = obj["votingAddress"]?.jsonPrimitive?.contentOrNull
                    ?.let { runCatching { it.hexToByteArray() }.getOrNull() }
                    ?.let { MasternodeVotingKeyAddress.of(it) }
                    ?: throw DashSdkError.SerializationError("$function returned a record without a 20-byte address")
                MasternodeRecordedVotingKey(
                    check = if (verdictOf(kind) == verdictOf(preferring)) preferring else kind,
                    votingAddress = address,
                )
            }
        }
        return MasternodeVotingKeyCheckOutcome(storeKey, record)
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

    /** What the identity's voting key slot held before the checked key was in it. */
    enum class SlotBefore {
        /** No key was stored; the checked key was just stored into the empty slot. */
        EMPTY,

        /** The checked key was already stored (or stays stored, for a refusal of it). */
        SAME_KEY,

        /** Another key was stored; the checked key replaced it. */
        DIFFERENT_KEY,
    }

    /**
     * Record [check] of the voting key [storedIdentifier] names on [identityId]'s row on
     * [networkRaw], as [rules] decide given what the slot held before ([slotBefore]; for
     * [SlotBefore.SAME_KEY] with the record the row holds now). One Room transaction reads the
     * row, asks Rust and writes the record, and only while the row still names
     * [storedIdentifier]: an answer about a key replaced meanwhile is dropped. Nothing is
     * written before the check's answer exists, so a check that never finishes (process death)
     * leaves the row as it was.
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
        slotBefore: SlotBefore,
        rules: MasternodeVotingKeyRules = MasternodeVotingKeyRules.Native,
    ): MasternodeVotingKeyCheckOutcome? {
        val address = MasternodeVotingKeyAddress.ofIdentifier(storedIdentifier)
        return database.withTransaction {
            val dao = database.identityDao()
            val row = dao.getByIdentityId(identityId)
                ?.takeIf { it.networkRaw == networkRaw }
                ?: return@withTransaction null
            val identifier = row.votingPrivateKeyIdentifier
                ?.takeIf { it.equals(storedIdentifier, ignoreCase = true) }
                ?: return@withTransaction null
            val slot = when (slotBefore) {
                SlotBefore.EMPTY -> MasternodeVotingKeySlot.Empty
                SlotBefore.SAME_KEY -> MasternodeVotingKeySlot.SameKey(row.recordedVotingKey)
                SlotBefore.DIFFERENT_KEY -> MasternodeVotingKeySlot.DifferentKey
            }
            val outcome = rules.outcome(check, address, slot)
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
            outcome
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
