package org.dashfoundation.dashsdk.identity

import java.nio.ByteBuffer
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.TransactionsNative

/**
 * One transition inside a parsed `BatchTransition`, in batch order — the
 * Kotlin mirror of Swift's `ManagedPlatformWallet.ParsedBatchedTransition`.
 *
 * [action] is the rs-dpp action name (`Create`, `Replace`, `Delete`,
 * `Transfer`, `Purchase`, `UpdatePrice`, `IndexOnlyDelete` for documents;
 * `Burn`, `Mint`, `Transfer`, `Freeze`, `Unfreeze`, `DestroyFrozenFunds`,
 * `Claim`, `EmergencyAction`, `ConfigUpdate`, `DirectPurchase`,
 * `SetPriceForDirectPurchase` for tokens).
 */
sealed class ParsedBatchedTransition {
    abstract val dataContractId: ByteArray
    abstract val action: String
    /**
     * Whether the typed fields describe every material field of the
     * transition. **A row with `complete == false` must not be approved from
     * the typed fields alone; the wallet must render [details].**
     */
    abstract val complete: Boolean
    /**
     * The material fields the typed fields do not cover (document data, a
     * token config change and who it grants or revokes, an emergency action,
     * a price schedule, a distribution type, notes, group-action info),
     * rendered for display. Null when [complete].
     */
    abstract val details: String?
    /** Credits or tokens the transition moves, when it moves any (a protocol u64). */
    abstract val amount: ULong?
    /**
     * The identity on the other side of the transition, when there is one.
     * Interpret it against [action]: a transfer's recipient, a mint's
     * issued-to identity, or the identity whose tokens a freeze / unfreeze /
     * destroy acts on.
     */
    abstract val recipientId: ByteArray?

    /**
     * A document transition. [amount] is a purchase price or an
     * update-price value; [recipientId] is a transfer's new owner.
     */
    class Document(
        override val dataContractId: ByteArray,
        val documentType: String,
        val documentId: ByteArray,
        override val action: String,
        override val amount: ULong?,
        override val recipientId: ByteArray?,
        override val complete: Boolean,
        override val details: String?,
    ) : ParsedBatchedTransition() {
        override fun equals(other: Any?): Boolean =
            other is Document &&
                dataContractId.contentEquals(other.dataContractId) &&
                documentType == other.documentType &&
                documentId.contentEquals(other.documentId) &&
                action == other.action &&
                amount == other.amount &&
                recipientId.contentEqualsNullable(other.recipientId) &&
                complete == other.complete &&
                details == other.details

        override fun hashCode(): Int =
            listOf(dataContractId.contentHashCode(), documentType, documentId.contentHashCode(), action)
                .hashCode()

        override fun toString(): String =
            "Document(action=$action, documentType=$documentType, amount=$amount)"
    }

    /**
     * A token transition. [amount] is the transferred / minted / burned
     * token count or a direct purchase's total agreed price; [recipientId]
     * is a transfer's recipient, a mint's issued-to identity, or the frozen
     * identity of a freeze / unfreeze / destroy.
     */
    class Token(
        override val dataContractId: ByteArray,
        val tokenId: ByteArray,
        val tokenContractPosition: Int,
        override val action: String,
        override val amount: ULong?,
        override val recipientId: ByteArray?,
        /** A `DirectPurchase`'s token count ([amount] is its total agreed price). */
        val tokenCount: ULong?,
        override val complete: Boolean,
        override val details: String?,
    ) : ParsedBatchedTransition() {
        override fun equals(other: Any?): Boolean =
            other is Token &&
                dataContractId.contentEquals(other.dataContractId) &&
                tokenId.contentEquals(other.tokenId) &&
                tokenContractPosition == other.tokenContractPosition &&
                action == other.action &&
                amount == other.amount &&
                recipientId.contentEqualsNullable(other.recipientId) &&
                tokenCount == other.tokenCount &&
                complete == other.complete &&
                details == other.details

        override fun hashCode(): Int =
            listOf(dataContractId.contentHashCode(), tokenId.contentHashCode(), tokenContractPosition, action)
                .hashCode()

        override fun toString(): String =
            "Token(action=$action, position=$tokenContractPosition, amount=$amount)"
    }
}

