package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for the contract moderation and fee pot reads (protocol version 14):
 * mirrors `rs-unified-sdk-jni/src/contract_moderation.rs`, one export per C function of
 * `rs-sdk-ffi` (`dash_sdk_contract_moderation_get_*` and `dash_sdk_contract_get_fee_pots`).
 *
 * Internal: the public API is [org.dashfoundation.dashsdk.moderation.ContractModeration],
 * one native call per method; no export stitches two C calls together. `sdk` is
 * `Sdk.handle`. Ids are base58 strings. Every read returns the JSON the C function wrote,
 * or null when it returned no data; errors throw [DashSDKException].
 *
 * The byte arguments mirror `ContractModerationListFFI` and `ContractTeamActionStatusFFI` in
 * `packages/rs-sdk-ffi/src/contract_moderation/mod.rs`: list 0 banlist, 1 suspensions,
 * 2 warnings; status 0 active, 1 closed. A `limit` of [NO_LIMIT] passes no bound (the page
 * cap of the network queried); any other value must fit a `u32` and is passed as it is.
 */
internal object ContractModerationNative {

    /** The `limit` that passes no page bound. */
    const val NO_LIMIT: Long = -1

    /**
     * One identity's status on the lists read (`dash_sdk_contract_moderation_get_status`).
     *
     * @param lists the list bytes; null for no selection (every list the contract keeps),
     *   empty for an explicit empty selection, which the node judges.
     */
    external fun getStatus(
        sdk: Long,
        contractId: String,
        identityId: String,
        lists: ByteArray?,
    ): String?

    /** One page of a moderation list (`dash_sdk_contract_moderation_get_entries`). */
    external fun getEntries(
        sdk: Long,
        contractId: String,
        list: Int,
        startAfter: String?,
        limit: Long,
    ): String?

    /** One page of a type's removal records (`dash_sdk_contract_moderation_get_document_removals`). */
    external fun getDocumentRemovals(
        sdk: Long,
        contractId: String,
        documentTypeName: String,
        startAfter: String?,
        limit: Long,
    ): String?

    /**
     * The removal records of the documents named
     * (`dash_sdk_contract_moderation_get_document_removals_by_ids`).
     */
    external fun getDocumentRemovalsByIds(
        sdk: Long,
        contractId: String,
        documentTypeName: String,
        documentIds: Array<String>,
    ): String?

    /** One page of an elected contract's team actions (`dash_sdk_contract_moderation_get_team_actions`). */
    external fun getTeamActions(
        sdk: Long,
        contractId: String,
        status: Int,
        startAtActionId: String?,
        startAtActionIdIncluded: Boolean,
        limit: Long,
    ): String?

    /** Who approved one team action (`dash_sdk_contract_moderation_get_team_action_signers`). */
    external fun getTeamActionSigners(
        sdk: Long,
        contractId: String,
        status: Int,
        actionId: String,
    ): String?

    /** The seated team's moderation action counts (`dash_sdk_contract_moderation_get_action_counts`). */
    external fun getActionCounts(sdk: Long, contractId: String): String?

    /** A contract's fee pots (`dash_sdk_contract_get_fee_pots`). */
    external fun getFeePots(sdk: Long, contractId: String): String?
}
