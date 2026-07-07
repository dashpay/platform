package org.dashfoundation.dashsdk.queries

import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.NativeCleaner
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

    /** Fetch an identity's public keys (JSON array), or null if absent. */
    suspend fun fetchPublicKeys(identityId: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.identityFetchPublicKeys(sdk.handle, identityId) }
    }

    /** Fetch an identity's balance + revision as a JSON object, or null. */
    suspend fun fetchBalanceAndRevision(identityId: String): String? =
        withContext(Dispatchers.IO) {
            mapNativeErrors { QueriesNative.identityFetchBalanceAndRevision(sdk.handle, identityId) }
        }

    /**
     * Fetch the identity that owns a unique public-key [hashHex] (hex), as
     * JSON, or null if none.
     */
    suspend fun fetchByPublicKeyHash(hashHex: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.identityFetchByPublicKeyHash(sdk.handle, hashHex) }
    }

    /**
     * Fetch identities sharing a non-unique public-key [hashHex] (hex), as a
     * JSON array. [startAfter] (base58 identity id) paginates; null for the
     * first page.
     */
    suspend fun fetchByNonUniquePublicKeyHash(
        hashHex: String,
        startAfter: String? = null,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.identityFetchByNonUniquePublicKeyHash(sdk.handle, hashHex, startAfter)
        }
    }

    /**
     * Fetch balances for many identities. [identityIds] are raw 32-byte ids
     * (the caller owns any base58 decoding). Returns a JSON array
     * `[{"identityIdHex":..,"balance":..,"found":..}]`, or null. Empty input
     * short-circuits to `"[]"`.
     */
    suspend fun fetchBalances(identityIds: List<ByteArray>): String? =
        withContext(Dispatchers.IO) {
            if (identityIds.isEmpty()) return@withContext "[]"
            require(identityIds.all { it.size == 32 }) {
                "each identity id must be exactly 32 bytes"
            }
            val flat = ByteArray(identityIds.size * 32)
            identityIds.forEachIndexed { i, id -> id.copyInto(flat, i * 32) }
            mapNativeErrors { QueriesNative.identitiesFetchBalances(sdk.handle, flat) }
        }

    /** Fetch an identity's current nonce, or null if not found. */
    suspend fun fetchNonce(identityId: String): Long? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.identityFetchNonce(sdk.handle, identityId) }
            ?.toLongOrNull()
    }

    /** Fetch an identity's nonce for a specific contract, or null if not found. */
    suspend fun fetchContractNonce(identityId: String, contractId: String): Long? =
        withContext(Dispatchers.IO) {
            mapNativeErrors {
                QueriesNative.identityFetchContractNonce(sdk.handle, identityId, contractId)
            }?.toLongOrNull()
        }

    /**
     * Contract keys for many identities, as a JSON object keyed by base58
     * identity id. [identityIds] are base58 identity ids; [purposes] are the
     * rs-sdk-ffi purpose discriminants (0 Authentication, 1 Encryption,
     * 2 Decryption, 3 Transfer). [documentTypeName] may be null. Empty
     * [identityIds] short-circuits to `"{}"`. Returns JSON, or null.
     */
    suspend fun fetchContractKeys(
        identityIds: List<String>,
        contractId: String,
        purposes: List<Int>,
        documentTypeName: String? = null,
    ): String? = withContext(Dispatchers.IO) {
        if (identityIds.isEmpty()) return@withContext "{}"
        require(purposes.isNotEmpty()) { "at least one key purpose is required" }
        mapNativeErrors {
            QueriesNative.identitiesFetchContractKeys(
                sdk.handle,
                identityIds.joinToString(","),
                contractId,
                documentTypeName,
                purposes.joinToString(","),
            )
        }
    }
}

/**
 * Platform address queries — mirrors Swift's `sdk.addresses`. Inputs are
 * raw address bytes (21 bytes: type byte + 20-byte hash); the caller owns
 * bech32m/hex decoding, matching Swift's `getInfo(addressBytes:)`.
 */
class Addresses internal constructor(private val sdk: Sdk) {

