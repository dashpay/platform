import Foundation
import DashSDKFFI

// MARK: - Decision proposals (protocol version 15)
//
// Thin bridges over the `dash_sdk_decision_proposal_*` functions of
// `rs-sdk-ffi`, which call `dash_sdk::platform::decision_proposals` and
// `dash_sdk::platform::yes_no_vote_polls`. rs-sdk decides everything: which
// documents are proposals, the poll's id, the vote. These wrappers marshal
// arguments in and decode the JSON out. Every string goes to C through
// ``withDecisionProposalCString(_:field:invalid:_:)``, which refuses one
// with an embedded NUL.
//
// Every read and the vote park their thread inside Rust until DAPI answers,
// so they run on the shared Platform query queue
// (``SDK/performBlockingQuery(_:)``) and the caller only awaits.
//
// Creating and editing a proposal are platform wallet operations:
// `ManagedPlatformWallet.createDecisionProposal` and
// `ManagedPlatformWallet.editDecisionProposal`.

extension SDK {

    // MARK: - Reads

    /// The decision proposal `id` names, or `nil` when there is none
    /// (`dash_sdk_decision_proposal_fetch`).
    ///
    /// - Parameter id: The proposal's id, base58.
    public nonisolated func decisionProposal(id: String) async throws -> DecisionProposal? {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                id, field: "id", invalid: SDKError.invalidParameter
            ) { idPtr in
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
            let result = try withDecisionProposalCString(
                startAfter, field: "startAfter", invalid: SDKError.invalidParameter
            ) { startAfterPtr in
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
            let result = try withDecisionProposalCString(
                ownerId, field: "ownerId", invalid: SDKError.invalidParameter
            ) { ownerPtr in
                try withDecisionProposalCString(
                    startAfter, field: "startAfter", invalid: SDKError.invalidParameter
                ) { startAfterPtr in
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
            let result = try withDecisionProposalCString(
                startAfter, field: "startAfter", invalid: SDKError.invalidParameter
            ) { startAfterPtr in
                dash_sdk_decision_proposal_list_by_created_at(handle, limit ?? 0, startAfterPtr)
            }
            return try DecisionProposalFFI.consumeListing(
                result, function: "dash_sdk_decision_proposal_list_by_created_at")
        }
    }

    // MARK: - End date

    /// The end date to create a proposal with when the user asked for
    /// `requested` now (`dash_sdk_decision_proposal_get_end_date`).
    ///
    /// Platform accepts an end date from a week to four weeks after the block
    /// that creates the proposal, which comes seconds after now, so exactly a
    /// week from now is always refused. rs-sdk moves an end date within an
    /// hour of either edge of that window (as of this device's clock) to an
    /// hour inside it, so by at most two hours, and returns any other
    /// unchanged, for Platform to judge. A network below protocol version 15
    /// has no decision proposals, which is an error.
    ///
    /// `ManagedPlatformWallet.createDecisionProposal` puts the end date it is
    /// given as it is: give it this one.
    ///
    /// - Parameter requested: The end date the user picked.
    public nonisolated func decisionProposalEndDate(requested: Date) async throws -> Date {
        let requestedMs = requested.millisecondsSince1970
        let endDateMs = try await performBlockingQuery { handle in
            try DecisionProposalFFI.consumeMilliseconds(
                dash_sdk_decision_proposal_get_end_date(handle, requestedMs),
                function: "dash_sdk_decision_proposal_get_end_date"
            )
        }
        return Date(millisecondsSince1970: endDateMs)
    }

    /// The id (base58) of the yes/no poll a decision proposal opens, the id
    /// the end-date listing of vote polls names it by
    /// (`dash_sdk_decision_proposal_get_vote_poll_id`).
    ///
    /// - Parameter proposalId: The proposal's id, base58.
    public nonisolated func decisionProposalVotePollId(proposalId: String) async throws -> String {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                proposalId, field: "proposalId", invalid: SDKError.invalidParameter
            ) { idPtr in
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
            let result = try withDecisionProposalCString(
                proposalId, field: "proposalId", invalid: SDKError.invalidParameter
            ) { idPtr in
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
            let result = try proTxHash.withUnsafeBytes { proTxPtr in
                try withDecisionProposalCString(
                    proposalId, field: "proposalId", invalid: SDKError.invalidParameter
                ) { idPtr in
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
            let result = try proTxHash.withUnsafeBytes { proTxPtr in
                try votingPrivateKey.withUnsafeBytes { keyPtr in
                    try withDecisionProposalCString(
                        proposalId, field: "proposalId", invalid: SDKError.invalidParameter
                    ) { idPtr in
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

    /// Answer a decision proposal for a masternode with `signer` and wait for
    /// Platform's response
    /// (`dash_sdk_decision_proposal_cast_vote_with_signer`).
    ///
    /// The same vote as
    /// ``castDecisionProposalVote(proposalId:choice:proTxHash:votingPrivateKey:)``,
    /// but no key bytes cross the FFI: the Rust side builds the masternode's
    /// voting key from `votingAddress` and asks `signer` to sign with it. Use
    /// ``KeychainSigner/withStoredMasternodeVotingKey(identityId:isolation:perform:)``
    /// to make a loaded masternode identity's stored voting key signable for
    /// the call.
    ///
    /// - Parameters:
    ///   - proposalId: The proposal's id, base58.
    ///   - choice: Yes, no or abstain.
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash in **wire order**,
    ///     not the byte order of the hex Core displays (see
    ///     ``castContestedResourceVote(dataContractId:documentTypeName:indexName:indexValues:choice:proTxHash:votingPrivateKey:)``).
    ///   - votingAddress: The masternode's 20-byte voting address, the
    ///     hash160 of the voting public key.
    ///   - signer: Signs with the voting key. Kept alive for the whole call.
    public nonisolated func castDecisionProposalVote(
        proposalId: String,
        choice: YesNoAbstainVoteChoice,
        proTxHash: Data,
        votingAddress: Data,
        signer: KeychainSigner
    ) async throws {
        // The FFI reads exactly 32 and 20 bytes through these pointers.
        guard proTxHash.count == 32 else {
            throw SDKError.invalidParameter("proTxHash must be exactly 32 bytes")
        }
        guard votingAddress.count == 20 else {
            throw SDKError.invalidParameter("votingAddress must be exactly 20 bytes")
        }
        let voteChoice = choice.ffiTag

        try await performBlockingQuery { handle in
            // The signer's FFI handle holds an unretained pointer to
            // `signer`, so pin it for the whole call (see the lifetime
            // contract on `KeychainSigner`).
            let result = try withExtendedLifetime(signer) {
                try proTxHash.withUnsafeBytes { proTxPtr in
                    try votingAddress.withUnsafeBytes { addressPtr in
                        try withDecisionProposalCString(
                            proposalId, field: "proposalId", invalid: SDKError.invalidParameter
                        ) { idPtr in
                            dash_sdk_decision_proposal_cast_vote_with_signer(
                                handle,
                                idPtr,
                                voteChoice,
                                proTxPtr.bindMemory(to: UInt8.self).baseAddress,
                                addressPtr.bindMemory(to: UInt8.self).baseAddress,
                                signer.handle
                            )
                        }
                    }
                }
            }
            // No data on success.
            _ = try DecisionProposalFFI.consumeString(result)
        }
    }
}

// MARK: - FFI result handling

/// Takes ownership of the `DashSDKResult`s the decision proposal functions
/// (yes/no and multiple choice) return: every owned string and error is
/// freed on every path, so only Swift values leave these functions.
enum DecisionProposalFFI {
    /// Throw the typed FFI error, freeing it.
    static func throwIfFailed(_ result: DashSDKResult) throws {
        guard let error = result.error else { return }
        // Keep the FFI error code, so a refused vote surfaces as a
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

    /// A time in milliseconds since the Unix epoch, written by the FFI as a
    /// decimal string (not JSON).
    static func consumeMilliseconds(_ result: DashSDKResult, function: String) throws -> UInt64 {
        guard let string = try consumeString(result) else {
            throw SDKError.internalError("\(function) returned no data")
        }
        return try DecisionProposalJSON.milliseconds(from: string)
    }
}

extension DecisionProposalJSON {
    /// The milliseconds since the Unix epoch `string` writes as a decimal
    /// number, the form the end date read returns.
    static func milliseconds(from string: String) throws -> UInt64 {
        guard let milliseconds = UInt64(string) else {
            throw SDKError.serializationError("Not a time in milliseconds: \(string)")
        }
        return milliseconds
    }
}
