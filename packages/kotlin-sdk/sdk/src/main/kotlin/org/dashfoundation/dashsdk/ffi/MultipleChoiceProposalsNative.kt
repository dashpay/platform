package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for multiple choice proposals (protocol version 15): mirrors
 * `rs-unified-sdk-jni/src/multiple_choice_proposals.rs`, one export per C function: each
 * `dash_sdk_multiple_choice_proposal_*` function of `rs-sdk-ffi` (the reads, the two amounts
 * and the masternode vote), and `platform_wallet_multiple_choice_proposal_create` / `_edit` /
 * `_enter_choice` of `platform-wallet-ffi` (the writes).
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.voting.MultipleChoiceProposals]
 * (reads and the masternode vote) and
 * [org.dashfoundation.dashsdk.voting.MultipleChoiceProposalTransactions] (create, edit and
 * enter a choice), one native call each; no export stitches two C calls together. `sdk` is
 * `Sdk.handle`, `walletHandle` a `ManagedPlatformWallet` handle and `signerHandle` the
 * manager's `PlatformWalletManager.signerHandle`. Ids are base58 strings except where a
 * `ByteArray` is named. Errors throw [DashSDKException].
 */
internal object MultipleChoiceProposalsNative {

    /**
     * Create + broadcast a multiple choice proposal of [ownerId] asking [title], signed via
     * [signerHandle]. Bridges `platform_wallet_multiple_choice_proposal_create`: the wallet
     * builds the proposal, picks the signing key, broadcasts and waits for Platform to
     * confirm; Platform charges the owner the processing fee [getProcessingFee] reads on top
     * of the fees.
     *
     * @param ownerId the proposer's 32-byte identity id.
     * @param descriptionDocumentId the 32-byte id of the document saying what is being
     *   decided; null, with the next two, for none.
     * @param descriptionDocumentContractId the 32-byte id of the contract that document
     *   lives in.
     * @param descriptionDocumentType that document's type.
     * @param url null for none.
     * @param open whether anyone may enter choices; only the owner may when it is false.
     * @param outProposalId a `ByteArray(32)`, checked before anything is broadcast, that
     *   receives the proposal's id.
     * @return the confirmed document's canonical JSON.
     */
    external fun create(
        walletHandle: Long,
        ownerId: ByteArray,
        title: String,
        descriptionDocumentId: ByteArray?,
        descriptionDocumentContractId: ByteArray?,
        descriptionDocumentType: String?,
        url: String?,
        open: Boolean,
        signerHandle: Long,
        outProposalId: ByteArray,
    ): String

    /**
     * Give the proposal [proposalId] of [ownerId] what the arguments change and broadcast
     * the replace, signed via [signerHandle]. Bridges
     * `platform_wallet_multiple_choice_proposal_edit`: the wallet reads the proposal once,
     * edits it at the revision after the one read and picks the signing key as [create]
     * does.
     *
     * @param ownerId the proposer's 32-byte identity id.
     * @param proposalId the proposal's 32-byte id.
     * @param title null keeps it.
     * @param descriptionDocumentId with the next two, the new description document; all
     *   null keeps it, unless [removeDescriptionDocument].
     * @param removeDescriptionDocument removes the description document; the wallet refuses
     *   it together with a new one.
     * @param url null keeps it, unless [removeUrl].
     * @param removeUrl removes the url; the wallet refuses it together with a non-null [url].
     * @return the confirmed document's canonical JSON, at its new revision.
     */
    external fun edit(
        walletHandle: Long,
        ownerId: ByteArray,
        proposalId: ByteArray,
        title: String?,
        descriptionDocumentId: ByteArray?,
        descriptionDocumentContractId: ByteArray?,
        descriptionDocumentType: String?,
        removeDescriptionDocument: Boolean,
        url: String?,
        removeUrl: Boolean,
        signerHandle: Long,
    ): String

