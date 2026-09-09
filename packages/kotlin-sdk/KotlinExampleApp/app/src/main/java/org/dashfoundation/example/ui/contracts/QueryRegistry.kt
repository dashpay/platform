package org.dashfoundation.example.ui.contracts

import org.dashfoundation.dashsdk.Sdk

/**
 * The runnable platform-query registry — the Kotlin counterpart of the
 * `QueryDefinition` lists in `PlatformQueriesView.swift` and the
 * `queriesToTest` catalog in `DiagnosticsView.swift`. Every query the
 * Kotlin SDK's read surface bridges today (Wave-1A/1B/2C) has one entry
 * here; the handful of iOS rows still awaiting their JNI exports
 * (data-contract history/multiple, current/finalized epoch, identity token
 * infos, platform status) stay tracked in `PARITY.md` rather than as dead
 * rows.
 *
 * Address queries (`getAddressInfo` / `getAddressesInfos`) live on their
 * own [AddressQueriesScreen] as in iOS, so they are not registry rows.
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
 * Testnet fixtures — the exact values `DiagnosticsView.swift` uses in its
 * `TestData` struct (themselves lifted from the WASM SDK docs for
 * cross-SDK consistency).
 */
internal object QueryTestData {
    const val TEST_IDENTITY_ID = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"
    const val TEST_IDENTITY_ID_2 = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
    const val DPNS_CONTRACT_ID = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
    const val CONTRACT_WITH_HISTORY = "HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM"
    const val TEST_PUBLIC_KEY_HASH = "b7e904ce25ed97594e72f7af0e66f298031c1754"
    const val TEST_NON_UNIQUE_PUBLIC_KEY_HASH = "518038dc858461bcee90478fd994bba8057b7531"
    const val TEST_DOCUMENT_TYPE = "domain"
    const val TEST_DOCUMENT_ID = "7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r"
    const val TEST_USERNAME = "therealslimshaddy5"
    const val TEST_TOKEN_ID = "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"
    const val TEST_GROUP_CONTRACT_ID = "49PJEnNx7ReCitzkLdkDNr4s6RScGsnNexcdSZJ1ph5N"
    const val TEST_ACTION_ID = "6XJzL6Qb8Zhwxt4HFwh8NAn7q1u4dwdoUf8EmgzDudFZ"
    const val TEST_PREFUNDED_SPECIALIZED_BALANCE_ID =
        "AzaU7zqCT7X1kxh8yWxkT9PxAgNqWDu4Gz13emwcRyAT"

    // Contested DPNS resource fixtures (parentNameAndLabel index).
    const val CONTESTED_INDEX_NAME = "parentNameAndLabel"
    const val CONTESTED_INDEX_VALUES_JSON = "[\"dash\",\"alice\"]"

    // rs-sdk-ffi enum discriminants used by the voting/group catalogs.
    const val VOTE_RESULT_DOCUMENTS = "0"
    const val VOTE_RESULT_DOCUMENTS_AND_TALLY = "2"
    const val GROUP_STATUS_ACTIVE = "0"
    const val KEY_PURPOSES_CSV = "0,1,2,3"

    // GroveDB diagnostic path — the identities subtree root.
    const val PATH_ELEMENTS_JSON = "[\"identities\"]"
}

