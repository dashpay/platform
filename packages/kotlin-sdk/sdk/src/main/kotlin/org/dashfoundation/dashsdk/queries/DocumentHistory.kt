package org.dashfoundation.dashsdk.queries

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * Which revisions of a keep-history document [Documents.history] reads.
 * Exactly one selector applies per call. [code] is the discriminant of the
 * FFI's `DashSDKDocumentHistorySelector`; [timeMs] and [revision] carry the
 * selector's operands, zero where the selector has none.
 */
sealed class DocumentHistorySelector(
    internal val code: Int,
    internal val timeMs: Long,
    internal val revision: Long,
) {
    /** Revisions written at or after [ms], oldest first; 0 is the first page of everything. */
    class StartAtTime(ms: Long) : DocumentHistorySelector(0, ms, 0)

    /** Revisions after the complete cursor of the last entry received. */
    class StartAfter(timeMs: Long, revision: Long) : DocumentHistorySelector(1, timeMs, revision)

    /** Revisions from history sequence number [revision] onwards. */
    class StartAtRevision(revision: Long) : DocumentHistorySelector(2, 0, revision)

    /** Exactly the revision at history sequence number [revision]; limit must be 1. */
    class Revision(revision: Long) : DocumentHistorySelector(3, 0, revision)
}

/** Where a keep-history document stands, as history v1 authenticates it. */
enum class DocumentLifecycleState {
    /** Visible to ordinary reads. */
    ACTIVE,

    /** Deleted with its revisions retained. */
    DELETED,

    /** An authorized erasure has begun and revisions are being removed. */
    ERASING,

    /** Nothing is left. */
    ABSENT,
}

/**
 * The `lifecycle` block of a [Documents.history] page. Times are epoch
 * milliseconds and are zero unless the state they describe has been reached.
 */
data class DocumentLifecycle(
    val state: DocumentLifecycleState,
    /** Exact count of revisions still retained; zero when absent. */
    val remainingRevisions: Long,
    val deletedAtMs: Long,
    val erasingStartedAtMs: Long,
    val erasingFromTimeMs: Long,
    val erasingFromRevision: Long,
) {
    companion object {
        private val lenient = Json { ignoreUnknownKeys = true; isLenient = true }

        /**
         * Parse the lifecycle block out of a history page JSON object; null
         * when the block, its state, or any of its numbers is missing or
         * malformed, so a partial block is never read as zeros.
         */
        fun fromHistoryJson(historyJson: String): DocumentLifecycle? = try {
            val root = lenient.parseToJsonElement(historyJson).jsonObject
            (root["lifecycle"] as? JsonObject)?.let(::fromLifecycleObject)
        } catch (_: Exception) {
            null
        }

        private fun fromLifecycleObject(obj: JsonObject): DocumentLifecycle? {
            val state = obj["state"]?.jsonPrimitive?.content
                ?.let { name -> DocumentLifecycleState.entries.firstOrNull { it.name == name } }
                ?: return null
            fun number(key: String): Long? = obj[key]?.jsonPrimitive?.content?.toLongOrNull()
            return DocumentLifecycle(
                state = state,
                remainingRevisions = number("remaining_revisions") ?: return null,
                deletedAtMs = number("deleted_at_ms") ?: return null,
                erasingStartedAtMs = number("erasing_started_at_ms") ?: return null,
                erasingFromTimeMs = number("erasing_from_time_ms") ?: return null,
                erasingFromRevision = number("erasing_from_revision") ?: return null,
            )
        }
    }
}

/**
 * Describe what an erase achieved, from the lifecycle read [before] it was
 * submitted (null when not read) and the one read [after] its absence was
 * observed. The erase result itself proves only that the document is absent
 * from ordinary reads, which it already was, so the words come from the
 * lifecycle: a document still DELETED with the same retained count is
 * reported as not established, never as an accepted or completed erase.
 */
fun describeEraseProgress(before: DocumentLifecycle?, after: DocumentLifecycle): String = when (after.state) {
    DocumentLifecycleState.ABSENT ->
        "Erasure complete: no revisions remain and the id is free again."
    DocumentLifecycleState.ERASING ->
        "Erasure in progress: ${after.remainingRevisions} revisions still retained; " +
            "submit another erase to continue."
    DocumentLifecycleState.DELETED ->
        if (before != null && before.state == DocumentLifecycleState.DELETED &&
            before.remainingRevisions == after.remainingRevisions
        ) {
            "Not established: the document is still DELETED with " +
                "${after.remainingRevisions} revisions retained, so the history shows " +
                "nothing this erase removed."
        } else {
            "The document is DELETED with ${after.remainingRevisions} revisions retained; " +
                "no erasure has been recorded."
        }
    DocumentLifecycleState.ACTIVE ->
        "The document is active; an erase applies only after it has been deleted."
}