    /**
     * Enter a choice of [ownerId] for the proposal [proposalId] and broadcast it, signed via
     * [signerHandle]. Bridges `platform_wallet_multiple_choice_proposal_enter_choice`: the
     * wallet builds the choice, whose put states the contest fund [getChoiceFundToJoin]
     * reads (paid by the owner on top of the fees), picks the signing key, broadcasts and
     * waits for Platform to confirm.
     *
     * @param ownerId the 32-byte id of the identity entering the choice.
     * @param proposalId the proposal's 32-byte id.
     * @param description null for none.
     * @param outChoiceId a `ByteArray(32)`, checked before anything is broadcast, that
     *   receives the choice's id.
     * @return the confirmed document's canonical JSON.
     */
    external fun enterChoice(
        walletHandle: Long,
        ownerId: ByteArray,
        proposalId: ByteArray,
        title: String,
        description: String?,
        signerHandle: Long,
        outChoiceId: ByteArray,
    ): String

    /**
     * A proposal as JSON, or null when there is none. Bridges
     * `dash_sdk_multiple_choice_proposal_fetch`.
     */
    external fun fetch(sdk: Long, proposalId: String): String?

    /**
     * One page of [ownerId]'s proposals in filing order, as a JSON array. [limit] 0 is 100.
     * Bridges `dash_sdk_multiple_choice_proposal_list_by_owner`.
     */
    external fun listByOwner(sdk: Long, ownerId: String, limit: Int, startAfter: String?): String?

    /**
     * One page of every proposal, newest first, as a JSON array. Bridges
     * `dash_sdk_multiple_choice_proposal_list_by_created_at`.
     */
    external fun listByCreatedAt(sdk: Long, limit: Int, startAfter: String?): String?

    /**
     * The vote among [proposalId]'s choices as JSON: every choice with its tally, the other
     * tallies and how the vote ended once it did. Bridges
     * `dash_sdk_multiple_choice_proposal_get_vote`.
     */
    external fun getVote(sdk: Long, proposalId: String): String?

    /**
     * The choice [proposalId]'s finished vote kept as JSON, or null while the vote runs and
     * when it kept none. Bridges `dash_sdk_multiple_choice_proposal_get_kept_choice`.
     */
    external fun getKeptChoice(sdk: Long, proposalId: String): String?

    /**
     * A masternode's answer in [proposalId]'s vote as `{"choice", "choiceId"}`, or null when
     * it has not voted or the vote ended. Bridges
     * `dash_sdk_multiple_choice_proposal_get_masternode_vote`.
     *
     * @param voterProTxHash 32 bytes in wire order, as [castVote] takes it.
     */
    external fun getMasternodeVote(sdk: Long, voterProTxHash: ByteArray, proposalId: String): String?

    /**
     * What creating one proposal adds to the processing fee of its batch, in credits as a
     * decimal string. Bridges `dash_sdk_multiple_choice_proposal_get_processing_fee`.
     */
    external fun getProcessingFee(sdk: Long): String?

    /**
     * The fund a choice entered now for [proposalId] pays, in credits as a decimal string.
     * Bridges `dash_sdk_multiple_choice_proposal_get_choice_fund_to_join`.
     */
    external fun getChoiceFundToJoin(sdk: Long, proposalId: String): String?

    /**
     * Vote among [proposalId]'s choices for a masternode and wait for the response. Bridges
     * `dash_sdk_multiple_choice_proposal_cast_vote`.
     *
     * @param voteChoice `0` for the choice [choiceId] names, `1` none of the choices, `2`
     *   abstain.
     * @param choiceId the document id of the choice voted for; null for the answers that
     *   name none.
     * @param voterProTxHash 32 bytes in wire order (the reverse of the hex Core displays).
     * @param votingPrivateKey the 32-byte masternode voting private key; not stored.
     * @param networkOrd `Network.ffiValue`.
     */
    external fun castVote(
        sdk: Long,
        proposalId: String,
        voteChoice: Int,
        choiceId: String?,
        voterProTxHash: ByteArray,
        votingPrivateKey: ByteArray,
        networkOrd: Int,
    )
}
