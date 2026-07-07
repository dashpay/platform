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

    /**
     * Fetch a data contract with both its JSON form and canonical serialized
     * bytes in one round-trip. Returns a two-element `Object[]` =
     * `[String? json, ByteArray? serialization]`, or null after throwing.
     * The native contract handle carried by the FFI result is freed on the
     * Rust side — this query surfaces the contract as data, not as a
     * document-query handle.
     */
    external fun dataContractFetchWithSerialization(sdk: Long, contractId: String): Array<Any?>?

    /** Release a handle from [dataContractFetch]. Safe on 0. */
    external fun dataContractDestroy(handle: Long)

    /**
     * Pre-load data contracts into the SDK's trusted context provider so
     * proof verification resolves them without a network fetch. [contractIds]
     * is a comma-separated list of base58 contract ids; [serializedContracts]
     * holds each contract's versioned binary serialization in the SAME order.
     * Empty input is a no-op. Throws on error (missing trusted provider,
     * malformed contract, or id/contract count mismatch).
     */
    external fun addKnownContracts(
        sdk: Long,
        contractIds: String,
        serializedContracts: Array<ByteArray>,
    )

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

    // ── Identity: keys, revision, by-pubkey-hash, batch balances ──────────

    /** Identity public keys as a JSON array, or null. */
    external fun identityFetchPublicKeys(sdk: Long, identityId: String): String?

    /** Identity balance + revision as a JSON object, or null. */
    external fun identityFetchBalanceAndRevision(sdk: Long, identityId: String): String?

    /** Identity owning a unique public-key hash (hex) as JSON, or null. */
    external fun identityFetchByPublicKeyHash(sdk: Long, publicKeyHash: String): String?

    /**
     * Identities sharing a non-unique public-key hash (hex) as a JSON array.
     * [startAfter] (base58 identity id) may be null for the first page.
     */
    external fun identityFetchByNonUniquePublicKeyHash(
        sdk: Long,
        publicKeyHash: String,
        startAfter: String?,
    ): String?

    /**
     * Balances for many identities. [identityIds] is a flat `byte[]` of
     * `n * 32` bytes (each raw 32-byte id concatenated). Returns a JSON
     * array `[{"identityIdHex":..,"balance":..,"found":..}]`, or null.
     */
    external fun identitiesFetchBalances(sdk: Long, identityIds: ByteArray): String?

    /** An identity's current nonce as a decimal string, or null. */
    external fun identityFetchNonce(sdk: Long, identityId: String): String?

    /** An identity's nonce for a contract as a decimal string, or null. */
    external fun identityFetchContractNonce(
        sdk: Long,
        identityId: String,
        contractId: String,
    ): String?

    /**
     * Contract keys for many identities as a JSON object keyed by base58 id,
     * or null. [identityIds] and [purposes] are comma-separated lists;
     * [documentTypeName] may be null.
     */
    external fun identitiesFetchContractKeys(
        sdk: Long,
        identityIds: String,
        contractId: String,
        documentTypeName: String?,
        purposes: String,
    ): String?

    // ── Addresses ─────────────────────────────────────────────────────────

    /**
     * Single Platform address info as JSON
     * `{"addressHex":..,"nonce":..,"balance":..,"found":..}`. [address] is
     * the raw 21-byte address. Null after throwing.
     */
    external fun addressFetchInfo(sdk: Long, address: ByteArray): String?

    /**
     * Many Platform addresses' info as a JSON array. [addresses] is a flat
     * `byte[]` of `count * addressLen` bytes (each address the same length).
     * Null after throwing.
     */
    external fun addressesFetchInfos(sdk: Long, addresses: ByteArray, addressLen: Int): String?

    // ── Contested resources / voting (read-only) ──────────────────────────

    /** Contested resources for a contract/type/index as JSON, or null. */
    external fun contestedResourcesGet(
        sdk: Long,
        contractId: String,
        documentTypeName: String,
        indexName: String,
        startIndexValuesJson: String?,
        endIndexValuesJson: String?,
        count: Int,
        orderAscending: Boolean,
    ): String?

    /** Contested-resource vote state (contenders + tallies) as JSON, or null. */
    external fun contestedResourceVoteState(
        sdk: Long,
        contractId: String,
        documentTypeName: String,
        indexName: String,
        indexValuesJson: String,
        resultType: Int,
        allowIncludeLockedAndAbstaining: Boolean,
        count: Int,
    ): String?

    /** Voters for a contestant on a contested resource as JSON, or null. */
    external fun contestedResourceVotersForIdentity(
        sdk: Long,
        contractId: String,
        documentTypeName: String,
        indexName: String,
        indexValuesJson: String,
        contestantId: String,
        count: Int,
        orderAscending: Boolean,
    ): String?

    /** All contested-resource votes cast by an identity as JSON, or null. */
    external fun contestedResourceIdentityVotes(
        sdk: Long,
        identityId: String,
        limit: Int,
        offset: Int,
        orderAscending: Boolean,
    ): String?

    /** Vote polls whose end date is in a time window, as JSON, or null. */
    external fun votingVotePollsByEndDate(
        sdk: Long,
        startTimeMs: Long,
        startTimeIncluded: Boolean,
        endTimeMs: Long,
        endTimeIncluded: Boolean,
        limit: Int,
        offset: Int,
        ascending: Boolean,
    ): String?

    // ── Evonode ───────────────────────────────────────────────────────────

    /** Evonode proposed-block counts for an epoch by id list (JSON), or null. */
    external fun evonodeProposedEpochBlocksByIds(sdk: Long, epoch: Int, idsJson: String): String?

    /** Evonode proposed-block counts for an epoch by range (JSON), or null. */
    external fun evonodeProposedEpochBlocksByRange(
        sdk: Long,
        epoch: Int,
        limit: Int,
        startAfter: String?,
        startAt: String?,
    ): String?

    // ── Protocol version ──────────────────────────────────────────────────

    /** Protocol-version upgrade state (per-version tallies) as JSON, or null. */
    external fun protocolVersionUpgradeState(sdk: Long): String?

    /** Protocol-version upgrade vote status by evonode as JSON, or null. */
    external fun protocolVersionUpgradeVoteStatus(
        sdk: Long,
        startProTxHash: String?,
        count: Int,
    ): String?

    // ── System ────────────────────────────────────────────────────────────

    /**
     * GroveDB path-elements query. [pathJson] is a JSON array of path
     * segments; [keysJson] (JSON array of keys) may be null. Returns a JSON
     * array of elements, or null.
     */
    external fun systemGetPathElements(sdk: Long, pathJson: String, keysJson: String?): String?

    /** Prefunded specialized balance for a base58 id, as JSON, or null. */
    external fun systemGetPrefundedSpecializedBalance(sdk: Long, id: String): String?

    /** Total credits in Platform as a decimal string, or null. */
    external fun systemGetTotalCreditsInPlatform(sdk: Long): String?

    /** Current quorums info as a JSON array, or null. */
    external fun systemGetCurrentQuorumsInfo(sdk: Long): String?

    /**
     * Epochs info as a JSON array. [startEpoch] (decimal string) may be null
     * to start from the current epoch. Returns JSON, or null.
     */
    external fun systemGetEpochsInfo(
        sdk: Long,
        startEpoch: String?,
        count: Int,
        ascending: Boolean,
    ): String?

    // ── Group ─────────────────────────────────────────────────────────────

    /** Group info (required power + members) at a position, as JSON, or null. */
    external fun groupGetInfo(sdk: Long, contractId: String, groupContractPosition: Int): String?

    /**
     * All groups in a contract as a JSON array. [startAtPosition] (decimal
     * string) may be null. Returns JSON, or null.
     */
    external fun groupGetInfos(sdk: Long, startAtPosition: String?, limit: Int): String?

    /**
     * Group actions at a position filtered by [status] (u8 enum discriminant).
     * [startAtActionId] may be null. Returns JSON, or null.
     */
    external fun groupGetActions(
        sdk: Long,
        contractId: String,
        groupContractPosition: Int,
        status: Int,
        startAtActionId: String?,
        limit: Int,
    ): String?

    /** Signers of a specific group action as JSON, or null. */
    external fun groupGetActionSigners(
        sdk: Long,
        contractId: String,
        groupContractPosition: Int,
        status: Int,
        actionId: String,
    ): String?

    // ── Tokens (additional read queries) ──────────────────────────────────

    /** A token's total supply as a decimal string, or null. */
    external fun tokenGetTotalSupply(sdk: Long, tokenId: String): String?

    /** An identity's last perpetual-distribution claim for a token (JSON), or null. */
    external fun tokenGetPerpetualDistributionLastClaim(
        sdk: Long,
        tokenId: String,
        identityId: String,
    ): String?

    /** Direct-purchase prices for a comma-separated base58 token-id list (JSON), or null. */
    external fun tokenGetDirectPurchasePrices(sdk: Long, tokenIds: String): String?

    /**
     * Pre-programmed distributions for a token, paginated. [startRecipient]
     * (base58) may be null. Returns JSON, or null.
     */
    external fun tokenGetPreProgrammedDistributions(
        sdk: Long,
        tokenId: String,
        startTimeMs: Long,
        startRecipient: String?,
        startRecipientIncluded: Boolean,
        limit: Int,
    ): String?
}
