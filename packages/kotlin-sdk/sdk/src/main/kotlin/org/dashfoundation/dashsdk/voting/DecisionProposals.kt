package org.dashfoundation.dashsdk.voting

import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.DecisionProposalsNative
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.wallet.TeardownGate
import org.dashfoundation.dashsdk.wallet.op

/**
 * Decision proposals (protocol version 15): yes/no questions any identity puts to the
 * masternodes, as `proposal` documents of the decision proposals system contract. Reads of
 * proposals and their polls, and the masternode vote. Kotlin counterpart of the Swift
 * SDK's `SDK+DecisionProposals.swift`; reach it through `Sdk.decisionProposals`. Creating
 * and editing a proposal go through the wallet: [DecisionProposalTransactions].
 *
 * Thin wrapper over [DecisionProposalsNative]: rs-sdk decides everything (which documents
 * are proposals, the poll's id, the vote), every read is proved, and nothing is checked
 * here. Every call runs under the SDK's query gate, so closing the SDK waits for it. Ids
 * are base58 strings. A network below protocol version 15 has no decision proposals
 * contract, which the calls report as an error.
 */
class DecisionProposals internal constructor(private val sdk: Sdk) {

    /** The decision proposals system contract id, base58. */
    val contractId: String by lazy { DecisionProposalsNative.contractId() }

    /** The document type name of a proposal. */
    val documentTypeName: String by lazy { DecisionProposalsNative.documentTypeName() }

    /** The proposal [proposalId], or null when there is none. */
    suspend fun fetch(proposalId: String): DecisionProposal? = sdk.queryGate.op {
        mapNativeErrors { DecisionProposalsNative.fetch(sdk.handle, proposalId) }
            ?.let(DecisionProposal::fromJson)
    }

    /**
     * One page of the proposals whose end date is within the bounds, closing soonest first
     * (by end date, then by creation time). The current time as [endDateFromMs] lists the
     * open proposals.
     *
     * @param endDateFromMs the earliest end date listed, included; 0 for no lower bound.
     * @param endDateToMs the end date the listing stops before; 0 for no upper bound.
     * @param limit the most proposals to return, at most 100; 0 for 100. A page shorter
     *   than the limit is the last.
     * @param startAfter the id of the last proposal of the previous page; null for the first.
     */
    suspend fun listByEndDate(
        endDateFromMs: Long = 0,
        endDateToMs: Long = 0,
        limit: Int = 0,
        startAfter: String? = null,
    ): List<DecisionProposal> = sdk.queryGate.op {
        val json = mapNativeErrors {
            DecisionProposalsNative.listByEndDate(
                sdk.handle,
                endDateFromMs,
                endDateToMs,
                limit,
                startAfter,
            )
        }
        DecisionProposal.listFromJson(json.returned("listByEndDate"))
    }

    /** One page of [ownerId]'s proposals, in the order it filed them; paging as [listByEndDate]. */
    suspend fun listByOwner(
        ownerId: String,
        limit: Int = 0,
        startAfter: String? = null,
    ): List<DecisionProposal> = sdk.queryGate.op {
        val json = mapNativeErrors {
            DecisionProposalsNative.listByOwner(sdk.handle, ownerId, limit, startAfter)
        }
        DecisionProposal.listFromJson(json.returned("listByOwner"))
    }

    /** One page of every proposal, newest first; paging as [listByEndDate]. */
    suspend fun listByCreatedAt(
        limit: Int = 0,
        startAfter: String? = null,
    ): List<DecisionProposal> = sdk.queryGate.op {
        val json = mapNativeErrors {
            DecisionProposalsNative.listByCreatedAt(sdk.handle, limit, startAfter)
        }
        DecisionProposal.listFromJson(json.returned("listByCreatedAt"))
    }

    /**
     * The id of the yes/no poll [proposalId] opens, base58: the id
     * `Voting.votePollsByEndDate` lists it under.
     */
    suspend fun votePollId(proposalId: String): String = sdk.queryGate.op {
        mapNativeErrors { DecisionProposalsNative.getVotePollId(sdk.handle, proposalId) }
            .returned("getVotePollId")
    }

    /** The status and tallies of [proposalId]'s poll, or null for a proposal never created. */
    suspend fun votePollState(proposalId: String): YesNoVotePollState? = sdk.queryGate.op {
        mapNativeErrors { DecisionProposalsNative.getVotePollState(sdk.handle, proposalId) }
            ?.let(YesNoVotePollState::fromJson)
    }

    /**
     * A masternode's answer on [proposalId], or null when it has not voted on it, and once
     * the poll finished (which removes its votes).
     *
     * @param proTxHash the masternode's 32-byte pro_tx_hash in wire order, as [castVote]
     *   takes it.
     */
    suspend fun masternodeVote(proTxHash: ByteArray, proposalId: String): MasternodeYesNoVote? =
        sdk.queryGate.op {
            mapNativeErrors {
                DecisionProposalsNative.getMasternodeVote(sdk.handle, proTxHash, proposalId)
            }?.let(MasternodeYesNoVote::fromJson)
        }

