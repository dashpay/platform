import Foundation
import DashSDKFFI

// MARK: - Decision proposals (protocol version 15)
//
// Thin bridges over the `dash_sdk_decision_proposal_*` functions of
// `rs-sdk-ffi`, which call `dash_sdk::platform::decision_proposals` and
// `dash_sdk::platform::yes_no_vote_polls`. rs-sdk decides everything: the
// proposal's properties, the fund the create states, which documents are
// proposals, refusing an edit that changes nothing, the next revision, the
// poll's id. These wrappers marshal arguments in and decode the JSON out.
//
// Every read and the vote park their thread inside Rust until DAPI answers,
// so they run on the shared Platform query queue
// (``SDK/performBlockingQuery(_:)``) and the caller only awaits.
//
// Creating and editing a proposal goes through the platform wallet, the same
// path the app creates and replaces every other document by:
// ``buildDecisionProposal(ownerIdentityId:params:)`` (or
// ``fetchDecisionProposalDocument(proposalId:)`` then
// ``DecisionProposalDocument/edited(_:)``) here, then
// `ManagedPlatformWallet.createDecisionProposal` /
// `ManagedPlatformWallet.replaceDecisionProposal`.

/// A decision proposal document held by `rs-sdk-ffi`: one built for a create
/// (``SDK/buildDecisionProposal(ownerIdentityId:params:)``), a stored one
/// fetched for an edit (``SDK/fetchDecisionProposalDocument(proposalId:)``),
/// or an edited copy (``edited(_:)``).
///
/// Owns its `DocumentHandle` and frees it on deinit. Both methods only read
/// the document, and it never changes after it is made, so the object may be
/// shared across threads.
public final class DecisionProposalDocument: @unchecked Sendable {
    /// The `DocumentHandle *` this object owns.
    private let handle: OpaquePointer

    /// Takes ownership of `handle`.
    fileprivate init(handle: OpaquePointer) {
        self.handle = handle
    }

    deinit {
        dash_sdk_document_free(handle)
    }

    /// The document's own properties as the JSON object the platform
    /// wallet's document create and replace take
    /// (`dash_sdk_decision_proposal_properties_json`). A proposal without a
    /// url has no `url` key.
    public func propertiesJSON() throws -> String {
        let result = dash_sdk_decision_proposal_properties_json(handle)
        guard let json = try DecisionProposalFFI.consumeString(result) else {
            throw SDKError.internalError("dash_sdk_decision_proposal_properties_json returned no data")
        }
        return json
    }

    /// A copy of this proposal with the text `edit` sets, at its next revision
    /// (`dash_sdk_decision_proposal_build_edit`). Pure: no network, and this
    /// document is left as it is.
    ///
    /// rs-sdk refuses, as ``SDKError/invalidParameter(_:)``, an edit that
    /// changes nothing and a document that is not a decision proposal.
    public func edited(_ edit: DecisionProposalEdit) throws -> DecisionProposalDocument {
        let url: String?
        let removeURL: Bool
        switch edit.url {
        case .keep:
            url = nil
            removeURL = false
        case .set(let value):
            url = value
            removeURL = false
        case .remove:
            url = nil
            removeURL = true
        }

        let result = withDecisionProposalCString(edit.title) { titlePtr in
            withDecisionProposalCString(edit.description) { descriptionPtr in
                withDecisionProposalCString(url) { urlPtr in
                    let ffiEdit = DashSDKDecisionProposalEdit(
                        title: titlePtr,
                        description: descriptionPtr,
                        url: urlPtr,
                        remove_url: removeURL
                    )
                    return withUnsafePointer(to: ffiEdit) { editPtr in
                        // The contract id and type name are the FFI's own
                        // static strings, passed straight back.
                        dash_sdk_decision_proposal_build_edit(
                            handle,
                            dash_sdk_decision_proposal_contract_id(),
                            dash_sdk_decision_proposal_document_type_name(),
                            editPtr
                        )
                    }
                }
            }
        }
        guard let edited = try DecisionProposalFFI.consumeDocument(result) else {
            throw SDKError.internalError("dash_sdk_decision_proposal_build_edit returned no document")
        }
        return edited
    }
}

extension SDK {

    // MARK: - Reads

    /// The decision proposal `id` names, or `nil` when there is none
    /// (`dash_sdk_decision_proposal_fetch`).
    ///
    /// - Parameter id: The proposal's id, base58.
    public nonisolated func decisionProposal(id: String) async throws -> DecisionProposal? {
        try await performBlockingQuery { handle in
            let result = id.withCString { idPtr in
                dash_sdk_decision_proposal_fetch(handle, idPtr)
            }
            return try DecisionProposalFFI.consumeString(result).map {
                try DecisionProposalJSON.decode(DecisionProposal.self, from: $0)
            }
        }
    }

