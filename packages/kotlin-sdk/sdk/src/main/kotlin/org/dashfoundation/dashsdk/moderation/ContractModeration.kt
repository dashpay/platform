package org.dashfoundation.dashsdk.moderation

import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.ContractModerationNative
import org.dashfoundation.dashsdk.wallet.op

/**
 * Contract moderation and fee pot reads (protocol version 14): a moderated contract's
 * banlist, suspension list and warning list, one identity's status on them, the records of
 * the documents its moderators removed, an elected contract's team actions and who approved
 * each, the team's moderation action counts, and a contract's fee pots. Kotlin counterpart
 * of the Swift SDK's `SDK+ContractModeration.swift`; reach it through
 * `Sdk.contractModeration`.
 *
 * Thin wrapper over [ContractModerationNative], one native call per method. rs-sdk decides
 * everything: which lists a contract keeps, the page caps, the cursor of the next page, how
 * a removal record's kept values read (one decoding budget per page, in rs-sdk's
 * `fetch_with_kept_values`). A page's cursor is passed back as it came. Every call runs
 * under the SDK's query gate, so closing the SDK waits for it. Ids are base58 strings.
 *
 * A refusal from the node (a list the contract does not keep, a type that keeps no removal
 * records, a contract that is not elected) is thrown as the `DashSdkError` the FFI reports.
 * Below protocol version 14 no request is sent: the SDK's protocol gate refuses it first,
 * and every method throws (a protocol error when the network is proven below 14 or the SDK
 * is pinned below it; a network error when the network's version could not be read, which
 * can pass on a retry).
 */
class ContractModeration internal constructor(private val sdk: Sdk) {

    /**
     * One identity's status on a moderated contract, on the lists read.
     *
     * @param lists the lists to read, each one the contract keeps. Null reads every list the
     *   contract keeps (rs-sdk fetches the contract to find them); an empty list is an
     *   explicit empty selection, sent for the node to judge.
     * @return the status, or null when the FFI returns no data.
     */
    suspend fun status(
        contractId: String,
        identityId: String,
        lists: List<ContractModerationList>? = null,
    ): ContractModerationStatus? = sdk.queryGate.op {
        val bytes = lists?.map { it.ffiTag.toByte() }?.toByteArray()
        mapNativeErrors {
            ContractModerationNative.getStatus(sdk.handle, contractId, identityId, bytes)
        }?.let(ContractModerationJson::status)
    }

    /**
     * One page of a moderated contract's banlist, suspension list or warning list, in
     * identity id order.
     *
     * @param startAfter the previous page's [ContractModerationEntriesPage.nextStartAfter];
     *   null for the first page.
     * @param limit the most entries to return, passed as it is (0 included, which the node
     *   judges); null for the page cap of the network queried.
     */
    suspend fun entries(
        contractId: String,
        list: ContractModerationList,
        startAfter: String? = null,
        limit: UInt? = null,
    ): ContractModerationEntriesPage? = sdk.queryGate.op {
        mapNativeErrors {
            ContractModerationNative.getEntries(
                sdk.handle,
                contractId,
                list.ffiTag,
                startAfter,
                limit.toNativeLimit(),
            )
        }?.let(ContractModerationJson::entriesPage)
    }

    /**
     * One page of the records of the documents a contract's moderators deleted within one
     * document type, in document id order. The node refuses a type that keeps no records.
     * Paging as for [entries].
     */
    suspend fun documentRemovals(
        contractId: String,
        documentTypeName: String,
        startAfter: String? = null,
        limit: UInt? = null,
    ): ContractDocumentRemovalsPage? = sdk.queryGate.op {
        mapNativeErrors {
            ContractModerationNative.getDocumentRemovals(
                sdk.handle,
                contractId,
                documentTypeName,
                startAfter,
                limit.toNativeLimit(),
            )
        }?.let(ContractModerationJson::removalsPage)
    }

    /**
     * The removal records of the documents named, within one document type. A document with
     * no record is left out of the answer; how many ids the node takes is the node's to
     * judge. A read by ids has no cursor.
     */
    suspend fun documentRemovals(
        contractId: String,
        documentTypeName: String,
        documentIds: List<String>,
    ): ContractDocumentRemovalsPage? = sdk.queryGate.op {
        mapNativeErrors {
            ContractModerationNative.getDocumentRemovalsByIds(
                sdk.handle,
                contractId,
                documentTypeName,
                documentIds.toTypedArray(),
            )
        }?.let(ContractModerationJson::removalsPage)
    }

    /**
     * One page of the actions an elected contract's seated moderation team votes on, in
     * action id order. The node refuses it unless at least one of the contract's document
     * types sets `moderatorAbilities.deleteSettled` (which only an elected contract may set);
     * a contract that keeps team actions but has none answers an empty page.
     *
     * @param startAtActionId the previous page's [ContractTeamActionsPage.nextStartAtActionId];
     *   null for the first page.
     * @param startAtActionIdIncluded the previous page's
     *   [ContractTeamActionsPage.nextStartAtActionIdIncluded]; the FFI refuses it without
     *   [startAtActionId].
     * @param limit as for [entries].
     */
    suspend fun teamActions(
        contractId: String,
        status: ContractTeamActionStatus,
        startAtActionId: String? = null,
        startAtActionIdIncluded: Boolean = false,
        limit: UInt? = null,
    ): ContractTeamActionsPage? = sdk.queryGate.op {
        mapNativeErrors {
            ContractModerationNative.getTeamActions(
                sdk.handle,
                contractId,
                status.ffiTag,
                startAtActionId,
                startAtActionIdIncluded,
                limit.toNativeLimit(),
            )
        }?.let(ContractModerationJson::teamActionsPage)
    }

    /**
     * Who approved one team action, in identity id order. The action is looked up among the
     * actions of [status] alone; the node refuses a contract none of whose types sets
     * `moderatorAbilities.deleteSettled`.
     */
    suspend fun teamActionSigners(
        contractId: String,
        status: ContractTeamActionStatus,
        actionId: String,
    ): ContractTeamActionSigners? = sdk.queryGate.op {
        mapNativeErrors {
            ContractModerationNative.getTeamActionSigners(
                sdk.handle,
                contractId,
                status.ffiTag,
                actionId,
            )
        }?.let(ContractModerationJson::teamActionSigners)
    }

    /**
     * How many counted moderation actions each member of an elected contract's seated team
     * signed since the moderators pot was last claimed or settled. A moderators pot
     * settlement (a membership change of the team) resets the counts even when it pays
     * nothing out.
     */
    suspend fun actionCounts(contractId: String): ContractModerationActionCounts? =
        sdk.queryGate.op {
            mapNativeErrors { ContractModerationNative.getActionCounts(sdk.handle, contractId) }
                ?.let(ContractModerationJson::actionCounts)
        }

    /**
     * What the document action fees of a contract have collected for its owner and its
     * moderation team, and the last explicit claim of each pot.
     */
    suspend fun feePots(contractId: String): ContractFeePots? = sdk.queryGate.op {
        mapNativeErrors { ContractModerationNative.getFeePots(sdk.handle, contractId) }
            ?.let(ContractModerationJson::feePots)
    }
}

/** The native `limit`: [ContractModerationNative.NO_LIMIT] for none, the value otherwise. */
private fun UInt?.toNativeLimit(): Long = this?.toLong() ?: ContractModerationNative.NO_LIMIT
