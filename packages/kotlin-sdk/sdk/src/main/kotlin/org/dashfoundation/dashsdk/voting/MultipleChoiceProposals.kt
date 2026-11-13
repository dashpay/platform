package org.dashfoundation.dashsdk.voting

import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.MultipleChoiceProposalsNative
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.wallet.TeardownGate
import org.dashfoundation.dashsdk.wallet.op

/**
 * Multiple choice proposals (protocol version 15): questions whose answers are the choices
 * entered for them, as `multipleChoiceProposal` and `choice` documents of the decision
 * proposals system contract. Reads of proposals and of the vote among their choices, the
 * two amounts a proposal and a choice cost, and the masternode vote. Kotlin counterpart of
 * the Swift SDK's `SDK+MultipleChoiceProposals.swift`; reach it through
 * `Sdk.multipleChoiceProposals`. Creating and editing a proposal and entering a choice go
 * through the wallet: [MultipleChoiceProposalTransactions].
 *
 * Thin wrapper over [MultipleChoiceProposalsNative]: rs-sdk decides everything (which
 * documents are proposals and choices, the vote's contest, the funds and fees), every read
 * is proved, and nothing is checked here. Every call runs under the SDK's query gate, so
 * closing the SDK waits for it. Ids are base58 strings. A network below protocol version 15
 * has no decision proposals contract, which the calls report as an error.
 */
class MultipleChoiceProposals internal constructor(private val sdk: Sdk) {

    /** The proposal [proposalId], or null when there is none. */
    suspend fun fetch(proposalId: String): MultipleChoiceProposal? = sdk.queryGate.op {
        mapNativeErrors { MultipleChoiceProposalsNative.fetch(sdk.handle, proposalId) }
            ?.let(MultipleChoiceProposal::fromJson)
    }

    /**
     * One page of [ownerId]'s proposals, in the order it filed them.
     *
     * @param limit the most proposals to return, at most 100; 0 for 100. A page shorter
     *   than the limit is the last.
     * @param startAfter the id of the last proposal of the previous page; null for the first.
     */
    suspend fun listByOwner(
        ownerId: String,
        limit: Int = 0,
        startAfter: String? = null,
    ): List<MultipleChoiceProposal> = sdk.queryGate.op {
        val json = mapNativeErrors {
            MultipleChoiceProposalsNative.listByOwner(sdk.handle, ownerId, limit, startAfter)
        }
        MultipleChoiceProposal.listFromJson(json.returned("listByOwner"))
    }

    /** One page of every proposal, newest first; paging as [listByOwner]. */
    suspend fun listByCreatedAt(
        limit: Int = 0,
        startAfter: String? = null,
    ): List<MultipleChoiceProposal> = sdk.queryGate.op {
        val json = mapNativeErrors {
            MultipleChoiceProposalsNative.listByCreatedAt(sdk.handle, limit, startAfter)
        }
        MultipleChoiceProposal.listFromJson(json.returned("listByCreatedAt"))
    }

    /**
     * The vote among [proposalId]'s choices: every choice with its tally while the vote
     * runs, and how it ended with every choice's final tally once it did. A proposal no
     * choice was entered for has no choices and no result.
     */
    suspend fun vote(proposalId: String): MultipleChoiceProposalVote = sdk.queryGate.op {
        val json = mapNativeErrors { MultipleChoiceProposalsNative.getVote(sdk.handle, proposalId) }
        MultipleChoiceProposalVote.fromJson(json.returned("getVote"))
    }

    /**
     * The choice [proposalId]'s finished vote kept, an ordinary document of the identity
     * that entered it; null while the vote runs, when it kept none of the choices, and for
     * a proposal no choice was entered for.
     */
    suspend fun keptChoice(proposalId: String): ProposalChoice? = sdk.queryGate.op {
        mapNativeErrors { MultipleChoiceProposalsNative.getKeptChoice(sdk.handle, proposalId) }
            ?.let(ProposalChoice::fromJson)
    }