/** The typed summary of a parsed state transition, discriminated by kind. */
sealed class ParsedStateTransitionKind {
    /** Key registration / revocation (`IdentityUpdateTransition`). */
    class IdentityUpdate(
        val identityId: ByteArray,
        val addPublicKeys: List<IdentityPubkey>,
        val disablePublicKeyIds: List<Int>,
    ) : ParsedStateTransitionKind()

    /** Document and token operations (`BatchTransition`). */
    class Batch(
        val ownerId: ByteArray,
        val transitions: List<ParsedBatchedTransition>,
    ) : ParsedStateTransitionKind()

    /** `IdentityCreditTransferTransition`. */
    class CreditTransfer(
        val identityId: ByteArray,
        val recipientId: ByteArray,
        val amount: ULong,
    ) : ParsedStateTransitionKind()

    /** `DataContractCreateTransition`. */
    class DataContractCreate(val contract: ParsedDataContract) : ParsedStateTransitionKind()

    /** `DataContractUpdateTransition`. */
    class DataContractUpdate(val contract: ParsedDataContract) : ParsedStateTransitionKind()

    /** Any other kind; see [ParsedStateTransition.kindName]. */
    object Other : ParsedStateTransitionKind()
}

/** Inspectable fields of a parsed data contract create / update. */
class ParsedDataContract(
    val contractId: ByteArray,
    val ownerId: ByteArray,
    /** Document type names the contract defines, as the contract orders them. */
    val documentTypeNames: List<String>,
)

/**
 * One parsed state transition (a `dash-st:` payload or a DashPay Connect
 * `sign` request), decoded whatever its kind — the Kotlin mirror of Swift's
 * `ManagedPlatformWallet.ParsedStateTransition`.
 *
 * [serialized] holds the decoded transition re-serialized in tagged DPP
 * framing; after approval, sign these rather than the input so what was
 * shown is what is signed. When [complete] is false, [details] (or the
 * batch rows' details) must be shown before approval. A `sign` request must arrive with `isSigned == false`
 * and [ownerId] equal to the wallet's own identity; both checks are the
 * caller's.
 */
class ParsedStateTransition(
    /** rs-dpp `StateTransition::name()`, e.g. `IdentityUpdate`, `MasternodeVote`. */
    val kindName: String,
    /** The identity the transition acts for; null for kinds that name none. */
    val ownerId: ByteArray?,
    /** Whether the transition already carries a signature. */
    val isSigned: Boolean,
    /**
     * Percentage added to the processing fee Platform charges (0 = none;
     * 65535 is about 656 times the base processing fee). Part of the signed
     * bytes: show a non-zero value on the sheet and refuse values above what
     * the wallet is willing to pay.
     */
    val userFeeIncrease: Int,
    /** Whether [kind] shows every material field; computed in Rust. */
    val complete: Boolean,
    /** The decoded bytes, tagged. */
    val serialized: ByteArray,
    /**
     * A structured multi-line dump: of the whole transition for
     * [ParsedStateTransitionKind.Other], of the whole contract for a data
     * contract create / update. Null otherwise.
     */
    val details: String?,
    val kind: ParsedStateTransitionKind,
)

/**
 * Decode-any-kind state transition parser — the Android analog of Swift's
 * `ManagedPlatformWallet.parseStateTransition`. One FFI call
 * ([TransactionsNative.parseStateTransition]) returns a packed blob that
 * [parseBlob] turns into a [ParsedStateTransition]; the blob layout is
 * documented in `rs-unified-sdk-jni/src/parse_state_transition.rs`.
 */
object StateTransitionParser {

    /** `ParsedStateTransitionFFI::kind` values (`PARSED_STATE_TRANSITION_KIND_*`). */
    private const val KIND_IDENTITY_UPDATE = 1
    private const val KIND_BATCH = 2
    private const val KIND_CREDIT_TRANSFER = 3
    private const val KIND_DATA_CONTRACT_CREATE = 4
    private const val KIND_DATA_CONTRACT_UPDATE = 5
    private const val KIND_OTHER = 255

    private const val FAMILY_DOCUMENT = 0
    private const val FAMILY_TOKEN = 1