    /**
     * Fetch a single address's balance + nonce as a JSON object
     * `{"addressHex":..,"nonce":..,"balance":..,"found":..}`, or null.
     */
    suspend fun fetchInfo(address: ByteArray): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.addressFetchInfo(sdk.handle, address) }
    }

    /**
     * Fetch balance + nonce for many addresses as a JSON array. All
     * [addresses] must share the same length (typically 21). Empty input
     * short-circuits to `"[]"`.
     */
    suspend fun fetchInfos(addresses: List<ByteArray>): String? = withContext(Dispatchers.IO) {
        if (addresses.isEmpty()) return@withContext "[]"
        val len = addresses.first().size
        require(addresses.all { it.size == len }) { "all addresses must be the same length" }
        val flat = ByteArray(addresses.size * len)
        addresses.forEachIndexed { i, a -> a.copyInto(flat, i * len) }
        mapNativeErrors { QueriesNative.addressesFetchInfos(sdk.handle, flat, len) }
    }
}

/**
 * Read-only contested-resource and voting queries — mirrors the iOS query
 * catalog (`getContestedResources`, vote state, voters, identity votes,
 * vote polls by end date). Casting a vote is a state transition owned by a
 * sibling module, not this class.
 */
class Voting internal constructor(private val sdk: Sdk) {

    /**
     * Contested resources for a contract/document-type/index. [resultType]
     * is the rs-sdk-ffi u8 enum discriminant; the index-value JSON bounds and
     * [orderAscending] mirror the iOS catalog. Returns JSON, or null.
     */
    suspend fun contestedResources(
        contractId: String,
        documentTypeName: String,
        indexName: String,
        startIndexValuesJson: String? = null,
        endIndexValuesJson: String? = null,
        count: Int = 0,
        orderAscending: Boolean = true,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.contestedResourcesGet(
                sdk.handle,
                contractId,
                documentTypeName,
                indexName,
                startIndexValuesJson,
                endIndexValuesJson,
                count,
                orderAscending,
            )
        }
    }

    /**
     * Vote state (contenders + tallies) for a contested resource.
     * [indexValuesJson] is a JSON array of the index path values;
     * [resultType] is the u8 enum discriminant. Returns JSON, or null.
     */
    suspend fun contestedResourceVoteState(
        contractId: String,
        documentTypeName: String,
        indexName: String,
        indexValuesJson: String,
        resultType: Int,
        allowIncludeLockedAndAbstaining: Boolean = true,
        count: Int = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.contestedResourceVoteState(
                sdk.handle,
                contractId,
                documentTypeName,
                indexName,
                indexValuesJson,
                resultType,
                allowIncludeLockedAndAbstaining,
                count,
            )
        }
    }

    /** Voters who voted for a specific contestant, as JSON, or null. */
    suspend fun contestedResourceVotersForIdentity(
        contractId: String,
        documentTypeName: String,
        indexName: String,
        indexValuesJson: String,
        contestantId: String,
        count: Int = 0,
        orderAscending: Boolean = true,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.contestedResourceVotersForIdentity(
                sdk.handle,
                contractId,
                documentTypeName,
                indexName,
                indexValuesJson,
                contestantId,
                count,
                orderAscending,
            )
        }
    }

    /** All contested-resource votes cast by an identity, as JSON, or null. */
    suspend fun contestedResourceIdentityVotes(
        identityId: String,
        limit: Int = 0,
        offset: Int = 0,
        orderAscending: Boolean = true,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.contestedResourceIdentityVotes(
                sdk.handle, identityId, limit, offset, orderAscending,
            )
        }
    }

    /**
     * Vote polls whose end date falls in `[startTimeMs, endTimeMs]` (bounds
     * inclusive per the `*Included` flags). Returns JSON, or null.
     */
    suspend fun votePollsByEndDate(
        startTimeMs: Long = 0,
        startTimeIncluded: Boolean = true,
        endTimeMs: Long = 0,
        endTimeIncluded: Boolean = true,
        limit: Int = 0,
        offset: Int = 0,
        ascending: Boolean = true,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.votingVotePollsByEndDate(
                sdk.handle,
                startTimeMs,
                startTimeIncluded,
                endTimeMs,
                endTimeIncluded,
                limit,
                offset,
                ascending,
            )
        }
    }
}

/** Evonode proposed-epoch-block queries — mirrors the iOS catalog. */
class Evonodes internal constructor(private val sdk: Sdk) {

