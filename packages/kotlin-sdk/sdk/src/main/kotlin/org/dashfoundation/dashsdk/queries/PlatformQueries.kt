package org.dashfoundation.dashsdk.queries

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.QueriesNative

/**
 * Read-only Platform query surface, grouped like the Swift SDK's lazy
 * sub-objects (`sdk.identities`, …). All calls block a worker thread at the
 * FFI boundary (the Rust side runs them on its Tokio runtime), so every
 * entry point is a suspend function on [Dispatchers.IO].
 *
 * Results are the raw JSON strings from Rust — the persistence layer and
 * ViewModels decode them with kotlinx.serialization as needed, mirroring
 * how the Swift wrappers hand `Data`/JSON up to their callers.
 */
class Identities internal constructor(private val sdk: Sdk) {

    /** Fetch an identity as JSON, or null if it doesn't exist. */
    suspend fun fetch(identityId: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.identityFetch(sdk.handle, identityId) }
    }

    /** Fetch an identity's balance in credits. */
    suspend fun fetchBalance(identityId: String): Long? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.identityFetchBalance(sdk.handle, identityId) }
            ?.toLongOrNull()
    }
}

class Dpns internal constructor(private val sdk: Sdk) {

    /** Resolve a DPNS name to its record (JSON), or null if unregistered. */
    suspend fun resolve(name: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.dpnsResolve(sdk.handle, name) }
    }

    /** Check label availability; returns the JSON availability object. */
    suspend fun checkAvailability(label: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.dpnsCheckAvailability(sdk.handle, label) }
    }

    /** Usernames owned by an identity (JSON array). */
    suspend fun usernames(identityId: String, limit: Int = 0): String? =
        withContext(Dispatchers.IO) {
            mapNativeErrors { QueriesNative.dpnsGetUsernames(sdk.handle, identityId, limit) }
        }

    /** Search names by prefix (JSON array). */
    suspend fun search(prefix: String, limit: Int = 0): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.dpnsSearch(sdk.handle, prefix, limit) }
    }
}

class TokenQueries internal constructor(private val sdk: Sdk) {

    /**
     * Canonical base58 token id for a contract id + token position. Pure
     * computation — no network. Returns null on a malformed contract id.
     * Mirrors Swift's `calculateTokenId(contractId:position:)`.
     */
    suspend fun calculateTokenId(contractId: String, position: Int): String? =
        withContext(Dispatchers.IO) {
            mapNativeErrors { QueriesNative.calculateTokenId(contractId, position) }
        }

    /**
     * Identity token balances as JSON `{"<base58Id>": <u64>, ...}`.
     * [tokenIds] are base58 token ids. Null if the query fails.
     */
    suspend fun identityTokenBalances(
        identityId: String,
        tokenIds: List<String>,
    ): String? = withContext(Dispatchers.IO) {
        if (tokenIds.isEmpty()) return@withContext null
        mapNativeErrors {
            QueriesNative.identityFetchTokenBalances(
                sdk.handle, identityId, tokenIds.joinToString(","),
            )
        }
    }

    /**
     * On-chain token statuses as JSON `{"<base58Id>": {"paused": bool}, ...}`.
     * [tokenIds] are base58 token ids. Null if the query fails.
     */
    suspend fun statuses(tokenIds: List<String>): String? = withContext(Dispatchers.IO) {
        if (tokenIds.isEmpty()) return@withContext null
        mapNativeErrors { QueriesNative.tokenGetStatuses(sdk.handle, tokenIds.joinToString(",")) }
    }

    /**
     * Data-contract locator for a single base58 [tokenId] as JSON
     * `{"contract_id": "<base58>", "token_contract_position": <u16>}`, or
     * null if the token is unknown.
     */
    suspend fun contractInfo(tokenId: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.tokenGetContractInfo(sdk.handle, tokenId) }
    }
}

class Contracts internal constructor(private val sdk: Sdk) {

    /** Fetch a data contract as JSON. */
    suspend fun fetchJson(contractId: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.dataContractFetchJson(sdk.handle, contractId) }
    }

    /**
     * Fetch a contract as a native handle for document queries. The handle
     * must be [DataContractRef.close]d; prefer `use {}`.
     */
    suspend fun fetch(contractId: String): DataContractRef = withContext(Dispatchers.IO) {
        val handle = mapNativeErrors { QueriesNative.dataContractFetch(sdk.handle, contractId) }
        DataContractRef(handle)
    }
}

/** Owned native data-contract handle. */
class DataContractRef internal constructor(handle: Long) : AutoCloseable {
    private var handle: Long = handle

    internal val value: Long
        get() = handle.also { check(it != 0L) { "DataContractRef has been closed" } }

    override fun close() {
        val h = handle
        handle = 0
        if (h != 0L) QueriesNative.dataContractDestroy(h)
    }
}

class Documents internal constructor(private val sdk: Sdk) {

    /** Search documents; returns a JSON array. */
    suspend fun search(
        contract: DataContractRef,
        documentType: String,
        whereJson: String? = null,
        orderByJson: String? = null,
        limit: Int = 0,
        startAt: Int = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.documentSearch(
                sdk.handle, contract.value, documentType, whereJson, orderByJson, limit, startAt,
            )
        }
    }

    /** Count documents; returns the JSON count result. */
    suspend fun count(
        contract: DataContractRef,
        documentType: String,
        whereJson: String? = null,
        orderByJson: String? = null,
        groupByJson: String? = null,
        limit: Long = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.documentCount(
                sdk.handle, contract.value, documentType, whereJson, orderByJson, groupByJson, limit,
            )
        }
    }

    /** Sum a numeric property; returns the JSON sum result. */
    suspend fun sum(
        contract: DataContractRef,
        documentType: String,
        property: String,
        whereJson: String? = null,
        orderByJson: String? = null,
        groupByJson: String? = null,
        limit: Long = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.documentSum(
                sdk.handle, contract.value, documentType, property,
                whereJson, orderByJson, groupByJson, limit,
            )
        }
    }

    /** Average a numeric property; returns the JSON average result. */
    suspend fun average(
        contract: DataContractRef,
        documentType: String,
        property: String,
        whereJson: String? = null,
        orderByJson: String? = null,
        groupByJson: String? = null,
        limit: Long = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.documentAverage(
                sdk.handle, contract.value, documentType, property,
                whereJson, orderByJson, groupByJson, limit,
            )
        }
    }
}