    /**
     * Decode [transitionBytes] (tagged or Yappr's tagless framing). Never
     * signs or broadcasts. Throws the mapped native error on undecodable
     * bytes.
     */
    fun parse(transitionBytes: ByteArray): ParsedStateTransition {
        require(transitionBytes.isNotEmpty()) { "transitionBytes must not be empty" }
        // This is a handle-free utility, so it may well be the first SDK
        // call in the process; load the native library and run dash_sdk_init
        // before touching JNI (as TransactionDecoder.decode does), otherwise
        // the external fun resolves to UnsatisfiedLinkError.
        Sdk.initialize()
        val blob = mapNativeErrors { TransactionsNative.parseStateTransition(transitionBytes) }
        return try {
            parseBlob(blob)
        } catch (e: RuntimeException) {
            // A blob the JNI layer produced but this decoder cannot read is
            // a Rust/Kotlin layout drift, not caller input; surface it as
            // the same exception type native failures use so callers have
            // one error path.
            throw DashSDKException(
                BLOB_DECODE_ERROR_CODE,
                "parsed state transition blob could not be decoded: ${e.message}",
            )
        }
    }

    /** Mirrors the `99` internal-marshalling code the JNI layer throws with. */
    private const val BLOB_DECODE_ERROR_CODE = 99

    /**
     * Decode the packed blob the JNI layer returns. Internal + pure so it is
     * host-JVM unit-testable without the native library
     * (`StateTransitionParserTest` decodes the goldens the Rust side pins).
     */
    internal fun parseBlob(blob: ByteArray): ParsedStateTransition {
        val buf = ByteBuffer.wrap(blob) // big-endian by default
        val kindTag = buf.get().toInt() and 0xFF
        val kindName = readString32(buf)
        val ownerId = if (readBool(buf)) readId32(buf) else null
        val isSigned = readBool(buf)
        val userFeeIncrease = buf.short.toInt() and 0xFFFF
        val complete = readBool(buf)
        val serialized = readBytes32Len(buf)
        val details = readString32(buf).ifEmpty { null }

        val kind: ParsedStateTransitionKind = when (kindTag) {
            KIND_IDENTITY_UPDATE -> {
                val identityId = readId32(buf)
                val added = List(readCount(buf, "added keys")) { readPublicKey(buf) }
                val disabled = List(readCount(buf, "disabled keys")) { buf.int }
                ParsedStateTransitionKind.IdentityUpdate(identityId, added, disabled)
            }
            KIND_BATCH -> {
                val batchOwner = readId32(buf)
                val transitions = List(readCount(buf, "batched transitions")) { readBatched(buf) }
                ParsedStateTransitionKind.Batch(batchOwner, transitions)
            }
            KIND_CREDIT_TRANSFER ->
                ParsedStateTransitionKind.CreditTransfer(readId32(buf), readId32(buf), readU64(buf))
            KIND_DATA_CONTRACT_CREATE ->
                ParsedStateTransitionKind.DataContractCreate(readDataContract(buf))
            KIND_DATA_CONTRACT_UPDATE ->
                ParsedStateTransitionKind.DataContractUpdate(readDataContract(buf))
            KIND_OTHER -> ParsedStateTransitionKind.Other
            else -> throw IllegalArgumentException("malformed parse blob: unknown kind $kindTag")
        }

        require(!buf.hasRemaining()) { "malformed parse blob: trailing bytes" }
        return ParsedStateTransition(
            kindName, ownerId, isSigned, userFeeIncrease, complete, serialized, details, kind,
        )
    }

    private fun readPublicKey(buf: ByteBuffer): IdentityPubkey {
        val keyId = buf.int
        val keyType = buf.get().toInt() and 0xFF
        val purpose = buf.get().toInt() and 0xFF
        val securityLevel = buf.get().toInt() and 0xFF
        val readOnly = readBool(buf)
        val boundsKind = buf.get().toInt() and 0xFF
        val dataLen = buf.short.toInt() and 0xFFFF
        val data = ByteArray(dataLen).also { buf.get(it) }
        val boundsId = if (boundsKind != 0) readId32(buf) else null
        val docType = if (boundsKind == 2) readString16(buf) else null
        val flags = buf.get().toInt() and 0xFF
        val totalBudget = if (flags and 1 != 0) readLimit(buf, "totalBudget") else null
        val expiresAt = if (flags and 2 != 0) readLimit(buf, "expiresAt") else null

        val bounds: ContractBounds? = when (boundsKind) {
            0 -> null
            1 -> ContractBounds.SingleContract(boundsId!!)
            2 -> ContractBounds.SingleContractDocumentType(boundsId!!, docType!!)
            3 -> ContractBounds.ContractGroup(boundsId!!)
            else -> throw IllegalArgumentException("malformed parse blob: contract bounds kind $boundsKind")
        }
        return IdentityPubkey(
            keyId = keyId,
            keyType = KeyType.entries.firstOrNull { it.ffiValue == keyType }
                ?: throw IllegalArgumentException("malformed parse blob: key type $keyType"),
            purpose = KeyPurpose.entries.firstOrNull { it.ffiValue == purpose }
                ?: throw IllegalArgumentException("malformed parse blob: purpose $purpose"),
            securityLevel = SecurityLevel.entries.firstOrNull { it.ffiValue == securityLevel }
                ?: throw IllegalArgumentException("malformed parse blob: security level $securityLevel"),
            pubkeyBytes = data,
            readOnly = readOnly,
            contractBounds = bounds,
            totalBudget = totalBudget,
            expiresAt = expiresAt,
        )
    }

