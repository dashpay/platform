import Foundation
import DashSDKFFI

// MARK: - Multiple choice proposals (protocol version 15)
//
// Thin bridges over the `dash_sdk_multiple_choice_proposal_*` functions of
// `rs-sdk-ffi`, which call
// `dash_sdk::platform::decision_proposals::multiple_choice`. rs-sdk decides
// everything: which documents are proposals and choices, the vote's
// contest, the fee and the fund. These wrappers marshal arguments in and
// decode the JSON out. Every string goes to C through
// ``withDecisionProposalCString(_:field:invalid:_:)``, which refuses one
// with an embedded NUL.
//
// Every read and the vote park their thread inside Rust until DAPI answers,
// so they run on the shared Platform query queue
// (``SDK/performBlockingQuery(_:)``) and the caller only awaits.
//
// Creating and editing a proposal and entering a choice are platform wallet
// operations: `ManagedPlatformWallet.createMultipleChoiceProposal`,
// `editMultipleChoiceProposal` and `enterMultipleChoiceProposalChoice`.

extension SDK {

    // MARK: - Reads

    /// The multiple choice proposal `id` names, or `nil` when there is none
    /// (`dash_sdk_multiple_choice_proposal_fetch`).
    ///
    /// - Parameter id: The proposal's id, base58.
    public nonisolated func multipleChoiceProposal(id: String) async throws -> MultipleChoiceProposal? {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                id, field: "id", invalid: SDKError.invalidParameter
            ) { idPtr in
                dash_sdk_multiple_choice_proposal_fetch(handle, idPtr)
            }
            return try DecisionProposalFFI.consumeString(result).map {
                try DecisionProposalJSON.decode(MultipleChoiceProposal.self, from: $0)
            }
        }
    }

    /// One page of an identity's multiple choice proposals, in the order it
    /// filed them (`dash_sdk_multiple_choice_proposal_list_by_owner`).
    ///
    /// - Parameters:
    ///   - ownerId: The proposer's identity id, base58.
    ///   - limit: The most proposals to return; `nil` for Platform's cap.
    ///   - startAfter: The id (base58) of the last proposal of the previous
    ///     page; `nil` for the first page.
    /// - Returns: The page; one shorter than `limit` is the last.
    public nonisolated func multipleChoiceProposalsByOwner(
        ownerId: String,
        limit: UInt32? = nil,
        startAfter: String? = nil
    ) async throws -> [MultipleChoiceProposal] {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                ownerId, field: "ownerId", invalid: SDKError.invalidParameter
            ) { ownerPtr in
                try withDecisionProposalCString(
                    startAfter, field: "startAfter", invalid: SDKError.invalidParameter
                ) { startAfterPtr in
                    // 0 is the FFI's "Platform's cap".
                    dash_sdk_multiple_choice_proposal_list_by_owner(
                        handle, ownerPtr, limit ?? 0, startAfterPtr)
                }
            }
            return try DecisionProposalFFI.consumeMultipleChoiceListing(
                result, function: "dash_sdk_multiple_choice_proposal_list_by_owner")
        }
    }

    /// One page of every multiple choice proposal, newest first
    /// (`dash_sdk_multiple_choice_proposal_list_by_created_at`).
    ///
    /// - Parameters:
    ///   - limit: The most proposals to return; `nil` for Platform's cap.
    ///   - startAfter: The id (base58) of the last proposal of the previous
    ///     page; `nil` for the first page.
    public nonisolated func multipleChoiceProposalsByCreatedAt(
        limit: UInt32? = nil,
        startAfter: String? = nil
    ) async throws -> [MultipleChoiceProposal] {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                startAfter, field: "startAfter", invalid: SDKError.invalidParameter
            ) { startAfterPtr in
                dash_sdk_multiple_choice_proposal_list_by_created_at(handle, limit ?? 0, startAfterPtr)
            }
            return try DecisionProposalFFI.consumeMultipleChoiceListing(
                result, function: "dash_sdk_multiple_choice_proposal_list_by_created_at")
        }
    }

    /// The vote among a multiple choice proposal's choices: every choice with
    /// its tally while the vote runs, and how it ended once it did
    /// (`dash_sdk_multiple_choice_proposal_get_vote`).
    ///
    /// - Parameter proposalId: The proposal's id, base58.
    public nonisolated func multipleChoiceProposalVote(
        proposalId: String
    ) async throws -> MultipleChoiceProposalVote {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                proposalId, field: "proposalId", invalid: SDKError.invalidParameter
            ) { idPtr in
                dash_sdk_multiple_choice_proposal_get_vote(handle, idPtr)
            }
            guard let json = try DecisionProposalFFI.consumeString(result) else {
                throw SDKError.internalError("dash_sdk_multiple_choice_proposal_get_vote returned no data")
            }
            return try DecisionProposalJSON.decode(MultipleChoiceProposalVote.self, from: json)
        }
    }

    /// The choice the finished vote of a multiple choice proposal kept, or
    /// `nil` while the vote runs, when it kept none of the choices, and for a
    /// proposal no choice was entered for
    /// (`dash_sdk_multiple_choice_proposal_get_kept_choice`).
    ///
    /// - Parameter proposalId: The proposal's id, base58.
    public nonisolated func multipleChoiceProposalKeptChoice(
        proposalId: String
    ) async throws -> ProposalChoice? {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                proposalId, field: "proposalId", invalid: SDKError.invalidParameter
            ) { idPtr in
                dash_sdk_multiple_choice_proposal_get_kept_choice(handle, idPtr)
            }
            return try DecisionProposalFFI.consumeString(result).map {
                try DecisionProposalJSON.decode(ProposalChoice.self, from: $0)
            }
        }
    }

    /// A masternode's answer in the vote among a multiple choice proposal's
    /// choices, or `nil` when it has not voted in it or the vote has ended
    /// (`dash_sdk_multiple_choice_proposal_get_masternode_vote`).
    ///
    /// - Parameters:
    ///   - proposalId: The proposal's id, base58.
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash in **wire order**,
    ///     the same bytes
    ///     ``castMultipleChoiceProposalVote(proposalId:choice:proTxHash:votingPrivateKey:)``
    ///     takes.
    public nonisolated func multipleChoiceProposalMasternodeVote(
        proposalId: String,
        proTxHash: Data
    ) async throws -> MultipleChoiceVoteChoice? {
        // The FFI reads exactly 32 bytes through the pointer.
        guard proTxHash.count == 32 else {
            throw SDKError.invalidParameter("proTxHash must be exactly 32 bytes")
        }
        return try await performBlockingQuery { handle in
            let result = try proTxHash.withUnsafeBytes { proTxPtr in
                try withDecisionProposalCString(
                    proposalId, field: "proposalId", invalid: SDKError.invalidParameter
                ) { idPtr in
                    dash_sdk_multiple_choice_proposal_get_masternode_vote(
                        handle,
                        proTxPtr.bindMemory(to: UInt8.self).baseAddress,
                        idPtr
                    )
                }
            }
            return try DecisionProposalFFI.consumeString(result).map {
                try DecisionProposalJSON.decode(MultipleChoiceVoteChoice.self, from: $0)
            }
        }
    }

    // MARK: - Fee and fund

    /// What creating one multiple choice proposal adds to the processing fee
    /// of its batch, in credits, which Platform charges the proposer on top
    /// of the fees (`dash_sdk_multiple_choice_proposal_get_processing_fee`).
    /// A network below protocol version 15 has no multiple choice proposals,
    /// which is an error.
    public nonisolated func multipleChoiceProposalProcessingFee() async throws -> UInt64 {
        try await performBlockingQuery { handle in
            try DecisionProposalFFI.consumeCredits(
                dash_sdk_multiple_choice_proposal_get_processing_fee(handle),
                function: "dash_sdk_multiple_choice_proposal_get_processing_fee"
            )
        }
    }

    /// The fund, in credits, a choice entered now for the multiple choice
    /// proposal `proposalId` states
    /// (`dash_sdk_multiple_choice_proposal_get_choice_fund_to_join`). The
    /// choice's put reads and states it itself; this is for showing it
    /// beforehand.
    ///
    /// - Parameter proposalId: The proposal's id, base58.
    public nonisolated func multipleChoiceProposalChoiceFundToJoin(
        proposalId: String
    ) async throws -> UInt64 {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                proposalId, field: "proposalId", invalid: SDKError.invalidParameter
            ) { idPtr in
                dash_sdk_multiple_choice_proposal_get_choice_fund_to_join(handle, idPtr)
            }
            return try DecisionProposalFFI.consumeCredits(
                result, function: "dash_sdk_multiple_choice_proposal_get_choice_fund_to_join")
        }
    }

    // MARK: - Vote

    /// Vote in the vote among a multiple choice proposal's choices for a
    /// masternode and wait for Platform's response
    /// (`dash_sdk_multiple_choice_proposal_cast_vote`).
    ///
    /// The vote is built, signed with the masternode voting key derived from
    /// `votingPrivateKey` and broadcast on the Rust side, exactly as
    /// ``castDecisionProposalVote(proposalId:choice:proTxHash:votingPrivateKey:)``
    /// does for a yes/no proposal. A wallet that is not a masternode has no
    /// voter identity, so Platform refuses its vote.
    ///
    /// - Parameters:
    ///   - proposalId: The proposal's id, base58.
    ///   - choice: For a choice (by its document id), for none of the
    ///     choices, or abstain.
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash in **wire order**,
    ///     not the byte order of the hex Core displays (see
    ///     ``castContestedResourceVote(dataContractId:documentTypeName:indexName:indexValues:choice:proTxHash:votingPrivateKey:)``).
    ///   - votingPrivateKey: The masternode's 32-byte voting private key. It
    ///     is not retained.
    public nonisolated func castMultipleChoiceProposalVote(
        proposalId: String,
        choice: MultipleChoiceVoteChoice,
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
        let choiceId = choice.choiceId
        let networkValue = network.ffiValue

        try await performBlockingQuery { handle in
            let result = try proTxHash.withUnsafeBytes { proTxPtr in
                try votingPrivateKey.withUnsafeBytes { keyPtr in
                    try withDecisionProposalCString(
                        proposalId, field: "proposalId", invalid: SDKError.invalidParameter
                    ) { idPtr in
                        try withDecisionProposalCString(
                            choiceId, field: "choiceId", invalid: SDKError.invalidParameter
                        ) { choiceIdPtr in
                            dash_sdk_multiple_choice_proposal_cast_vote(
                                handle,
                                idPtr,
                                voteChoice,
                                choiceIdPtr,
                                proTxPtr.bindMemory(to: UInt8.self).baseAddress,
                                keyPtr.bindMemory(to: UInt8.self).baseAddress,
                                networkValue
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

extension DecisionProposalFFI {
    /// A multiple choice listing page: a JSON array, `[]` when empty and
    /// never no data.
    static func consumeMultipleChoiceListing(
        _ result: DashSDKResult,
        function: String
    ) throws -> [MultipleChoiceProposal] {
        guard let json = try consumeString(result) else {
            throw SDKError.internalError("\(function) returned no data")
        }
        return try DecisionProposalJSON.decode([MultipleChoiceProposal].self, from: json)
    }

    /// An amount of credits, written by the FFI as a decimal string (not
    /// JSON).
    static func consumeCredits(_ result: DashSDKResult, function: String) throws -> UInt64 {
        guard let string = try consumeString(result) else {
            throw SDKError.internalError("\(function) returned no data")
        }
        return try DecisionProposalJSON.credits(from: string)
    }
}

extension DecisionProposalJSON {
    /// The credits `string` writes as a decimal number, the form the fee and
    /// fund reads return.
    static func credits(from string: String) throws -> UInt64 {
        guard let credits = UInt64(string) else {
            throw SDKError.serializationError("Not an amount of credits: \(string)")
        }
        return credits
    }
}