    /**
     * Vote on [proposalId] for a masternode and wait for the response: the vote is signed
     * with the masternode voting key derived from [votingPrivateKey] and broadcast for
     * [proTxHash]. A masternode may vote 5 times on a proposal, but not give the answer it
     * already gave. A vote from a wallet that is not a masternode is refused by Platform
     * (no voter identity), the expected outcome.
     *
     * @param proTxHash 32-byte pro_tx_hash in wire order: the orientation `Txid` stores,
     *   which a parsed ProRegTx yields. NOT the byte order of the hex Core displays, which
     *   is its reverse (see [VoteCasting.castVote]).
     * @param votingPrivateKey 32-byte masternode voting private key; not stored.
     */
    suspend fun castVote(
        proposalId: String,
        choice: YesNoAbstainVoteChoice,
        proTxHash: ByteArray,
        votingPrivateKey: ByteArray,
    ) = sdk.queryGate.op {
        mapNativeErrors {
            DecisionProposalsNative.castVote(
                sdk.handle,
                proposalId,
                choice.discriminant,
                proTxHash,
                votingPrivateKey,
                sdk.network.ffiValue,
            )
        }
    }
}

/**
 * Creating and editing decision proposals through the platform wallet: the Kotlin
 * counterpart of Swift's `ManagedPlatformWallet` decision proposal slice
 * (`ManagedPlatformWalletDecisionProposals.swift`). Reach it through
 * `PlatformWalletManager.decisionProposals`; callers thread the wallet handle and the
 * manager's `signerHandle` into each call, as for the wallet's document calls.
 *
 * Each call is one native call (`platform_wallet_decision_proposal_create` / `_edit`): the
 * wallet builds the proposal (or reads the stored one and edits it), picks the signing key,
 * broadcasts and waits for Platform to confirm. Nothing is decided or checked here.
 */
class DecisionProposalTransactions internal constructor(private val gate: TeardownGate?) {

    /**
     * Create and broadcast a decision proposal of [ownerId] asking what [params] says,
     * signed via [signerHandle] with a key the wallet selects. The create states the
     * decision proposal fund (0.5 Dash at protocol version 15), paid by the owner on top of
     * the fees.
     *
     * @param walletHandle the handle of the wallet that holds [ownerId].
     * @param ownerId the proposer's 32-byte identity id.
     * @return the proposal Platform confirmed: its id and the document's canonical JSON.
     */
    suspend fun create(
        walletHandle: Long,
        ownerId: ByteArray,
        params: DecisionProposalParams,
        signerHandle: Long,
    ): CreatedDecisionProposal = gate.op {
        val proposalId = ByteArray(PROPOSAL_ID_SIZE)
        val documentJson = mapNativeErrors {
            DecisionProposalsNative.create(
                walletHandle,
                ownerId,
                params.title,
                params.description,
                params.url,
                params.passPercent,
                params.minimumTurnoutPercent,
                params.endDateMs,
                signerHandle,
                proposalId,
            )
        }
        CreatedDecisionProposal(id = proposalId.toBase58String(), documentJson = documentJson)
    }

    /**
     * Give the proposal [proposalId] of [ownerId] the text [edit] sets and broadcast the
     * replace, signed via [signerHandle] with a key the wallet selects as [create] does.
     *
     * The wallet reads the proposal once and puts the edit at the revision after the one
     * read: if another edit lands in between, Platform refuses this one (its
     * [DashSdkError.consensusError] code is 40106) rather than overwrite it, and the caller
     * reads the proposal again before offering the edit anew. rs-sdk refuses an edit that
     * changes nothing, and Platform one more than a day after the proposal was created.
     *
     * @param proposalId the proposal's 32-byte id.
     * @return the confirmed document's canonical JSON, at its new revision.
     * @throws DashSdkError.PlatformWallet.NotFound when there is no such proposal.
     */
    suspend fun edit(
        walletHandle: Long,
        ownerId: ByteArray,
        proposalId: ByteArray,
        edit: DecisionProposalEdit,
        signerHandle: Long,
    ): String = gate.op {
        val url = edit.url.toNative()
        mapNativeErrors {
            DecisionProposalsNative.edit(
                walletHandle,
                ownerId,
                proposalId,
                edit.title,
                edit.description,
                url.url,
                url.removeUrl,
                signerHandle,
            )
        }
    }

    private companion object {
        /** The length of the id the create writes back. */
        const val PROPOSAL_ID_SIZE = 32
    }
}

/**
 * The value a native call returned. Null is reserved for a thrown error (already raised by
 * the JNI layer) or an absence the call can report; for a call that always answers, null is
 * a broken native result.
 */
private fun <T : Any> T?.returned(call: String): T =
    this ?: throw DashSdkError.InternalError("DecisionProposalsNative.$call returned nothing")
