package org.dashfoundation.dashsdk.tokens

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import org.dashfoundation.dashsdk.queries.ContractWithSerialization

/**
 * What [Tokens.updateConfig] did, as Rust decided it
 * (`platform_wallet::TokenConfigUpdateOutcome`).
 *
 * Platform applies a configuration change to the contract and raises its
 * version, but the update's proof does not carry the contract. So after an
 * applied update Rust fetches the contract again, refusing any version older
 * than the one the update produced, and hands it back here for the host to
 * store.
 */
sealed interface TokenConfigUpdateOutcome {

    /** A group action still waiting for signatures: the contract is unchanged. */
    data object Pending : TokenConfigUpdateOutcome

    /**
     * Applied. [contract] is the proof-verified contract the update produced,
     * in the shape `PlatformQueries.fetchWithSerialization` returns.
     */
    data class Applied(val contract: ContractWithSerialization) : TokenConfigUpdateOutcome

    /**
     * Applied, but no fetch returned a contract at [minimumVersion] or later
     * ([reason] says why): the stored contract still has the old
     * configuration.
     */
    data class AppliedNotRefreshed(
        val minimumVersion: Long,
        val reason: String,
    ) : TokenConfigUpdateOutcome

    companion object {
        private val json = Json { ignoreUnknownKeys = true }

        /** Decode the outcome JSON `platform_wallet_token_update_config_and_refresh` writes. */
        fun fromJson(text: String): TokenConfigUpdateOutcome {
            val root = json.parseToJsonElement(text).jsonObject
            return when (val kind = root.string("outcome")) {
                "pending" -> Pending
                "applied" -> {
                    val contract = root["contract"] as? JsonObject
                        ?: throw IllegalArgumentException("applied outcome without a contract")
                    Applied(
                        ContractWithSerialization(
                            json = contract.toString(),
                            binarySerialization = root.string("serializationHex")?.hexToByteArray(),
                        ),
                    )
                }
                "appliedNotRefreshed" -> AppliedNotRefreshed(
                    minimumVersion = root["minimumVersion"]?.jsonPrimitive?.longOrNull
                        ?: throw IllegalArgumentException("appliedNotRefreshed without minimumVersion"),
                    reason = root.string("reason").orEmpty(),
                )
                else -> throw IllegalArgumentException("unknown config update outcome '$kind'")
            }
        }

        private fun JsonObject.string(key: String): String? =
            (get(key) as? kotlinx.serialization.json.JsonPrimitive)?.takeIf { it.isString }?.content

        private fun String.hexToByteArray(): ByteArray {
            require(length % 2 == 0) { "odd-length hex" }
            return ByteArray(length / 2) { i -> substring(2 * i, 2 * i + 2).toInt(16).toByte() }
        }
    }
}
