package org.dashfoundation.dashsdk.ffi

/**
 * What [DecisionProposalsNative.build] returns: the built proposal document's handle and
 * the 32-byte entropy `dash_sdk_decision_proposal_build` drew for it. Constructed by
 * `rs-unified-sdk-jni` with the descriptor `(J[B)V`.
 *
 * Native frees [documentHandle] itself when it cannot construct or return this object;
 * once Kotlin holds it, Kotlin owns the handle and frees it with
 * [DecisionProposalsNative.documentFree].
 */
class DecisionProposalBuildNativeResult(
    @JvmField val documentHandle: Long,
    @JvmField val entropy: ByteArray,
)

/**
 * Raw JNI surface for decision proposals (protocol version 15): mirrors
 * `rs-unified-sdk-jni/src/decision_proposals.rs`, one export per
 * `dash_sdk_decision_proposal_*` function of `rs-sdk-ffi`, plus the document free its
 * handles need.
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.voting.DecisionProposals] (reads
 * and the masternode vote) and [org.dashfoundation.dashsdk.voting.DecisionProposalTransactions]
 * (create and edit), which sequence these calls; no export stitches two C calls together.
 * Handles are raw Rust pointers as [Long]: `sdk` is `Sdk.handle`, and a document handle
 * from [build], [fetchDocument] or [buildEdit] is freed exactly once with [documentFree].
 * Ids are base58 strings except where a `ByteArray` is named. Errors throw
 * [DashSDKException].
 */
internal object DecisionProposalsNative {

    /** The decision proposals system contract id, base58. */
    external fun contractId(): String

    /** The 32 bytes of the decision proposals system contract id. */
    external fun contractIdBytes(): ByteArray

    /** The document type name of a proposal (`"proposal"`). */
    external fun documentTypeName(): String

    /**
     * Build a decision proposal of [ownerIdentityId] asking [title], with its rule. Nothing
     * is broadcast. Bridges `dash_sdk_decision_proposal_build`.
     *
     * @param url null for none.
     * @param passPercent must fit a `u8`; Platform's limits apply.
     * @param minimumTurnoutPercent must fit a `u8`; Platform's limits apply.
     * @param endDateMs milliseconds since the Unix epoch; must be non-negative.
     * @return the document handle (free with [documentFree]) and its entropy.
     */
    external fun build(
        sdk: Long,
        ownerIdentityId: String,
        title: String,
        description: String,
        url: String?,
        passPercent: Int,
        minimumTurnoutPercent: Int,
        endDateMs: Long,
    ): DecisionProposalBuildNativeResult

    /**
     * The properties of [document] as the JSON object the platform wallet's document create
     * and replace take. Bridges `dash_sdk_decision_proposal_properties_json`; the caller still
     * owns [document].
     */
    external fun propertiesJson(document: Long): String?

    /**
     * The stored proposal [proposalId] as a document handle (free with [documentFree]), or 0
     * when there is none. Bridges `dash_sdk_decision_proposal_fetch_document`.
     */
    external fun fetchDocument(sdk: Long, proposalId: String): Long

    /**
     * A copy of [proposal] with new text and its next revision, as a new document handle
     * (free with [documentFree]). No network. Bridges `dash_sdk_decision_proposal_build_edit`.
     *
     * @param title null keeps it.
     * @param description null keeps it.
     * @param url null keeps it, unless [removeUrl].
     * @param removeUrl removes the url; rs-sdk-ffi refuses it together with a non-null [url].
     */
    external fun buildEdit(
        proposal: Long,
        dataContractId: String,
        documentTypeName: String,
        title: String?,
        description: String?,
        url: String?,
        removeUrl: Boolean,
    ): Long

    /** Free a document handle from [build], [fetchDocument] or [buildEdit]. Safe on 0. */
    external fun documentFree(document: Long)

    /** A proposal as JSON, or null when there is none. Bridges `dash_sdk_decision_proposal_fetch`. */
    external fun fetch(sdk: Long, proposalId: String): String?

    /**
     * One page of proposals by end date, closing soonest first, as a JSON array.
     * [endDateFromMs] is included, [endDateToMs] left out; 0 is no bound. [limit] 0 is 100.
     * Bridges `dash_sdk_decision_proposal_list_by_end_date`.
     */
    external fun listByEndDate(
        sdk: Long,
        endDateFromMs: Long,
        endDateToMs: Long,
        limit: Int,
        startAfter: String?,
    ): String?

    /**
     * One page of [ownerId]'s proposals in filing order, as a JSON array. Bridges
     * `dash_sdk_decision_proposal_list_by_owner`.
     */
    external fun listByOwner(sdk: Long, ownerId: String, limit: Int, startAfter: String?): String?

    /**
     * One page of every proposal, newest first, as a JSON array. Bridges
     * `dash_sdk_decision_proposal_list_by_created_at`.
     */
    external fun listByCreatedAt(sdk: Long, limit: Int, startAfter: String?): String?

    /**
     * The id of [proposalId]'s yes/no poll, as a plain base58 string. Bridges
     * `dash_sdk_decision_proposal_get_vote_poll_id`.
     */
    external fun getVotePollId(sdk: Long, proposalId: String): String?

    /**
     * The status and tallies of [proposalId]'s poll as JSON, or null when it was never
     * opened. Bridges `dash_sdk_decision_proposal_get_vote_poll_state`.
     */
    external fun getVotePollState(sdk: Long, proposalId: String): String?

    /**
     * A masternode's answer on [proposalId] as `{"choice", "timesVoted"}`, or null when it
     * has not voted or the poll finished. Bridges
     * `dash_sdk_decision_proposal_get_masternode_vote`.
     *
     * @param voterProTxHash 32 bytes in wire order, as [castVote] takes it.
     */
    external fun getMasternodeVote(sdk: Long, voterProTxHash: ByteArray, proposalId: String): String?

    /**
     * Vote on [proposalId] for a masternode and wait for the response. Bridges
     * `dash_sdk_decision_proposal_cast_vote`.
     *
     * @param voteChoice `0` yes, `1` no, `2` abstain.
     * @param voterProTxHash 32 bytes in wire order (the reverse of the hex Core displays).
     * @param votingPrivateKey the 32-byte masternode voting private key; not stored.
     * @param networkOrd `Network.ffiValue`.
     */
    external fun castVote(
        sdk: Long,
        proposalId: String,
        voteChoice: Int,
        voterProTxHash: ByteArray,
        votingPrivateKey: ByteArray,
        networkOrd: Int,
    )
}
