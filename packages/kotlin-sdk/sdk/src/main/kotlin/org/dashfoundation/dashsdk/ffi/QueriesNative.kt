package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for read-only Platform queries — mirrors
 * `rs-unified-sdk-jni/src/queries.rs`. All payloads are JSON strings
 * produced by the Rust layer; parsing happens in the public wrappers.
 */
internal object QueriesNative {

    /** Identity as JSON, or null if it does not exist. */
    external fun identityFetch(sdk: Long, identityId: String): String?

    /** Identity balance in credits as a decimal string. */
    external fun identityFetchBalance(sdk: Long, identityId: String): String?

    /** DPNS name record as JSON, or null if unregistered. */
    external fun dpnsResolve(sdk: Long, name: String): String?

    /** JSON availability object for a DPNS label. */
    external fun dpnsCheckAvailability(sdk: Long, label: String): String?

    /** JSON array of usernames owned by an identity. */
    external fun dpnsGetUsernames(sdk: Long, identityId: String, limit: Int): String?

    /** JSON array of DPNS names matching a prefix. */
    external fun dpnsSearch(sdk: Long, prefix: String, limit: Int): String?

    /** Data contract as JSON. */
    external fun dataContractFetchJson(sdk: Long, contractId: String): String?

    /** Opaque data-contract handle for document queries; release with [dataContractDestroy]. */
    external fun dataContractFetch(sdk: Long, contractId: String): Long

    /** Release a handle from [dataContractFetch]. Safe on 0. */
    external fun dataContractDestroy(handle: Long)

    /** JSON array of documents. whereJson/orderByJson may be null. */
    external fun documentSearch(
        sdk: Long,
        contractHandle: Long,
        documentType: String,
        whereJson: String?,
        orderByJson: String?,
        limit: Int,
        startAt: Int,
    ): String?

    /** JSON count result (optionally grouped). */
    external fun documentCount(
        sdk: Long,
        contractHandle: Long,
        documentType: String,
        whereJson: String?,
        orderByJson: String?,
        groupByJson: String?,
        limit: Long,
    ): String?

    /** JSON sum result over a numeric property. */
    external fun documentSum(
        sdk: Long,
        contractHandle: Long,
        documentType: String,
        sumProperty: String,
        whereJson: String?,
        orderByJson: String?,
        groupByJson: String?,
        limit: Long,
    ): String?

    /** JSON average result over a numeric property. */
    external fun documentAverage(
        sdk: Long,
        contractHandle: Long,
        documentType: String,
        sumProperty: String,
        whereJson: String?,
        orderByJson: String?,
        groupByJson: String?,
        limit: Long,
    ): String?

    /**
     * Canonical base58 token id for a contract id + token position. Pure
     * computation (no SDK handle). Returns the base58 token id or null.
     */
    external fun calculateTokenId(contractId: String, position: Int): String?

    /**
     * Identity token balances as JSON `{"<base58Id>": <u64>, ...}` for the
     * comma-separated base58 [tokenIds]. Null if the query fails.
     */
    external fun identityFetchTokenBalances(
        sdk: Long,
        identityId: String,
        tokenIds: String,
    ): String?

    /**
     * On-chain token statuses as JSON `{"<base58Id>": {"paused": bool}, ...}`
     * for the comma-separated base58 [tokenIds]. Null if the query fails.
     */
    external fun tokenGetStatuses(sdk: Long, tokenIds: String): String?

    /**
     * Data-contract locator for a single base58 [tokenId] as JSON
     * `{"contract_id": "<base58>", "token_contract_position": <u16>}`, or
     * null if the token is unknown.
     */
    external fun tokenGetContractInfo(sdk: Long, tokenId: String): String?
}