    /**
     * Proposed-block counts for [epoch] by pro-tx-hash list. [idsJson] is a
     * JSON array of pro-tx-hash strings. Returns JSON, or null.
     */
    suspend fun proposedEpochBlocksByIds(epoch: Int, idsJson: String): String? =
        withContext(Dispatchers.IO) {
            mapNativeErrors {
                QueriesNative.evonodeProposedEpochBlocksByIds(sdk.handle, epoch, idsJson)
            }
        }

    /**
     * Proposed-block counts for [epoch], paginated by range.
     * [startAfter]/[startAt] (pro-tx-hash) may be null. Returns JSON, or null.
     */
    suspend fun proposedEpochBlocksByRange(
        epoch: Int,
        limit: Int = 0,
        startAfter: String? = null,
        startAt: String? = null,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.evonodeProposedEpochBlocksByRange(
                sdk.handle, epoch, limit, startAfter, startAt,
            )
        }
    }
}

/**
 * System / protocol / epoch queries — GroveDB path elements, prefunded
 * balances, total credits, quorums, epochs, protocol-version upgrade state.
 */
class SystemQueries internal constructor(private val sdk: Sdk) {

    /**
     * GroveDB path-elements query. [pathJson] is a JSON array of the path
     * segments; [keysJson] (a JSON array of keys) may be null. Returns a JSON
     * array of elements, or null. Backs the iOS `GroveDBPathElementsView`.
     */
    suspend fun groveDbPathElements(pathJson: String, keysJson: String?): String? =
        withContext(Dispatchers.IO) {
            mapNativeErrors { QueriesNative.systemGetPathElements(sdk.handle, pathJson, keysJson) }
        }

    /** Prefunded specialized balance for a base58 [id], as JSON, or null. */
    suspend fun prefundedSpecializedBalance(id: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.systemGetPrefundedSpecializedBalance(sdk.handle, id) }
    }

    /** Total credits currently in Platform, or null. */
    suspend fun totalCreditsInPlatform(): Long? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.systemGetTotalCreditsInPlatform(sdk.handle) }
            ?.toLongOrNull()
    }

    /** Current quorums info as a JSON array, or null. */
    suspend fun currentQuorumsInfo(): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.systemGetCurrentQuorumsInfo(sdk.handle) }
    }

    /**
     * Epochs info as a JSON array. [startEpoch] (decimal string) may be null
     * to start from the current epoch. Returns JSON, or null.
     */
    suspend fun epochsInfo(
        startEpoch: String? = null,
        count: Int = 0,
        ascending: Boolean = true,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.systemGetEpochsInfo(sdk.handle, startEpoch, count, ascending)
        }
    }

    /** Protocol-version upgrade state (per-version tallies) as JSON, or null. */
    suspend fun protocolVersionUpgradeState(): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.protocolVersionUpgradeState(sdk.handle) }
    }

    /**
     * Protocol-version upgrade vote status by evonode. [startProTxHash] may
     * be null. Returns JSON, or null.
     */
    suspend fun protocolVersionUpgradeVoteStatus(
        startProTxHash: String? = null,
        count: Int = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.protocolVersionUpgradeVoteStatus(sdk.handle, startProTxHash, count)
        }
    }
}

/** Group (multi-party control) read queries — mirrors the iOS catalog. */
class Groups internal constructor(private val sdk: Sdk) {

    /** Group info (required power + members) at a position, as JSON, or null. */
    suspend fun info(contractId: String, groupContractPosition: Int): String? =
        withContext(Dispatchers.IO) {
            mapNativeErrors {
                QueriesNative.groupGetInfo(sdk.handle, contractId, groupContractPosition)
            }
        }

    /**
     * All groups in a contract as a JSON array. [startAtPosition] (decimal
     * string) may be null. Returns JSON, or null.
     */
    suspend fun infos(startAtPosition: String? = null, limit: Int = 0): String? =
        withContext(Dispatchers.IO) {
            mapNativeErrors { QueriesNative.groupGetInfos(sdk.handle, startAtPosition, limit) }
        }

