package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for decision proposals (protocol version 15): mirrors
 * `rs-unified-sdk-jni/src/decision_proposals.rs`, one export per C function: each
 * `dash_sdk_decision_proposal_*` function of `rs-sdk-ffi` (the contract's names, the reads
 * and the masternode vote), and `platform_wallet_decision_proposal_create` / `_edit` of
 * `platform-wallet-ffi` (the create and the edit).
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.voting.DecisionProposals] (reads
 * and the masternode vote) and [org.dashfoundation.dashsdk.voting.DecisionProposalTransactions]
 * (create and edit), one native call each; no export stitches two C calls together. `sdk`
 * is `Sdk.handle`, `walletHandle` a `ManagedPlatformWallet` handle and `signerHandle` the
 * manager's `PlatformWalletManager.signerHandle`. Ids are base58 strings except where a
 * `ByteArray` is named. Errors throw [DashSDKException].
 */
internal object DecisionProposalsNative {

    /** The decision proposals system contract id, base58. */
    external fun contractId(): String

    /** The 32 bytes of the decision proposals system contract id. */
    external fun contractIdBytes(): ByteArray

    /** The document type name of a proposal (`"proposal"`). */
    external fun documentTypeName(): String

    /**
     * Create + broadcast a decision proposal of [ownerId] asking [title], with its rule,
     * signed via [signerHandle]. Bridges `platform_wallet_decision_proposal_create`: the
     * wallet builds the proposal, whose create states the decision proposal fund (paid by
     * the owner on top of the fees), picks the signing key, broadcasts and waits for
     * Platform to confirm.
     *
     * @param ownerId the proposer's 32-byte identity id.
     * @param url null for none.
     * @param passPercent must fit a `u8`; Platform's limits apply.
     * @param minimumTurnoutPercent must fit a `u8`; Platform's limits apply.
     * @param endDateMs milliseconds since the Unix epoch; must be non-negative.
     * @param outProposalId a `ByteArray(32)`, checked before anything is broadcast, that
     *   receives the proposal's id.
     * @return the confirmed document's canonical JSON.
     */
    external fun create(
        walletHandle: Long,
        ownerId: ByteArray,
        title: String,
        description: String,
        url: String?,
        passPercent: Int,
        minimumTurnoutPercent: Int,
        endDateMs: Long,
        signerHandle: Long,
        outProposalId: ByteArray,
    ): String

    /**
     * Give the proposal [proposalId] of [ownerId] new text and broadcast the replace, signed
     * via [signerHandle]. Bridges `platform_wallet_decision_proposal_edit`: the wallet reads
     * the proposal once, edits it at the revision after the one read and picks the signing
     * key as [create] does.
     *
     * @param ownerId the proposer's 32-byte identity id.
     * @param proposalId the proposal's 32-byte id.
     * @param title null keeps it.
     * @param description null keeps it.
     * @param url null keeps it, unless [removeUrl].
     * @param removeUrl removes the url; the wallet refuses it together with a non-null [url].
     * @return the confirmed document's canonical JSON, at its new revision.
     */
    external fun edit(
        walletHandle: Long,
        ownerId: ByteArray,
        proposalId: ByteArray,
        title: String?,
        description: String?,
        url: String?,
        removeUrl: Boolean,
        signerHandle: Long,
    ): String

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