    /// One page of the decision proposals whose end date is within the
    /// bounds, closing soonest first (`dash_sdk_decision_proposal_list_by_end_date`).
    ///
    /// - Parameters:
    ///   - from: The earliest end date listed, included; `nil` for no lower
    ///     bound. The current time lists the open proposals.
    ///   - to: The end date the listing stops before; `nil` for no upper bound.
    ///   - limit: The most proposals to return; `nil` for Platform's cap.
    ///   - startAfter: The id (base58) of the last proposal of the previous
    ///     page; `nil` for the first page.
    /// - Returns: The page; one shorter than `limit` is the last.
    public nonisolated func decisionProposalsByEndDate(
        from: Date? = nil,
        to: Date? = nil,
        limit: UInt32? = nil,
        startAfter: String? = nil
    ) async throws -> [DecisionProposal] {
        // 0 is the FFI's "no bound".
        let fromMs = from?.millisecondsSince1970 ?? 0
        let toMs = to?.millisecondsSince1970 ?? 0
        return try await performBlockingQuery { handle in
            let result = withDecisionProposalCString(startAfter) { startAfterPtr in
                dash_sdk_decision_proposal_list_by_end_date(
                    handle, fromMs, toMs, limit ?? 0, startAfterPtr)
            }
            return try DecisionProposalFFI.consumeListing(
                result, function: "dash_sdk_decision_proposal_list_by_end_date")
        }
    }

    /// One page of an identity's decision proposals, in the order it filed
    /// them (`dash_sdk_decision_proposal_list_by_owner`).
    ///
    /// - Parameters:
    ///   - ownerId: The proposer's identity id, base58.
    ///   - limit: The most proposals to return; `nil` for Platform's cap.
    ///   - startAfter: The id (base58) of the last proposal of the previous
    ///     page; `nil` for the first page.
    public nonisolated func decisionProposalsByOwner(
        ownerId: String,
        limit: UInt32? = nil,
        startAfter: String? = nil
    ) async throws -> [DecisionProposal] {
        try await performBlockingQuery { handle in
            let result = ownerId.withCString { ownerPtr in
                withDecisionProposalCString(startAfter) { startAfterPtr in
                    dash_sdk_decision_proposal_list_by_owner(
                        handle, ownerPtr, limit ?? 0, startAfterPtr)
                }
            }
            return try DecisionProposalFFI.consumeListing(
                result, function: "dash_sdk_decision_proposal_list_by_owner")
        }
    }

    /// One page of every decision proposal, newest first
    /// (`dash_sdk_decision_proposal_list_by_created_at`).
    ///
    /// - Parameters:
    ///   - limit: The most proposals to return; `nil` for Platform's cap.
    ///   - startAfter: The id (base58) of the last proposal of the previous
    ///     page; `nil` for the first page.
    public nonisolated func decisionProposalsByCreatedAt(
        limit: UInt32? = nil,
        startAfter: String? = nil
    ) async throws -> [DecisionProposal] {
        try await performBlockingQuery { handle in
            let result = withDecisionProposalCString(startAfter) { startAfterPtr in
                dash_sdk_decision_proposal_list_by_created_at(handle, limit ?? 0, startAfterPtr)
            }
            return try DecisionProposalFFI.consumeListing(
                result, function: "dash_sdk_decision_proposal_list_by_created_at")
        }
    }

    /// The id (base58) of the yes/no poll a decision proposal opens, the id
    /// the end-date listing of vote polls names it by
    /// (`dash_sdk_decision_proposal_get_vote_poll_id`).
    ///
    /// - Parameter proposalId: The proposal's id, base58.
    public nonisolated func decisionProposalVotePollId(proposalId: String) async throws -> String {
        try await performBlockingQuery { handle in
            let result = proposalId.withCString { idPtr in
                dash_sdk_decision_proposal_get_vote_poll_id(handle, idPtr)
            }
            guard let pollId = try DecisionProposalFFI.consumeString(result) else {
                throw SDKError.internalError("dash_sdk_decision_proposal_get_vote_poll_id returned no data")
            }
            return pollId
        }
    }

    /// The status and tallies of a decision proposal's yes/no poll, or `nil`
    /// when the poll was never opened
    /// (`dash_sdk_decision_proposal_get_vote_poll_state`).
    ///
    /// - Parameter proposalId: The proposal's id, base58.
    public nonisolated func decisionProposalVotePollState(
        proposalId: String
    ) async throws -> YesNoVotePollState? {
        try await performBlockingQuery { handle in
            let result = proposalId.withCString { idPtr in
                dash_sdk_decision_proposal_get_vote_poll_state(handle, idPtr)
            }
            return try DecisionProposalFFI.consumeString(result).map {
                try DecisionProposalJSON.decode(YesNoVotePollState.self, from: $0)
            }
        }
    }