    private fun readBatched(buf: ByteBuffer): ParsedBatchedTransition {
        val family = buf.get().toInt() and 0xFF
        val dataContractId = readId32(buf)
        val action = readString16(buf)
        return when (family) {
            FAMILY_DOCUMENT -> {
                val documentType = readString16(buf)
                val documentId = readId32(buf)
                val amount = if (readBool(buf)) readU64(buf) else null
                val recipient = if (readBool(buf)) readId32(buf) else null
                val tokenCount = if (readBool(buf)) readU64(buf) else null
                require(tokenCount == null) { "malformed parse blob: token count on a document row" }
                val complete = readBool(buf)
                val details = readString32(buf).ifEmpty { null }
                ParsedBatchedTransition.Document(
                    dataContractId, documentType, documentId, action, amount, recipient, complete, details,
                )
            }
            FAMILY_TOKEN -> {
                val position = buf.short.toInt() and 0xFFFF
                val tokenId = readId32(buf)
                val amount = if (readBool(buf)) readU64(buf) else null
                val recipient = if (readBool(buf)) readId32(buf) else null
                val tokenCount = if (readBool(buf)) readU64(buf) else null
                val complete = readBool(buf)
                val details = readString32(buf).ifEmpty { null }
                ParsedBatchedTransition.Token(
                    dataContractId, tokenId, position, action, amount, recipient, tokenCount, complete, details,
                )
            }
            else -> throw IllegalArgumentException("malformed parse blob: batched family $family")
        }
    }

    private fun readDataContract(buf: ByteBuffer): ParsedDataContract {
        val contractId = readId32(buf)
        val ownerId = readId32(buf)
        val names = List(readCount(buf, "document type names")) { readString16(buf) }
        return ParsedDataContract(contractId, ownerId, names)
    }

    private fun readBool(buf: ByteBuffer): Boolean = buf.get().toInt() != 0

    private fun readId32(buf: ByteBuffer): ByteArray = ByteArray(32).also { buf.get(it) }

    private fun readCount(buf: ByteBuffer, what: String): Int {
        val count = buf.int
        require(count >= 0) { "malformed parse blob: negative $what count" }
        return count
    }

    /** `u16 len + UTF-8 bytes`. */
    private fun readString16(buf: ByteBuffer): String {
        val len = buf.short.toInt() and 0xFFFF
        val bytes = ByteArray(len).also { buf.get(it) }
        return String(bytes, Charsets.UTF_8)
    }

    /** A protocol u64, big-endian. */
    private fun readU64(buf: ByteBuffer): ULong = buf.long.toULong()

    /**
     * A key limit. [IdentityPubkey] carries limits as non-negative `Long`s, so
     * a u64 above `Long.MAX_VALUE` is refused rather than read as negative.
     */
    private fun readLimit(buf: ByteBuffer, what: String): Long {
        val value = readU64(buf)
        require(value <= Long.MAX_VALUE.toULong()) {
            "unsupported parsed transition: $what $value exceeds ${Long.MAX_VALUE}"
        }
        return value.toLong()
    }

    /** `u32 len + UTF-8 bytes`; an empty string encodes "none". */
    private fun readString32(buf: ByteBuffer): String = String(readBytes32Len(buf), Charsets.UTF_8)

    /** `u32 len + bytes`. */
    private fun readBytes32Len(buf: ByteBuffer): ByteArray {
        val len = buf.int
        require(len >= 0) { "malformed parse blob: negative length" }
        return ByteArray(len).also { buf.get(it) }
    }
}

private fun ByteArray?.contentEqualsNullable(other: ByteArray?): Boolean =
    if (this == null || other == null) this === other else contentEquals(other)