    /**
     * Group actions at a position filtered by [status] (u8 enum discriminant).
     * [startAtActionId] may be null. Returns JSON, or null.
     */
    suspend fun actions(
        contractId: String,
        groupContractPosition: Int,
        status: Int,
        startAtActionId: String? = null,
        limit: Int = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.groupGetActions(
                sdk.handle, contractId, groupContractPosition, status, startAtActionId, limit,
            )
        }
    }

    /** Signers of a specific group action, as JSON, or null. */
    suspend fun actionSigners(
        contractId: String,
        groupContractPosition: Int,
        status: Int,
        actionId: String,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.groupGetActionSigners(
                sdk.handle, contractId, groupContractPosition, status, actionId,
            )
        }
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
            require(position in 0..0xFFFF) { "position must be in 0..65535, got $position" }
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

    /** A token's total supply, or null. */
    suspend fun totalSupply(tokenId: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.tokenGetTotalSupply(sdk.handle, tokenId) }
    }

    /**
     * An identity's last perpetual-distribution claim for a token, as JSON,
     * or null.
     */
    suspend fun perpetualDistributionLastClaim(
        tokenId: String,
        identityId: String,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.tokenGetPerpetualDistributionLastClaim(sdk.handle, tokenId, identityId)
        }
    }

    /**
     * Direct-purchase prices for [tokenIds] (base58) as JSON, or null. Empty
     * input short-circuits to null.
     */
    suspend fun directPurchasePrices(tokenIds: List<String>): String? =
        withContext(Dispatchers.IO) {
            if (tokenIds.isEmpty()) return@withContext null
            mapNativeErrors {
                QueriesNative.tokenGetDirectPurchasePrices(sdk.handle, tokenIds.joinToString(","))
            }
        }

    /**
     * Pre-programmed distributions for a token, paginated. [startRecipient]
     * (base58) may be null. Returns JSON, or null.
     */
    suspend fun preProgrammedDistributions(
        tokenId: String,
        startTimeMs: Long = 0,
        startRecipient: String? = null,
        startRecipientIncluded: Boolean = true,
        limit: Int = 0,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.tokenGetPreProgrammedDistributions(
                sdk.handle,
                tokenId,
                startTimeMs,
                startRecipient,
                startRecipientIncluded,
                limit,
            )
        }
    }
}

class Contracts internal constructor(private val sdk: Sdk) {

    /** Fetch a data contract as JSON. */
    suspend fun fetchJson(contractId: String): String? = withContext(Dispatchers.IO) {
        mapNativeErrors { QueriesNative.dataContractFetchJson(sdk.handle, contractId) }
    }

    /**
     * Fetch a data contract with both its JSON form and its canonical
     * serialized bytes in a single round-trip, or null if not found.
     * Mirrors Swift's `dataContractGetWithSerialization`.
     */
    suspend fun fetchWithSerialization(contractId: String): ContractWithSerialization? =
        withContext(Dispatchers.IO) {
            val payload = mapNativeErrors {
                QueriesNative.dataContractFetchWithSerialization(sdk.handle, contractId)
            } ?: return@withContext null
            val json = payload.getOrNull(0) as? String ?: return@withContext null
            val serialization = payload.getOrNull(1) as? ByteArray
            ContractWithSerialization(json = json, binarySerialization = serialization)
        }

    /**
     * Fetch a contract as a native handle for document queries. The handle
     * must be [DataContractRef.close]d; prefer `use {}`.
     */
    suspend fun fetch(contractId: String): DataContractRef = withContext(Dispatchers.IO) {
        val handle = mapNativeErrors { QueriesNative.dataContractFetch(sdk.handle, contractId) }
        DataContractRef(handle)
    }

    /**
     * Pre-load known contracts into the SDK's trusted context provider so
     * proof verification resolves them without a network fetch — mirrors
     * Swift's `SDK.loadKnownContracts`. Each pair is
     * `(base58ContractId, versionedBinarySerialization)`; the second element
     * must be the contract's versioned binary serialization (what
     * `DataContract.versionedDeserialize` expects), NOT its JSON. Entries with
     * an empty id or empty bytes are skipped; empty input short-circuits.
     * Throws on error (missing trusted provider or a malformed contract).
     */
    suspend fun loadKnownContracts(contracts: List<Pair<String, ByteArray>>) =
        withContext(Dispatchers.IO) {
            val filtered = contracts.filter { it.first.isNotEmpty() && it.second.isNotEmpty() }
            if (filtered.isEmpty()) return@withContext
            mapNativeErrors {
                QueriesNative.addKnownContracts(
                    sdk.handle,
                    filtered.joinToString(",") { it.first },
                    filtered.map { it.second }.toTypedArray(),
                )
            }
        }
}