    /**
     * The answer a masternode gave in [proposalId]'s vote, or null when it has not voted in
     * it, and once the vote ended.
     *
     * @param proTxHash the masternode's 32-byte pro_tx_hash in wire order, as [castVote]
     *   takes it.
     */
    suspend fun masternodeVote(
        proTxHash: ByteArray,
        proposalId: String,
    ): MultipleChoiceVoteChoice? = sdk.queryGate.op {
        mapNativeErrors {
            MultipleChoiceProposalsNative.getMasternodeVote(sdk.handle, proTxHash, proposalId)
        }?.let(MultipleChoiceVoteChoice::fromJson)
    }

    /**
     * What creating one multiple choice proposal adds to the processing fee of its batch,
     * in credits (0.2 Dash at protocol version 15). Platform charges the owner it on top of
     * the fees; the create does not state it.
     */
    suspend fun processingFee(): ULong = sdk.queryGate.op {
        creditsFromDecimal(
            mapNativeErrors { MultipleChoiceProposalsNative.getProcessingFee(sdk.handle) }
                .returned("getProcessingFee"),
        )
    }

    /**
     * The fund a choice entered now for [proposalId] pays, in credits: the contest's fund,
     * raised as the vote fills up. Entering a choice reads and states it itself; this is
     * for showing it beforehand.
     */
    suspend fun choiceFundToJoin(proposalId: String): ULong = sdk.queryGate.op {
        creditsFromDecimal(
            mapNativeErrors {
                MultipleChoiceProposalsNative.getChoiceFundToJoin(sdk.handle, proposalId)
            }.returned("getChoiceFundToJoin"),
        )
    }

    /**
     * Vote among [proposalId]'s choices for a masternode and wait for the response: the
     * vote is signed with the masternode voting key derived from [votingPrivateKey] and
     * broadcast for [proTxHash]. A vote from a wallet that is not a masternode is refused
     * by Platform (no voter identity), the expected outcome.
     *
     * @param proTxHash 32-byte pro_tx_hash in wire order: the orientation `Txid` stores,
     *   which a parsed ProRegTx yields. NOT the byte order of the hex Core displays, which
     *   is its reverse (see [VoteCasting.castVote]).
     * @param votingPrivateKey 32-byte masternode voting private key; not stored.
     */
    suspend fun castVote(
        proposalId: String,
        choice: MultipleChoiceVoteChoice,
        proTxHash: ByteArray,
        votingPrivateKey: ByteArray,
    ) = sdk.queryGate.op {
        val vote = choice.toNative()
        mapNativeErrors {
            MultipleChoiceProposalsNative.castVote(
                sdk.handle,
                proposalId,
                vote.discriminant,
                vote.choiceId,
                proTxHash,
                votingPrivateKey,
                sdk.network.ffiValue,
            )
        }
    }
}

/**
 * Creating and editing multiple choice proposals and entering their choices through the
 * platform wallet: the Kotlin counterpart of Swift's `ManagedPlatformWallet` multiple choice
 * proposal slice (`ManagedPlatformWalletMultipleChoiceProposals.swift`). Reach it through
 * `PlatformWalletManager.multipleChoiceProposals`; callers thread the wallet handle and the
 * manager's `signerHandle` into each call, as for the wallet's document calls.
 *
 * Each call is one native call (`platform_wallet_multiple_choice_proposal_create` / `_edit`
 * / `_enter_choice`): the wallet builds the document (or reads the stored proposal and edits
 * it), picks the signing key, broadcasts and waits for Platform to confirm. Nothing is
 * decided or checked here.
 */
class MultipleChoiceProposalTransactions internal constructor(private val gate: TeardownGate?) {