internal val QUERY_REGISTRY: List<QueryDefinition> = listOf(
    // ── Identity ────────────────────────────────────────────────────────
    QueryDefinition(
        name = "getIdentity",
        label = "Get Identity",
        description = "Fetch an identity by its base58 identifier.",
        inputs = listOf(QueryInput("identityId", "Identity ID", required = true)),
        diagnosticInputs = mapOf("identityId" to QueryTestData.TEST_IDENTITY_ID),
    ) { sdk, inputs -> sdk.identities.fetch(inputs.getValue("identityId")) },
    QueryDefinition(
        name = "getIdentityKeys",
        label = "Get Identity Keys",
        description = "Retrieve the public keys associated with an identity.",
        inputs = listOf(QueryInput("identityId", "Identity ID", required = true)),
        diagnosticInputs = mapOf("identityId" to QueryTestData.TEST_IDENTITY_ID),
    ) { sdk, inputs -> sdk.identities.fetchPublicKeys(inputs.getValue("identityId")) },
    QueryDefinition(
        name = "getIdentitiesContractKeys",
        label = "Get Identities Contract Keys",
        description = "Get keys for multiple identities related to a specific contract.",
        inputs = listOf(
            QueryInput("identityIds", "Identity IDs (comma-separated)", required = true),
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("documentType", "Document Type", required = false, placeholder = "domain"),
            QueryInput("purposes", "Purposes (comma-separated)", required = false, placeholder = "0,1,2,3"),
        ),
        diagnosticInputs = mapOf(
            "identityIds" to
                "${QueryTestData.TEST_IDENTITY_ID},${QueryTestData.TEST_IDENTITY_ID_2}",
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
            "documentType" to QueryTestData.TEST_DOCUMENT_TYPE,
            "purposes" to QueryTestData.KEY_PURPOSES_CSV,
        ),
    ) { sdk, inputs ->
        val ids = inputs.getValue("identityIds").split(",")
            .map { it.trim() }.filter { it.isNotEmpty() }
        val purposes = (inputs["purposes"]?.takeIf { it.isNotBlank() } ?: QueryTestData.KEY_PURPOSES_CSV)
            .split(",").mapNotNull { it.trim().toIntOrNull() }
        sdk.identities.fetchContractKeys(
            identityIds = ids,
            contractId = inputs.getValue("contractId"),
            purposes = purposes,
            documentTypeName = inputs["documentType"]?.takeIf { it.isNotBlank() },
        )
    },
    QueryDefinition(
        name = "getIdentityNonce",
        label = "Get Identity Nonce",
        description = "Get the current nonce for an identity.",
        inputs = listOf(QueryInput("identityId", "Identity ID", required = true)),
        diagnosticInputs = mapOf("identityId" to QueryTestData.TEST_IDENTITY_ID),
    ) { sdk, inputs ->
        sdk.identities.fetchNonce(inputs.getValue("identityId"))?.let { "{\"nonce\": $it}" }
    },
    QueryDefinition(
        name = "getIdentityContractNonce",
        label = "Get Identity Contract Nonce",
        description = "Get the nonce for an identity in relation to a specific contract.",
        inputs = listOf(
            QueryInput("identityId", "Identity ID", required = true),
            QueryInput("contractId", "Contract ID", required = true),
        ),
        diagnosticInputs = mapOf(
            "identityId" to QueryTestData.TEST_IDENTITY_ID,
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
        ),
    ) { sdk, inputs ->
        sdk.identities.fetchContractNonce(
            inputs.getValue("identityId"),
            inputs.getValue("contractId"),
        )?.let { "{\"contractNonce\": $it}" }
    },
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
        name = "getIdentitiesBalances",
        label = "Get Identities Balances",
        description = "Get balances for multiple identities (base58 ids).",
        inputs = listOf(
            QueryInput("identityIds", "Identity IDs (comma-separated)", required = true),
        ),
        diagnosticInputs = mapOf(
            "identityIds" to
                "${QueryTestData.TEST_IDENTITY_ID},${QueryTestData.TEST_IDENTITY_ID_2}",
        ),
    ) { sdk, inputs ->
        val idBytes = inputs.getValue("identityIds").split(",")
            .map { it.trim() }.filter { it.isNotEmpty() }
            .mapNotNull { org.dashfoundation.example.util.Base58.decodeIdentifier(it) }
        sdk.identities.fetchBalances(idBytes)
    },
    QueryDefinition(
        name = "getIdentityBalanceAndRevision",
        label = "Get Identity Balance and Revision",
        description = "Get both balance and revision number for an identity.",
        inputs = listOf(QueryInput("identityId", "Identity ID", required = true)),
        diagnosticInputs = mapOf("identityId" to QueryTestData.TEST_IDENTITY_ID),
    ) { sdk, inputs -> sdk.identities.fetchBalanceAndRevision(inputs.getValue("identityId")) },
    QueryDefinition(
        name = "getIdentityByPublicKeyHash",
        label = "Get Identity by Public Key Hash",
        description = "Find an identity by its unique public key hash (hex).",
        inputs = listOf(QueryInput("publicKeyHash", "Public Key Hash (hex)", required = true)),
        diagnosticInputs = mapOf("publicKeyHash" to QueryTestData.TEST_PUBLIC_KEY_HASH),
    ) { sdk, inputs -> sdk.identities.fetchByPublicKeyHash(inputs.getValue("publicKeyHash")) },
    QueryDefinition(
        name = "getIdentityByNonUniquePublicKeyHash",
        label = "Get Identity by Non-Unique Public Key Hash",
        description = "Find identities by a non-unique public key hash (hex).",
        inputs = listOf(
            QueryInput("publicKeyHash", "Public Key Hash (hex)", required = true),
            QueryInput("startAfter", "Start After (base58 id)", required = false),
        ),
        diagnosticInputs = mapOf(
            "publicKeyHash" to QueryTestData.TEST_NON_UNIQUE_PUBLIC_KEY_HASH,
        ),
    ) { sdk, inputs ->
        sdk.identities.fetchByNonUniquePublicKeyHash(
            inputs.getValue("publicKeyHash"),
            inputs["startAfter"]?.takeIf { it.isNotBlank() },
        )
    },

    // ── Data Contract ───────────────────────────────────────────────────
    QueryDefinition(
        name = "getDataContract",
        label = "Get Data Contract",
        description = "Fetch a data contract as JSON by its base58 identifier.",
        inputs = listOf(QueryInput("contractId", "Contract ID", required = true)),
        diagnosticInputs = mapOf("contractId" to QueryTestData.DPNS_CONTRACT_ID),
    ) { sdk, inputs -> sdk.contracts.fetchJson(inputs.getValue("contractId")) },
    QueryDefinition(
        name = "getDataContractWithSerialization",
        label = "Get Data Contract with Serialization",
        description = "Fetch a data contract's JSON plus its canonical serialized bytes.",
        inputs = listOf(QueryInput("contractId", "Contract ID", required = true)),
        diagnosticInputs = mapOf("contractId" to QueryTestData.DPNS_CONTRACT_ID),
    ) { sdk, inputs ->
        sdk.contracts.fetchWithSerialization(inputs.getValue("contractId"))?.let { result ->
            val bytes = result.binarySerialization?.size ?: 0
            "{\"serializedBytes\": $bytes, \"json\": ${result.json}}"
        }
    },

    // ── Documents ───────────────────────────────────────────────────────
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
        name = "getDocument",
        label = "Get Document",
        description = "Fetch a single document by its base58 id.",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("documentType", "Document Type", required = true, placeholder = "domain"),
            QueryInput("documentId", "Document ID", required = true),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
            "documentType" to QueryTestData.TEST_DOCUMENT_TYPE,
            "documentId" to QueryTestData.TEST_DOCUMENT_ID,
        ),
    ) { sdk, inputs ->
        sdk.contracts.fetch(inputs.getValue("contractId")).use { contract ->
            sdk.documents.fetch(
                contract = contract,
                documentType = inputs.getValue("documentType"),
                documentId = inputs.getValue("documentId"),
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

    // ── DPNS ────────────────────────────────────────────────────────────
    QueryDefinition(
        name = "dpnsResolve",
        label = "DPNS Resolve",
        description = "Resolve a DPNS name (e.g. alice.dash) to its record.",
        inputs = listOf(QueryInput("name", "Name", required = true, placeholder = "alice.dash")),
        diagnosticInputs = mapOf("name" to QueryTestData.TEST_USERNAME),
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
        diagnosticInputs = mapOf("identityId" to QueryTestData.TEST_IDENTITY_ID, "limit" to "5"),
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

    // ── Voting & Contested Resources ──────────────────────────────────────
    QueryDefinition(
        name = "getContestedResources",
        label = "Get Contested Resources",
        description = "List contested resources for a contract/document-type/index.",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("documentType", "Document Type", required = true, placeholder = "domain"),
            QueryInput("indexName", "Index Name", required = true, placeholder = "parentNameAndLabel"),
            QueryInput("count", "Count", required = false, placeholder = "5"),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
            "documentType" to QueryTestData.TEST_DOCUMENT_TYPE,
            "indexName" to QueryTestData.CONTESTED_INDEX_NAME,
            "count" to "5",
        ),
    ) { sdk, inputs ->
        sdk.voting.contestedResources(
            contractId = inputs.getValue("contractId"),
            documentTypeName = inputs.getValue("documentType"),
            indexName = inputs.getValue("indexName"),
            count = inputs["count"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
    QueryDefinition(
        name = "getContestedResourceVoteState",
        label = "Get Contested Resource Vote State",
        description = "Get contenders + tallies for a contested resource.",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("documentType", "Document Type", required = true, placeholder = "domain"),
            QueryInput("indexName", "Index Name", required = true, placeholder = "parentNameAndLabel"),
            QueryInput("indexValues", "Index Values (JSON)", required = true),
            QueryInput("resultType", "Result Type (0/1/2)", required = false, placeholder = "2"),
            QueryInput("count", "Count", required = false, placeholder = "5"),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
            "documentType" to QueryTestData.TEST_DOCUMENT_TYPE,
            "indexName" to QueryTestData.CONTESTED_INDEX_NAME,
            "indexValues" to QueryTestData.CONTESTED_INDEX_VALUES_JSON,
            "resultType" to QueryTestData.VOTE_RESULT_DOCUMENTS_AND_TALLY,
            "count" to "5",
        ),
    ) { sdk, inputs ->
        sdk.voting.contestedResourceVoteState(
            contractId = inputs.getValue("contractId"),
            documentTypeName = inputs.getValue("documentType"),
            indexName = inputs.getValue("indexName"),
            indexValuesJson = inputs.getValue("indexValues"),
            resultType = inputs["resultType"]?.trim()?.toIntOrNull() ?: 2,
            count = inputs["count"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
    QueryDefinition(
        name = "getContestedResourceVotersForIdentity",
        label = "Get Contested Resource Voters for Identity",
        description = "Get voters who voted for a contestant in a contested resource.",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("documentType", "Document Type", required = true, placeholder = "domain"),
            QueryInput("indexName", "Index Name", required = true, placeholder = "parentNameAndLabel"),
            QueryInput("indexValues", "Index Values (JSON)", required = true),
            QueryInput("contestantId", "Contestant ID", required = true),
            QueryInput("count", "Count", required = false, placeholder = "5"),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
            "documentType" to QueryTestData.TEST_DOCUMENT_TYPE,
            "indexName" to QueryTestData.CONTESTED_INDEX_NAME,
            "indexValues" to QueryTestData.CONTESTED_INDEX_VALUES_JSON,
            "contestantId" to QueryTestData.TEST_IDENTITY_ID,
            "count" to "5",
        ),
    ) { sdk, inputs ->
        sdk.voting.contestedResourceVotersForIdentity(
            contractId = inputs.getValue("contractId"),
            documentTypeName = inputs.getValue("documentType"),
            indexName = inputs.getValue("indexName"),
            indexValuesJson = inputs.getValue("indexValues"),
            contestantId = inputs.getValue("contestantId"),
            count = inputs["count"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
    QueryDefinition(
        name = "getContestedResourceIdentityVotes",
        label = "Get Contested Resource Identity Votes",
        description = "Get all contested-resource votes cast by an identity.",
        inputs = listOf(
            QueryInput("identityId", "Identity ID", required = true),
            QueryInput("limit", "Limit", required = false, placeholder = "5"),
        ),
        diagnosticInputs = mapOf(
            "identityId" to QueryTestData.TEST_IDENTITY_ID,
            "limit" to "5",
        ),
    ) { sdk, inputs ->
        sdk.voting.contestedResourceIdentityVotes(
            identityId = inputs.getValue("identityId"),
            limit = inputs["limit"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
    QueryDefinition(
        name = "getVotePollsByEndDate",
        label = "Get Vote Polls by End Date",
        description = "Get vote polls whose end date falls in a time window.",
        inputs = listOf(
            QueryInput("startTimeMs", "Start Time (ms)", required = false),
            QueryInput("endTimeMs", "End Time (ms)", required = false),
            QueryInput("limit", "Limit", required = false, placeholder = "5"),
        ),
        diagnosticInputs = mapOf("limit" to "5"),
    ) { sdk, inputs ->
        sdk.voting.votePollsByEndDate(
            startTimeMs = inputs["startTimeMs"]?.trim()?.toLongOrNull() ?: 0,
            endTimeMs = inputs["endTimeMs"]?.trim()?.toLongOrNull() ?: 0,
            limit = inputs["limit"]?.trim()?.toIntOrNull() ?: 0,
        )
    },

    // ── Protocol & Version ────────────────────────────────────────────────
    QueryDefinition(
        name = "getProtocolVersionUpgradeState",
        label = "Get Protocol Version Upgrade State",
        description = "Per-version tallies for the current protocol upgrade window.",
        inputs = emptyList(),
    ) { sdk, _ -> sdk.system.protocolVersionUpgradeState() },
    QueryDefinition(
        name = "getProtocolVersionUpgradeVoteStatus",
        label = "Get Protocol Version Upgrade Vote Status",
        description = "Protocol-version upgrade vote status by evonode.",
        inputs = listOf(
            QueryInput("startProTxHash", "Start Pro-Tx-Hash", required = false),
            QueryInput("count", "Count", required = false, placeholder = "5"),
        ),
        diagnosticInputs = mapOf("count" to "5"),
    ) { sdk, inputs ->
        sdk.system.protocolVersionUpgradeVoteStatus(
            startProTxHash = inputs["startProTxHash"]?.takeIf { it.isNotBlank() },
            count = inputs["count"]?.trim()?.toIntOrNull() ?: 0,
        )
    },

    // ── Epoch & Block ─────────────────────────────────────────────────────
    QueryDefinition(
        name = "getEpochsInfo",
        label = "Get Epochs Info",
        description = "Information about epochs starting from a given epoch.",
        inputs = listOf(
            QueryInput("startEpoch", "Start Epoch", required = false),
            QueryInput("count", "Count", required = false, placeholder = "1"),
        ),
        diagnosticInputs = mapOf("count" to "1"),
    ) { sdk, inputs ->
        sdk.system.epochsInfo(
            startEpoch = inputs["startEpoch"]?.takeIf { it.isNotBlank() },
            count = inputs["count"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
    QueryDefinition(
        name = "getEvonodesProposedEpochBlocksByIds",
        label = "Get Evonodes Proposed Epoch Blocks by IDs",
        description = "Proposed-block counts for an epoch by pro-tx-hash list.",
        inputs = listOf(
            QueryInput("epoch", "Epoch", required = true),
            QueryInput("ids", "Pro-Tx-Hashes (JSON array)", required = true),
        ),
        diagnosticInputs = mapOf(
            "epoch" to "5",
            "ids" to "[\"78adfbe419a528bb0f17e9a31b4ecc4f6b73ad1c97cdcef90f96bb6f0c432c87\"]",
        ),
    ) { sdk, inputs ->
        sdk.evonodes.proposedEpochBlocksByIds(
            epoch = inputs.getValue("epoch").trim().toIntOrNull() ?: 0,
            idsJson = inputs.getValue("ids"),
        )
    },
    QueryDefinition(
        name = "getEvonodesProposedEpochBlocksByRange",
        label = "Get Evonodes Proposed Epoch Blocks by Range",
        description = "Proposed-block counts for an epoch, paginated by range.",
        inputs = listOf(
            QueryInput("epoch", "Epoch", required = true),
            QueryInput("limit", "Limit", required = false, placeholder = "5"),
            QueryInput("startAfter", "Start After (pro-tx-hash)", required = false),
        ),
        diagnosticInputs = mapOf(
            "epoch" to "100",
            "limit" to "5",
            "startAfter" to "85F15A31D3838293A9C1D72A1A0FA21E66110CE20878BD4C1024C4AE1D5BE824",
        ),
    ) { sdk, inputs ->
        sdk.evonodes.proposedEpochBlocksByRange(
            epoch = inputs.getValue("epoch").trim().toIntOrNull() ?: 0,
            limit = inputs["limit"]?.trim()?.toIntOrNull() ?: 0,
            startAfter = inputs["startAfter"]?.takeIf { it.isNotBlank() },
        )
    },

    // ── Token ─────────────────────────────────────────────────────────────
    QueryDefinition(
        name = "calculateTokenId",
        label = "Calculate Token ID",
        description = "Canonical base58 token id for a contract id + token position (offline).",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("position", "Token Position", required = false, placeholder = "0"),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.DPNS_CONTRACT_ID,
            "position" to "0",
        ),
    ) { sdk, inputs ->
        sdk.tokenQueries.calculateTokenId(
            inputs.getValue("contractId"),
            inputs["position"]?.trim()?.toIntOrNull() ?: 0,
        )?.let { "{\"tokenId\": \"$it\"}" }
    },
    QueryDefinition(
        name = "getIdentityTokenBalances",
        label = "Get Identity Token Balances",
        description = "Token balances for an identity across a set of tokens.",
        inputs = listOf(
            QueryInput("identityId", "Identity ID", required = true),
            QueryInput("tokenIds", "Token IDs (comma-separated)", required = true),
        ),
        diagnosticInputs = mapOf(
            "identityId" to QueryTestData.TEST_IDENTITY_ID,
            "tokenIds" to QueryTestData.TEST_TOKEN_ID,
        ),
    ) { sdk, inputs ->
        sdk.tokenQueries.identityTokenBalances(
            inputs.getValue("identityId"),
            inputs.getValue("tokenIds").split(",").map { it.trim() }.filter { it.isNotEmpty() },
        )
    },
    QueryDefinition(
        name = "getTokenStatuses",
        label = "Get Token Statuses",
        description = "On-chain statuses (paused flags) for a set of tokens.",
        inputs = listOf(
            QueryInput("tokenIds", "Token IDs (comma-separated)", required = true),
        ),
        diagnosticInputs = mapOf("tokenIds" to QueryTestData.TEST_TOKEN_ID),
    ) { sdk, inputs ->
        sdk.tokenQueries.statuses(
            inputs.getValue("tokenIds").split(",").map { it.trim() }.filter { it.isNotEmpty() },
        )
    },
    QueryDefinition(
        name = "getTokenDirectPurchasePrices",
        label = "Get Token Direct Purchase Prices",
        description = "Direct-purchase prices for a set of tokens.",
        inputs = listOf(
            QueryInput("tokenIds", "Token IDs (comma-separated)", required = true),
        ),
        diagnosticInputs = mapOf("tokenIds" to QueryTestData.TEST_TOKEN_ID),
    ) { sdk, inputs ->
        sdk.tokenQueries.directPurchasePrices(
            inputs.getValue("tokenIds").split(",").map { it.trim() }.filter { it.isNotEmpty() },
        )
    },
    QueryDefinition(
        name = "getTokenContractInfo",
        label = "Get Token Contract Info",
        description = "The data-contract locator for a single token id.",
        inputs = listOf(QueryInput("tokenId", "Token ID", required = true)),
        diagnosticInputs = mapOf("tokenId" to QueryTestData.TEST_TOKEN_ID),
    ) { sdk, inputs -> sdk.tokenQueries.contractInfo(inputs.getValue("tokenId")) },
    QueryDefinition(
        name = "getTokenPerpetualDistributionLastClaim",
        label = "Get Token Perpetual Distribution Last Claim",
        description = "An identity's last perpetual-distribution claim for a token.",
        inputs = listOf(
            QueryInput("tokenId", "Token ID", required = true),
            QueryInput("identityId", "Identity ID", required = true),
        ),
        diagnosticInputs = mapOf(
            "tokenId" to QueryTestData.TEST_TOKEN_ID,
            "identityId" to QueryTestData.TEST_IDENTITY_ID,
        ),
    ) { sdk, inputs ->
        sdk.tokenQueries.perpetualDistributionLastClaim(
            inputs.getValue("tokenId"),
            inputs.getValue("identityId"),
        )
    },
    QueryDefinition(
        name = "getTokenTotalSupply",
        label = "Get Token Total Supply",
        description = "A token's total supply.",
        inputs = listOf(QueryInput("tokenId", "Token ID", required = true)),
        diagnosticInputs = mapOf("tokenId" to QueryTestData.TEST_TOKEN_ID),
    ) { sdk, inputs -> sdk.tokenQueries.totalSupply(inputs.getValue("tokenId")) },
    QueryDefinition(
        name = "getTokenPreProgrammedDistributions",
        label = "Get Token Pre-Programmed Distributions",
        description = "Pre-programmed distributions for a token, paginated.",
        inputs = listOf(
            QueryInput("tokenId", "Token ID", required = true),
            QueryInput("limit", "Limit", required = false, placeholder = "5"),
        ),
        diagnosticInputs = mapOf(
            "tokenId" to QueryTestData.TEST_TOKEN_ID,
            "limit" to "5",
        ),
    ) { sdk, inputs ->
        sdk.tokenQueries.preProgrammedDistributions(
            tokenId = inputs.getValue("tokenId"),
            limit = inputs["limit"]?.trim()?.toIntOrNull() ?: 0,
        )
    },

    // ── Group ─────────────────────────────────────────────────────────────
    QueryDefinition(
        name = "getGroupInfo",
        label = "Get Group Info",
        description = "Group info (required power + members) at a position.",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("position", "Group Position", required = false, placeholder = "0"),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.TEST_GROUP_CONTRACT_ID,
            "position" to "0",
        ),
    ) { sdk, inputs ->
        sdk.groups.info(
            inputs.getValue("contractId"),
            inputs["position"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
    QueryDefinition(
        name = "getGroupActions",
        label = "Get Group Actions",
        description = "Group actions at a position filtered by status (0 active, 1 closed).",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("position", "Group Position", required = false, placeholder = "0"),
            QueryInput("status", "Status (0/1)", required = false, placeholder = "0"),
            QueryInput("limit", "Limit", required = false, placeholder = "5"),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.TEST_GROUP_CONTRACT_ID,
            "position" to "0",
            "status" to QueryTestData.GROUP_STATUS_ACTIVE,
            "limit" to "5",
        ),
    ) { sdk, inputs ->
        sdk.groups.actions(
            contractId = inputs.getValue("contractId"),
            groupContractPosition = inputs["position"]?.trim()?.toIntOrNull() ?: 0,
            status = inputs["status"]?.trim()?.toIntOrNull() ?: 0,
            limit = inputs["limit"]?.trim()?.toIntOrNull() ?: 0,
        )
    },
    QueryDefinition(
        name = "getGroupActionSigners",
        label = "Get Group Action Signers",
        description = "Signers of a specific group action.",
        inputs = listOf(
            QueryInput("contractId", "Contract ID", required = true),
            QueryInput("position", "Group Position", required = false, placeholder = "0"),
            QueryInput("status", "Status (0/1)", required = false, placeholder = "0"),
            QueryInput("actionId", "Action ID", required = true),
        ),
        diagnosticInputs = mapOf(
            "contractId" to QueryTestData.TEST_GROUP_CONTRACT_ID,
            "position" to "0",
            "status" to QueryTestData.GROUP_STATUS_ACTIVE,
            "actionId" to QueryTestData.TEST_ACTION_ID,
        ),
    ) { sdk, inputs ->
        sdk.groups.actionSigners(
            contractId = inputs.getValue("contractId"),
            groupContractPosition = inputs["position"]?.trim()?.toIntOrNull() ?: 0,
            status = inputs["status"]?.trim()?.toIntOrNull() ?: 0,
            actionId = inputs.getValue("actionId"),
        )
    },

    // ── System & Utility ──────────────────────────────────────────────────
    QueryDefinition(
        name = "getTotalCreditsInPlatform",
        label = "Get Total Credits in Platform",
        description = "Total credits currently in Platform.",
        inputs = emptyList(),
    ) { sdk, _ ->
        sdk.system.totalCreditsInPlatform()?.let { "{\"totalCredits\": $it}" }
    },
    QueryDefinition(
        name = "getCurrentQuorumsInfo",
        label = "Get Current Quorums Info",
        description = "Information about current validator quorums.",
        inputs = emptyList(),
    ) { sdk, _ -> sdk.system.currentQuorumsInfo() },
    QueryDefinition(
        name = "getPrefundedSpecializedBalance",
        label = "Get Prefunded Specialized Balance",
        description = "Balance of a prefunded specialized account by base58 id.",
        inputs = listOf(QueryInput("id", "Balance ID", required = true)),
        diagnosticInputs = mapOf("id" to QueryTestData.TEST_PREFUNDED_SPECIALIZED_BALANCE_ID),
    ) { sdk, inputs -> sdk.system.prefundedSpecializedBalance(inputs.getValue("id")) },
    QueryDefinition(
        name = "getPathElements",
        label = "Get GroveDB Path Elements",
        description = "Fetch raw GroveDB elements by path (diagnostic).",
        inputs = listOf(
            QueryInput("path", "Path (JSON array)", required = true),
            QueryInput("keys", "Keys (JSON array)", required = false),
        ),
        diagnosticInputs = mapOf("path" to QueryTestData.PATH_ELEMENTS_JSON),
    ) { sdk, inputs ->
        sdk.system.groveDbPathElements(
            pathJson = inputs.getValue("path"),
            keysJson = inputs["keys"]?.takeIf { it.isNotBlank() },
        )
    },
)