/**
 * A data contract fetched with both representations — port of the Swift
 * `dataContractGetWithSerialization` result. [json] is the contract's JSON
 * form; [binarySerialization] is its canonical serialized bytes (null when
 * the network omitted them).
 */
data class ContractWithSerialization(
    val json: String,
    val binarySerialization: ByteArray?,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is ContractWithSerialization) return false
        if (json != other.json) return false
        if (binarySerialization != null) {
            if (other.binarySerialization == null) return false
            if (!binarySerialization.contentEquals(other.binarySerialization)) return false
        } else if (other.binarySerialization != null) {
            return false
        }
        return true
    }

    override fun hashCode(): Int {
        var result = json.hashCode()
        result = 31 * result + (binarySerialization?.contentHashCode() ?: 0)
        return result
    }
}

/** Owned native data-contract handle. */
class DataContractRef internal constructor(handle: Long) : AutoCloseable {
    private val handleRef = AtomicLong(handle)
    private val cleanable = NativeCleaner.register(this, HandleCleanup(handleRef))

    internal val value: Long
        get() = handleRef.get().also { check(it != 0L) { "DataContractRef has been closed" } }

    /** Idempotent: destroys the handle exactly once, on [close] or the GC backstop. */
    override fun close() {
        cleanable.clean()
    }

    /** Runs on [NativeCleaner] or [close]; destroys the handle exactly once. */
    private class HandleCleanup(private val handleRef: AtomicLong) : Runnable {
        override fun run() {
            val h = handleRef.getAndSet(0)
            if (h != 0L) QueriesNative.dataContractDestroy(h)
        }
    }
}

class Documents internal constructor(private val sdk: Sdk) {

    /**
     * Fetch a single document by id as its JSON object, or null if not
     * found. Implemented over the bridged [search] with a `$id = documentId`
     * where clause (rs-sdk-ffi exposes single-document fetch only as an
     * opaque handle, which has no JSON serializer across the FFI). The
     * document id is base58, so it needs no JSON escaping. Returns the first
     * matching document object, unwrapped from the search array, or null.
     */
    suspend fun fetch(
        contract: DataContractRef,
        documentType: String,
        documentId: String,
    ): String? = withContext(Dispatchers.IO) {
        val whereJson = """[{"field":"${'$'}id","operator":"=","value":"$documentId"}]"""
        val array = mapNativeErrors {
            QueriesNative.documentSearch(
                sdk.handle, contract.value, documentType, whereJson, null, 1, 0,
            )
        } ?: return@withContext null
        // Unwrap the single element from the JSON array `[ {...} ]`. Empty
        // (`[]`) means not found → null. Kept as a raw substring so this
        // wrapper stays serialization-library-free like its siblings.
        val trimmed = array.trim()
        if (!trimmed.startsWith("[")) return@withContext null
        val inner = trimmed.removePrefix("[").removeSuffix("]").trim()
        if (inner.isEmpty()) null else inner
    }

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
        // -1 = server default (no cap). Drive rejects an explicit limit of 0
        // on aggregate queries ("zero-cap query is structurally meaningless"),
        // and these forms have no limit control, so default to the server cap.
        limit: Long = -1,
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
        // -1 = server default (no cap). Drive rejects an explicit limit of 0
        // on aggregate queries ("zero-cap query is structurally meaningless"),
        // and these forms have no limit control, so default to the server cap.
        limit: Long = -1,
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
        // -1 = server default (no cap). Drive rejects an explicit limit of 0
        // on aggregate queries ("zero-cap query is structurally meaningless"),
        // and these forms have no limit control, so default to the server cap.
        limit: Long = -1,
    ): String? = withContext(Dispatchers.IO) {
        mapNativeErrors {
            QueriesNative.documentAverage(
                sdk.handle, contract.value, documentType, property,
                whereJson, orderByJson, groupByJson, limit,
            )
        }
    }
}