    /**
     * Create and broadcast a multiple choice proposal of [ownerId] asking what [params]
     * says, signed via [signerHandle] with a key the wallet selects. Platform charges the
     * owner the processing fee [MultipleChoiceProposals.processingFee] reads on top of the
     * fees.
     *
     * @param walletHandle the handle of the wallet that holds [ownerId].
     * @param ownerId the proposer's 32-byte identity id.
     * @return the proposal Platform confirmed: its id and the document's canonical JSON.
     */
    suspend fun create(
        walletHandle: Long,
        ownerId: ByteArray,
        params: MultipleChoiceProposalParams,
        signerHandle: Long,
    ): CreatedMultipleChoiceProposal = gate.op {
        val proposalId = ByteArray(ID_SIZE)
        val documentJson = mapNativeErrors {
            MultipleChoiceProposalsNative.create(
                walletHandle,
                ownerId,
                params.title,
                params.descriptionDocument?.id,
                params.descriptionDocument?.contractId,
                params.descriptionDocument?.documentType,
                params.url,
                params.open,
                signerHandle,
                proposalId,
            )
        }
        CreatedMultipleChoiceProposal(id = proposalId.toBase58String(), documentJson = documentJson)
    }

    /**
     * Give the proposal [proposalId] of [ownerId] what [edit] changes and broadcast the
     * replace, signed via [signerHandle] with a key the wallet selects as [create] does.
     *
     * The wallet reads the proposal once and puts the edit at the revision after the one
     * read: if another edit lands in between, Platform refuses this one (its
     * [DashSdkError.consensusError] code is 40106) rather than overwrite it, and the caller
     * reads the proposal again before offering the edit anew. rs-sdk refuses an edit that
     * changes nothing, and Platform one more than two hours after the proposal was created
     * and one that leaves it with neither a description document nor a url.
     *
     * @param proposalId the proposal's 32-byte id.
     * @return the confirmed document's canonical JSON, at its new revision.
     * @throws DashSdkError.PlatformWallet.NotFound when there is no such proposal.
     */
    suspend fun edit(
        walletHandle: Long,
        ownerId: ByteArray,
        proposalId: ByteArray,
        edit: MultipleChoiceProposalEdit,
        signerHandle: Long,
    ): String = gate.op {
        val descriptionDocument = edit.descriptionDocument.toNative()
        val url = edit.url.toNative()
        mapNativeErrors {
            MultipleChoiceProposalsNative.edit(
                walletHandle,
                ownerId,
                proposalId,
                edit.title,
                descriptionDocument.value?.id,
                descriptionDocument.value?.contractId,
                descriptionDocument.value?.documentType,
                descriptionDocument.remove,
                url.value,
                url.remove,
                signerHandle,
            )
        }
    }

    /**
     * Enter the choice [params] describes, owned by [ownerId], for the proposal
     * [proposalId] and broadcast it, signed via [signerHandle] with a key the wallet
     * selects as [create] does. The put states the contest fund
     * [MultipleChoiceProposals.choiceFundToJoin] reads, paid by the owner on top of the
     * fees; the first choice opens the vote. Platform refuses a choice for a proposal that
     * is not open from anyone but its owner, and one entered once the vote's join window
     * closed. A choice is never edited.
     *
     * @param ownerId the 32-byte id of the identity entering the choice.
     * @param proposalId the proposal's 32-byte id.
     * @return the choice Platform confirmed: its id and the document's canonical JSON.
     */
    suspend fun enterChoice(
        walletHandle: Long,
        ownerId: ByteArray,
        proposalId: ByteArray,
        params: ProposalChoiceParams,
        signerHandle: Long,
    ): EnteredProposalChoice = gate.op {
        val choiceId = ByteArray(ID_SIZE)
        val documentJson = mapNativeErrors {
            MultipleChoiceProposalsNative.enterChoice(
                walletHandle,
                ownerId,
                proposalId,
                params.title,
                params.description,
                signerHandle,
                choiceId,
            )
        }
        EnteredProposalChoice(id = choiceId.toBase58String(), documentJson = documentJson)
    }

    private companion object {
        /** The length of the id a create writes back. */
        const val ID_SIZE = 32
    }
}

/**
 * The value a native call returned. Null is reserved for a thrown error (already raised by
 * the JNI layer) or an absence the call can report; for a call that always answers, null is
 * a broken native result.
 */
private fun <T : Any> T?.returned(call: String): T = this
    ?: throw DashSdkError.InternalError("MultipleChoiceProposalsNative.$call returned nothing")
