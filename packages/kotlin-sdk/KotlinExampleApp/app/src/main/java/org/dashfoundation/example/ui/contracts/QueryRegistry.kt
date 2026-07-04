package org.dashfoundation.example.ui.contracts

import org.dashfoundation.dashsdk.Sdk

/**
 * The runnable platform-query registry — the Kotlin counterpart of the
 * `QueryDefinition` lists in `PlatformQueriesView.swift`, restricted to the
 * queries the Kotlin SDK's read surface actually bridges today. The iOS
 * view routes ~40 more (contested resources, epochs, groups, tokens,
 * protocol versions, …) that wait on their JNI exports; those stay listed
 * in `PARITY.md` rather than as dead rows here.
 *
 * [QueryDefinition.diagnosticInputs] carries the known-good testnet
 * fixtures from `DiagnosticsView.swift`'s `TestData` so the Diagnostics
 * screen can run every registry entry unattended (← `runAllQueries`).
 */

internal data class QueryInput(
    val name: String,
    val label: String,
    val required: Boolean,
    val placeholder: String = "",
)

internal class QueryDefinition(
    val name: String,
    val label: String,
    val description: String,
    val inputs: List<QueryInput>,
    val diagnosticInputs: Map<String, String> = emptyMap(),
    val execute: suspend (Sdk, Map<String, String>) -> String?,
)

/**
 * Testnet fixtures — the exact values `DiagnosticsView.swift` uses
 * (themselves lifted from the WASM SDK docs for cross-SDK consistency).
 */
internal object QueryTestData {
    const val TEST_IDENTITY_ID = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"
    const val DPNS_CONTRACT_ID = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
    const val TEST_DOCUMENT_TYPE = "domain"
    const val TEST_USERNAME = "therealslimshaddy5"
}

internal val QUERY_REGISTRY: List<QueryDefinition> = listOf(
    QueryDefinition(
        name = "getIdentity",
        label = "Get Identity",
        description = "Fetch an identity by its base58 identifier.",
        inputs = listOf(QueryInput("identityId", "Identity ID", required = true)),
        diagnosticInputs = mapOf("identityId" to QueryTestData.TEST_IDENTITY_ID),
    ) { sdk, inputs -> sdk.identities.fetch(inputs.getValue("identityId")) },
    QueryDefinition(
        name = "getIdentityBalance",
        label = "Get Identity Balance",
        description = "Fetch an identity's balance in credits.",
        inputs = listOf(QueryInput("identityId", "Identity ID", required = true)),
        diagnosticInputs = mapOf("identityId" to QueryTestData.TEST_IDENTITY_ID),
    ) { sdk, inputs ->
        sdk.identities.fetchBalance(inputs.getValue("identityId"))
            ?.let { "{\"balance\": $it}" }
    },
    QueryDefinition(
        name = "getDataContract",
        label = "Get Data Contract",
        description = "Fetch a data contract as JSON by its base58 identifier.",
        inputs = listOf(QueryInput("contractId", "Contract ID", required = true)),
        diagnosticInputs = mapOf("contractId" to QueryTestData.DPNS_CONTRACT_ID),
    ) { sdk, inputs -> sdk.contracts.fetchJson(inputs.getValue("contractId")) },
    QueryDefinition(
        name = "getDocuments",
        label = "Get Documents",
        description = "Query documents of a contract's document type.",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("documentType", "Document Type", required = true, placeholder = "domain"),
            QueryInput(
                "where", "Where (JSON)", required = false,
                placeholder = "[{\"field\":\"...\",\"operator\":\"==\",\"value\":\"...\"}]",
            ),
            QueryInput(
                "orderBy", "Order By (JSON)", required = false,
                placeholder = "[{\"field\":\"...\",\"ascending\":true}]",
            ),
            QueryInput("limit", "Limit", required = false, placeholder = "25"),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
            "documentType" to QueryTestData.TEST_DOCUMENT_TYPE,
            "limit" to "5",
        ),
    ) { sdk, inputs ->
        sdk.contracts.fetch(inputs.getValue("contractId")).use { contract ->
            sdk.documents.search(
                contract = contract,
                documentType = inputs.getValue("documentType"),
                whereJson = inputs["where"]?.takeIf { it.isNotBlank() },
                orderByJson = inputs["orderBy"]?.takeIf { it.isNotBlank() },
                limit = inputs["limit"]?.trim()?.toIntOrNull() ?: 0,
            )
        }
    },
    QueryDefinition(
        name = "countDocuments",
        label = "Count Documents",
        description = "Count documents of a contract's document type (aggregate).",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("documentType", "Document Type", required = true, placeholder = "domain"),
            QueryInput(
                "where", "Where (JSON)", required = false,
                placeholder = "[{\"field\":\"...\",\"operator\":\"==\",\"value\":\"...\"}]",
            ),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
            "documentType" to QueryTestData.TEST_DOCUMENT_TYPE,
        ),
    ) { sdk, inputs ->
        sdk.contracts.fetch(inputs.getValue("contractId")).use { contract ->
            sdk.documents.count(
                contract = contract,
                documentType = inputs.getValue("documentType"),
                whereJson = inputs["where"]?.takeIf { it.isNotBlank() },
            )
        }
    },
    QueryDefinition(
        name = "dpnsResolve",
        label = "DPNS Resolve",
        description = "Resolve a DPNS name (e.g. alice.dash) to its record.",
        inputs = listOf(QueryInput("name", "Name", required = true, placeholder = "alice.dash")),
        diagnosticInputs = mapOf("name" to "${QueryTestData.TEST_USERNAME}.dash"),
    ) { sdk, inputs -> sdk.dpns.resolve(inputs.getValue("name")) },
    QueryDefinition(
        name = "dpnsCheckAvailability",
        label = "DPNS Check Availability",
        description = "Check whether a DPNS label is still available.",
        inputs = listOf(QueryInput("label", "Label", required = true, placeholder = "alice")),
        diagnosticInputs = mapOf("label" to QueryTestData.TEST_USERNAME),
    ) { sdk, inputs -> sdk.dpns.checkAvailability(inputs.getValue("label")) },
    QueryDefinition(
        name = "dpnsGetUsernames",
        label = "DPNS Usernames",
        description = "List usernames owned by an identity.",
        inputs = listOf(
            QueryInput("identityId", "Identity ID", required = true),
            QueryInput("limit", "Limit", required = false, placeholder = "10"),
        ),
        diagnosticInputs = mapOf("identityId" to QueryTestData.TEST_IDENTITY_ID),
    ) { sdk, inputs ->
        sdk.dpns.usernames(
            inputs.getValue("identityId"),
            inputs["limit"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
    QueryDefinition(
        name = "dpnsSearch",
        label = "DPNS Search",
        description = "Search DPNS names by prefix.",
        inputs = listOf(
            QueryInput("prefix", "Prefix", required = true, placeholder = "ali"),
            QueryInput("limit", "Limit", required = false, placeholder = "10"),
        ),
        diagnosticInputs = mapOf("prefix" to "dash", "limit" to "5"),
    ) { sdk, inputs ->
        sdk.dpns.search(
            inputs.getValue("prefix"),
            inputs["limit"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
)
