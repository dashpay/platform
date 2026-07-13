package org.dashfoundation.dashsdk.voting

import org.dashfoundation.dashsdk.wallet.op

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.TransactionsNative

/**
 * A contested-resource vote choice — Kotlin mirror of Swift's
 * `ContestedResourceVoteChoice`. The [discriminant] matches the FFI's
 * `vote_choice` byte (0 TowardsIdentity, 1 Abstain, 2 Lock).
 */
sealed class VoteChoice(val discriminant: Int) {
    /** Vote for a specific contender (base58 identity id). */
    data class TowardsIdentity(val contenderIdentityId: String) : VoteChoice(0)

    /** Abstain from the contest. */
    data object Abstain : VoteChoice(1)

    /** Vote that nobody should win the contested resource. */
    data object Lock : VoteChoice(2)
}

/**
 * Masternode contested-resource vote bridge — port of the vote-casting slice
 * of `SDK.castContestedResourceVote` (driven by Swift `ContestDetailView`).
 * Thin wrapper over [TransactionsNative]; no orchestration lives here (see
 * `packages/kotlin-sdk/CLAUDE.md`). The SDK handle is supplied by the caller
 * (`Sdk.handle`); the ECDSA_HASH160 voting key and signer are derived
 * Rust-side from [votingPrivateKey], so the key bytes are not stored.
 */
class VoteCasting internal constructor() {

    /**
     * Cast a masternode contested-resource vote and wait for the response —
     * mirrors Swift `SDK.castContestedResourceVote`'s parameter order.
     *
     * Builds a `ResourceVote` over the vote poll `(dataContractId,
     * documentTypeName, indexName, indexValues)` with [choice], signs it with
     * the masternode voting key derived from [votingPrivateKey], and
     * broadcasts it for [proTxHash].
     *
     * A vote from a non-masternode wallet is expected to be rejected by
     * Platform with an authorization-style error — the correct deterministic
     * outcome (surfaced as a thrown exception).
     *
     * @param indexValues JSON-encodable index values (DPNS: `["dash",
     *   label]`). Text values are taken verbatim; a `"0x"`-prefixed value is
     *   hex-decoded Rust-side.
     * @param proTxHash 32-byte masternode pro_tx_hash.
     * @param votingPrivateKey 32-byte masternode voting private key.
     * @param networkOrd `Network.ffiValue` (0 Mainnet, 1 Testnet, 2 Devnet,
     *   3 Regtest).
     */
    suspend fun castVote(
        sdk: org.dashfoundation.dashsdk.Sdk,
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        indexValues: List<String>,
        choice: VoteChoice,
        proTxHash: ByteArray,
        votingPrivateKey: ByteArray,
        networkOrd: Int,
    ) = sdk.queryGate.op {
        require(proTxHash.size == 32) { "proTxHash must be 32 bytes, got ${proTxHash.size}" }
        require(votingPrivateKey.size == 32) {
            "votingPrivateKey must be 32 bytes, got ${votingPrivateKey.size}"
        }
        val contender = (choice as? VoteChoice.TowardsIdentity)?.contenderIdentityId
        mapNativeErrors {
            TransactionsNative.castContestedResourceVote(
                sdk.handle,
                dataContractId,
                documentTypeName,
                indexName,
                encodeIndexValues(indexValues),
                choice.discriminant,
                contender,
                proTxHash,
                votingPrivateKey,
                networkOrd,
            )
        }
    }

    /**
     * Encode [values] as a JSON array of strings — the shape
     * `dash_sdk_contested_resource_cast_vote` parses (each element decoded
     * verbatim as text, or hex when `0x`-prefixed). Minimal escaping (quote +
     * backslash) matches the DPNS text-label inputs the vote poll uses.
     */
    private fun encodeIndexValues(values: List<String>): String =
        values.joinToString(prefix = "[", postfix = "]", separator = ",") { v ->
            val escaped = v.replace("\\", "\\\\").replace("\"", "\\\"")
            "\"$escaped\""
        }
}