    /// A masternode's answer on a decision proposal, or `nil` when it has not
    /// voted on it or the poll has finished (which removes its votes)
    /// (`dash_sdk_decision_proposal_get_masternode_vote`).
    ///
    /// - Parameters:
    ///   - proposalId: The proposal's id, base58.
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash in **wire order**,
    ///     the same bytes
    ///     ``castDecisionProposalVote(proposalId:choice:proTxHash:votingPrivateKey:)``
    ///     takes (see
    ///     ``SDK/castContestedResourceVote(dataContractId:documentTypeName:indexName:indexValues:choice:proTxHash:votingPrivateKey:)``
    ///     for why the orientation matters).
    public nonisolated func decisionProposalMasternodeVote(
        proposalId: String,
        proTxHash: Data
    ) async throws -> MasternodeYesNoVote? {
        // The FFI reads exactly 32 bytes through the pointer.
        guard proTxHash.count == 32 else {
            throw SDKError.invalidParameter("proTxHash must be exactly 32 bytes")
        }
        return try await performBlockingQuery { handle in
            let result = proTxHash.withUnsafeBytes { proTxPtr in
                proposalId.withCString { idPtr in
                    dash_sdk_decision_proposal_get_masternode_vote(
                        handle,
                        proTxPtr.bindMemory(to: UInt8.self).baseAddress,
                        idPtr
                    )
                }
            }
            return try DecisionProposalFFI.consumeString(result).map {
                try DecisionProposalJSON.decode(MasternodeYesNoVote.self, from: $0)
            }
        }
    }

    // MARK: - Vote

    /// Answer a decision proposal for a masternode and wait for Platform's
    /// response (`dash_sdk_decision_proposal_cast_vote`).
    ///
    /// The vote is built, signed with the masternode voting key derived from
    /// `votingPrivateKey` and broadcast on the Rust side, exactly as
    /// ``castContestedResourceVote(dataContractId:documentTypeName:indexName:indexValues:choice:proTxHash:votingPrivateKey:)``
    /// does for a contest. A wallet that is not a masternode has no voter
    /// identity, so Platform refuses its vote.
    ///
    /// - Parameters:
    ///   - proposalId: The proposal's id, base58.
    ///   - choice: Yes, no or abstain.
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash in **wire order**,
    ///     not the byte order of the hex Core displays (see
    ///     ``castContestedResourceVote(dataContractId:documentTypeName:indexName:indexValues:choice:proTxHash:votingPrivateKey:)``).
    ///   - votingPrivateKey: The masternode's 32-byte voting private key. It
    ///     is not retained.
    public nonisolated func castDecisionProposalVote(
        proposalId: String,
        choice: YesNoAbstainVoteChoice,
        proTxHash: Data,
        votingPrivateKey: Data
    ) async throws {
        // The FFI reads exactly 32 bytes through each pointer.
        guard proTxHash.count == 32 else {
            throw SDKError.invalidParameter("proTxHash must be exactly 32 bytes")
        }
        guard votingPrivateKey.count == 32 else {
            throw SDKError.invalidParameter("votingPrivateKey must be exactly 32 bytes")
        }
        let voteChoice = choice.ffiTag
        let networkValue = network.ffiValue

        try await performBlockingQuery { handle in
            let result = proTxHash.withUnsafeBytes { proTxPtr in
                votingPrivateKey.withUnsafeBytes { keyPtr in
                    proposalId.withCString { idPtr in
                        dash_sdk_decision_proposal_cast_vote(
                            handle,
                            idPtr,
                            voteChoice,
                            proTxPtr.bindMemory(to: UInt8.self).baseAddress,
                            keyPtr.bindMemory(to: UInt8.self).baseAddress,
                            networkValue
                        )
                    }
                }
            }
            // No data on success.
            _ = try DecisionProposalFFI.consumeString(result)
        }
    }

    // MARK: - Create and edit (documents for the platform wallet)

