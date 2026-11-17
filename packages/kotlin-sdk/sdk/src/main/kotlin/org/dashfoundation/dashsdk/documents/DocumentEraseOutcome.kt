package org.dashfoundation.dashsdk.documents

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.dashsdk.queries.DocumentLifecycle

/** A Rust decision about whether to submit the requested erase. */
enum class DocumentEraseNotSubmittedReason { ACTIVE, ABSENT, NOT_NEWEST_OWNER, PREFLIGHT_UNAVAILABLE }

/** The paid operation and its observations are separate facts. */
sealed class DocumentEraseOutcome {
    data class NotSubmitted(
        val reason: DocumentEraseNotSubmittedReason,
        val owner: String?,
        val before: DocumentLifecycle?,
    ) : DocumentEraseOutcome()

    data class Submitted(
        val before: DocumentLifecycle?,
        val after: DocumentLifecycle?,
    ) : DocumentEraseOutcome()

    companion object {
        internal fun fromJson(json: String): DocumentEraseOutcome = try {
            val root = Json.parseToJsonElement(json).jsonObject
            fun observation(key: String): DocumentLifecycle? {
                val value = requireNotNull(root[key])
                if (value == JsonNull) return null
                val obj = value as? JsonObject ?: error("invalid observation")
                return requireNotNull(DocumentLifecycle.fromLifecycleObject(obj))
            }
            val before = observation("before")
            val after = observation("after")
            val reasonValue = root.getValue("reason")
            val reason = if (reasonValue == JsonNull) null else
                DocumentEraseNotSubmittedReason.valueOf(
                    requireNotNull(reasonValue.jsonPrimitive.takeIf { it.isString }).content,
                )
            val ownerValue = root.getValue("owner")
            val owner = if (ownerValue == JsonNull) null else
                requireNotNull(ownerValue.jsonPrimitive.takeIf { it.isString }).content
            when (root.getValue("kind").jsonPrimitive.takeIf { it.isString }?.content) {
                "NOT_SUBMITTED" -> {
                    requireNotNull(reason)
                    if (reason == DocumentEraseNotSubmittedReason.NOT_NEWEST_OWNER) {
                        requireNotNull(owner)
                    }
                    NotSubmitted(reason, owner, before)
                }
                "SUBMITTED" -> Submitted(before, after)
                else -> error("unknown outcome")
            }
        } catch (e: Exception) {
            throw DocumentEraseResultUnreadableException(e)
        }
    }
}

/** An OK native result could not be decoded; it is unsafe to infer submission failure. */
class DocumentEraseResultUnreadableException(cause: Throwable? = null) :
    IllegalStateException("Erase result unreadable; read history before retrying.", cause)