    /// Build a decision proposal of `ownerIdentityId` asking what `params`
    /// says (`dash_sdk_decision_proposal_build`). Nothing is broadcast.
    ///
    /// Create it with `ManagedPlatformWallet.createDecisionProposal`, which
    /// passes its ``DecisionProposalDocument/propertiesJSON()`` to the wallet's
    /// document create. The create states the decision proposal fund on top
    /// of the fees. Platform judges the text, the rule and the end date then;
    /// nothing is checked before. A network below protocol version 15 has no
    /// decision proposals contract, which is an error.
    ///
    /// - Parameter ownerIdentityId: The proposer's identity id, base58.
    public nonisolated func buildDecisionProposal(
        ownerIdentityId: String,
        params: DecisionProposalParams
    ) async throws -> DecisionProposalDocument {
        try await performBlockingQuery { handle in
            let result = ownerIdentityId.withCString { ownerPtr in
                params.title.withCString { titlePtr in
                    params.description.withCString { descriptionPtr in
                        withDecisionProposalCString(params.url) { urlPtr in
                            let ffiParams = DashSDKDecisionProposalParams(
                                title: titlePtr,
                                description: descriptionPtr,
                                url: urlPtr,
                                pass_percent: params.passPercent,
                                minimum_turnout_percent: params.minimumTurnoutPercent,
                                end_date_ms: params.endDateMs
                            )
                            return withUnsafePointer(to: ffiParams) { paramsPtr in
                                dash_sdk_decision_proposal_build(handle, ownerPtr, paramsPtr)
                            }
                        }
                    }
                }
            }
            return try DecisionProposalFFI.consumeBuildResult(result)
        }
    }

    /// The stored decision proposal `proposalId` names, as a document to edit
    /// with ``DecisionProposalDocument/edited(_:)``, or `nil` when there is
    /// none (`dash_sdk_decision_proposal_fetch_document`).
    ///
    /// - Parameter proposalId: The proposal's id, base58.
    public nonisolated func fetchDecisionProposalDocument(
        proposalId: String
    ) async throws -> DecisionProposalDocument? {
        try await performBlockingQuery { handle in
            let result = proposalId.withCString { idPtr in
                dash_sdk_decision_proposal_fetch_document(handle, idPtr)
            }
            return try DecisionProposalFFI.consumeDocument(result)
        }
    }
}

// MARK: - FFI result handling

/// Takes ownership of the `DashSDKResult`s the decision proposal functions
/// return: every owned string, document handle and error is freed on every
/// path, so only Swift values (or a ``DecisionProposalDocument`` that owns
/// its handle) leave these functions.
private enum DecisionProposalFFI {
    /// Throw the typed FFI error, freeing it.
    static func throwIfFailed(_ result: DashSDKResult) throws {
        guard let error = result.error else { return }
        // Keep the FFI error code, so a refused vote or create surfaces as a
        // consensus rejection and a transport failure as `.networkError` or
        // `.timeout`.
        let sdkError = SDKError.fromDashSDKError(error.pointee)
        dash_sdk_error_free(error)
        throw sdkError
    }

    /// The string the result carries, copied and freed; `nil` for no data.
    static func consumeString(_ result: DashSDKResult) throws -> String? {
        try throwIfFailed(result)
        guard let data = result.data else { return nil }
        let pointer = data.assumingMemoryBound(to: CChar.self)
        let string = String(cString: pointer)
        dash_sdk_string_free(pointer)
        return string
    }

    /// A listing page: a JSON array, `[]` when empty and never no data.
    static func consumeListing(_ result: DashSDKResult, function: String) throws -> [DecisionProposal] {
        guard let json = try consumeString(result) else {
            throw SDKError.internalError("\(function) returned no data")
        }
        return try DecisionProposalJSON.decode([DecisionProposal].self, from: json)
    }

    /// The document handle the result carries, now owned by the returned
    /// object; `nil` for no data.
    static func consumeDocument(_ result: DashSDKResult) throws -> DecisionProposalDocument? {
        try throwIfFailed(result)
        guard let data = result.data else { return nil }
        return DecisionProposalDocument(handle: OpaquePointer(data))
    }

    /// The document of a `DashSDKDocumentCreateResult`. The struct is freed;
    /// its document handle stays alive and is owned by the returned object.
    /// The entropy is not kept: the platform wallet's create draws its own.
    static func consumeBuildResult(_ result: DashSDKResult) throws -> DecisionProposalDocument {
        try throwIfFailed(result)
        guard let data = result.data else {
            throw SDKError.internalError("dash_sdk_decision_proposal_build returned no data")
        }
        let createResult = data.assumingMemoryBound(to: DashSDKDocumentCreateResult.self)
        let documentHandle = createResult.pointee.document_handle
        dash_sdk_document_create_result_free(createResult)
        guard let documentHandle else {
            throw SDKError.internalError("dash_sdk_decision_proposal_build returned no document handle")
        }
        return DecisionProposalDocument(handle: documentHandle)
    }
}

/// Run `body` with `string` as a C string, or with `nil` when it is `nil`.
private func withDecisionProposalCString<R>(
    _ string: String?,
    _ body: (UnsafePointer<CChar>?) throws -> R
) rethrows -> R {
    guard let string else { return try body(nil) }
    return try string.withCString { try body($0) }
}
