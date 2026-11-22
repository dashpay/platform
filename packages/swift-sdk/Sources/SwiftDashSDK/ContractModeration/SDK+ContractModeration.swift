import Foundation
import DashSDKFFI

// MARK: - Contract moderation and fee pots (protocol version 14)
//
// Thin bridges over the `dash_sdk_contract_moderation_*` and
// `dash_sdk_contract_get_fee_pots` functions of `rs-sdk-ffi`, which call
// `dash_sdk::platform::contract_moderation` and
// `dash_sdk::platform::contract_fee_pots`. rs-sdk decides everything: which
// lists a contract keeps, the page caps, the cursor of the next page, how a
// removal record's kept values read. These wrappers marshal arguments in and
// decode the JSON out; a page's cursor is passed back as it came. Every
// string goes to C through ``withDecisionProposalCString(_:field:invalid:_:)``,
// which refuses one with an embedded NUL.
//
// Every read parks its thread inside Rust until DAPI answers, so it runs on
// the shared Platform query queue (``SDK/performBlockingQuery(_:)``) and the
// caller only awaits. A refusal from the node (a list the contract does not
// keep, a type that keeps no removal records, a contract that is not
// elected) is thrown as the `SDKError` the FFI reports.
//
// Below protocol version 14 no request is sent: the SDK's protocol gate
// refuses it first, and every method below throws. The error tells why:
// - `SDKError.protocolError` when the network is proven below 14, or the SDK
//   is pinned below 14: check the pin, or wait for the network to activate 14;
// - `SDKError.networkError` when the query for the network's version failed
//   for a reason that can pass: retry.

extension SDK {

    /// One identity's status on a moderated contract, on the lists read
    /// (`dash_sdk_contract_moderation_get_status`).
    ///
    /// - Parameters:
    ///   - contractId: The moderated contract's id, base58.
    ///   - identityId: The identity's id, base58.
    ///   - lists: The lists to read, each one the contract keeps. `nil` reads
    ///     every list the contract keeps (rs-sdk fetches the contract to find
    ///     them); `[]` is an explicit empty selection, sent for the node to
    ///     judge.
    /// - Returns: The status, or `nil` when the FFI returns no data.
    /// - Throws: The FFI's `SDKError`; below protocol version 14 a refusal
    ///   of the SDK's protocol gate, before any request (see the top of this
    ///   file for telling its causes apart).
    public nonisolated func contractModerationStatus(
        contractId: String,
        identityId: String,
        lists: [ContractModerationList]? = nil
    ) async throws -> ContractModerationStatus? {
        return try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                contractId, field: "contractId", invalid: SDKError.invalidParameter
            ) { contractPtr in
                try withDecisionProposalCString(
                    identityId, field: "identityId", invalid: SDKError.invalidParameter
                ) { identityPtr in
                    withContractModerationLists(lists) { listsPtr, count in
                        dash_sdk_contract_moderation_get_status(
                            handle, contractPtr, identityPtr, listsPtr, UInt(count))
                    }
                }
            }
            return try ContractModerationFFI.decode(ContractModerationStatus.self, result)
        }
    }

    /// One page of a moderated contract's banlist, suspension list or warning
    /// list, in identity id order (`dash_sdk_contract_moderation_get_entries`).
    ///
    /// - Parameters:
    ///   - contractId: The moderated contract's id, base58.
    ///   - list: The list; the contract must keep it.
    ///   - startAfter: The previous page's ``ContractModerationEntriesPage/nextStartAfter``;
    ///     `nil` for the first page.
    ///   - limit: The most entries to return, passed as it is (0 included,
    ///     which the node judges); `nil` for the page cap of the network
    ///     queried.
    /// - Throws: The FFI's `SDKError`; below protocol version 14 a refusal
    ///   of the SDK's protocol gate, before any request (see the top of this
    ///   file for telling its causes apart).
    public nonisolated func contractModerationEntries(
        contractId: String,
        list: ContractModerationList,
        startAfter: String? = nil,
        limit: UInt32? = nil
    ) async throws -> ContractModerationEntriesPage? {
        let listByte = list.ffiTag
        return try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                contractId, field: "contractId", invalid: SDKError.invalidParameter
            ) { contractPtr in
                try withDecisionProposalCString(
                    startAfter, field: "startAfter", invalid: SDKError.invalidParameter
                ) { startAfterPtr in
                    withContractModerationLimit(limit) { limitPtr in
                        dash_sdk_contract_moderation_get_entries(
                            handle, contractPtr, listByte, startAfterPtr, limitPtr)
                    }
                }
            }
            return try ContractModerationFFI.decode(ContractModerationEntriesPage.self, result)
        }
    }

    /// One page of the records of the documents a contract's moderators
    /// deleted within one document type, in document id order
    /// (`dash_sdk_contract_moderation_get_document_removals`).
    ///
    /// - Parameters:
    ///   - contractId: The moderated contract's id, base58.
    ///   - documentTypeName: The document type; the node refuses one that
    ///     keeps no records.
    ///   - startAfter: The previous page's ``ContractDocumentRemovalsPage/nextStartAfter``;
    ///     `nil` for the first page.
    ///   - limit: The most records to return, passed as it is (0 included,
    ///     which the node judges); `nil` for the page cap of the network
    ///     queried.
    /// - Throws: The FFI's `SDKError`; below protocol version 14 a refusal
    ///   of the SDK's protocol gate, before any request (see the top of this
    ///   file for telling its causes apart).
    public nonisolated func contractDocumentRemovals(
        contractId: String,
        documentTypeName: String,
        startAfter: String? = nil,
        limit: UInt32? = nil
    ) async throws -> ContractDocumentRemovalsPage? {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                contractId, field: "contractId", invalid: SDKError.invalidParameter
            ) { contractPtr in
                try withDecisionProposalCString(
                    documentTypeName, field: "documentTypeName", invalid: SDKError.invalidParameter
                ) { typePtr in
                    try withDecisionProposalCString(
                        startAfter, field: "startAfter", invalid: SDKError.invalidParameter
                    ) { startAfterPtr in
                        withContractModerationLimit(limit) { limitPtr in
                            dash_sdk_contract_moderation_get_document_removals(
                                handle, contractPtr, typePtr, startAfterPtr, limitPtr)
                        }
                    }
                }
            }
            return try ContractModerationFFI.decode(ContractDocumentRemovalsPage.self, result)
        }
    }

    /// The removal records of the documents named, within one document type
    /// (`dash_sdk_contract_moderation_get_document_removals_by_ids`). A
    /// document with no record is left out of the answer.
    ///
    /// - Parameters:
    ///   - contractId: The moderated contract's id, base58.
    ///   - documentTypeName: The document type; the node refuses one that
    ///     keeps no records.
    ///   - documentIds: The documents' ids, base58. How many the node takes is
    ///     the node's to judge.
    /// - Throws: The FFI's `SDKError`; below protocol version 14 a refusal
    ///   of the SDK's protocol gate, before any request (see the top of this
    ///   file for telling its causes apart).
    public nonisolated func contractDocumentRemovals(
        contractId: String,
        documentTypeName: String,
        documentIds: [String]
    ) async throws -> ContractDocumentRemovalsPage? {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                contractId, field: "contractId", invalid: SDKError.invalidParameter
            ) { contractPtr in
                try withDecisionProposalCString(
                    documentTypeName, field: "documentTypeName", invalid: SDKError.invalidParameter
                ) { typePtr in
                    try withContractModerationCStrings(documentIds, field: "documentIds") { idsPtr, count in
                        dash_sdk_contract_moderation_get_document_removals_by_ids(
                            handle, contractPtr, typePtr, idsPtr, UInt(count))
                    }
                }
            }
            return try ContractModerationFFI.decode(ContractDocumentRemovalsPage.self, result)
        }
    }

    /// One page of the actions an elected contract's seated moderation team
    /// votes on, in action id order
    /// (`dash_sdk_contract_moderation_get_team_actions`).
    /// The node refuses it unless at least one of the contract's document
    /// types sets `moderatorAbilities.deleteSettled` (which only an elected
    /// contract may set); a contract that keeps team actions but has none
    /// answers an empty page.
    ///
    /// - Parameters:
    ///   - contractId: The elected contract's id, base58.
    ///   - status: Active (still gathering approvals) or closed (they ran).
    ///   - startAtActionId: Start the page at this action, base58: the
    ///     previous page's ``ContractTeamActionsPage/nextStartAtActionId``;
    ///     `nil` for the first page.
    ///   - startAtActionIdIncluded: Whether the page includes that action
    ///     itself: the previous page's
    ///     ``ContractTeamActionsPage/nextStartAtActionIdIncluded``. The FFI
    ///     refuses it without `startAtActionId`.
    ///   - limit: The most actions to return, passed as it is (0 included,
    ///     which the node judges); `nil` for the page cap of the network
    ///     queried.
    /// - Throws: The FFI's `SDKError`; below protocol version 14 a refusal
    ///   of the SDK's protocol gate, before any request (see the top of this
    ///   file for telling its causes apart).
    public nonisolated func contractTeamActions(
        contractId: String,
        status: ContractTeamActionStatus,
        startAtActionId: String? = nil,
        startAtActionIdIncluded: Bool = false,
        limit: UInt32? = nil
    ) async throws -> ContractTeamActionsPage? {
        let statusByte = status.ffiTag
        return try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                contractId, field: "contractId", invalid: SDKError.invalidParameter
            ) { contractPtr in
                try withDecisionProposalCString(
                    startAtActionId, field: "startAtActionId", invalid: SDKError.invalidParameter
                ) { startPtr in
                    withContractModerationLimit(limit) { limitPtr in
                        dash_sdk_contract_moderation_get_team_actions(
                            handle, contractPtr, statusByte, startPtr, startAtActionIdIncluded, limitPtr)
                    }
                }
            }
            return try ContractModerationFFI.decode(ContractTeamActionsPage.self, result)
        }
    }

    /// Who approved one team action, in identity id order
    /// (`dash_sdk_contract_moderation_get_team_action_signers`). The action is
    /// looked up among the actions of `status` alone. The node refuses it
    /// unless at least one of the contract's document types sets
    /// `moderatorAbilities.deleteSettled` (which only an elected contract may
    /// set).
    ///
    /// - Parameters:
    ///   - contractId: The elected contract's id, base58.
    ///   - status: Whether the action is active or closed.
    ///   - actionId: The action's id, base58.
    /// - Throws: The FFI's `SDKError`; below protocol version 14 a refusal
    ///   of the SDK's protocol gate, before any request (see the top of this
    ///   file for telling its causes apart).
    public nonisolated func contractTeamActionSigners(
        contractId: String,
        status: ContractTeamActionStatus,
        actionId: String
    ) async throws -> ContractTeamActionSigners? {
        let statusByte = status.ffiTag
        return try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                contractId, field: "contractId", invalid: SDKError.invalidParameter
            ) { contractPtr in
                try withDecisionProposalCString(
                    actionId, field: "actionId", invalid: SDKError.invalidParameter
                ) { actionPtr in
                    dash_sdk_contract_moderation_get_team_action_signers(
                        handle, contractPtr, statusByte, actionPtr)
                }
            }
            return try ContractModerationFFI.decode(ContractTeamActionSigners.self, result)
        }
    }

    /// How many counted moderation actions each member of an elected
    /// contract's seated team signed since the moderators pot was last claimed
    /// or settled (`dash_sdk_contract_moderation_get_action_counts`). A
    /// moderators pot settlement (a membership change of the team) resets
    /// the counts even when it pays nothing out.
    ///
    /// - Parameter contractId: The elected contract's id, base58.
    /// - Throws: The FFI's `SDKError`; below protocol version 14 a refusal
    ///   of the SDK's protocol gate, before any request (see the top of this
    ///   file for telling its causes apart).
    public nonisolated func contractModerationActionCounts(
        contractId: String
    ) async throws -> ContractModerationActionCounts? {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                contractId, field: "contractId", invalid: SDKError.invalidParameter
            ) { contractPtr in
                dash_sdk_contract_moderation_get_action_counts(handle, contractPtr)
            }
            return try ContractModerationFFI.decode(ContractModerationActionCounts.self, result)
        }
    }

    /// What the document action fees of a contract have collected for its
    /// owner and its moderation team, and the last explicit claim of each pot
    /// (`dash_sdk_contract_get_fee_pots`).
    ///
    /// - Parameter contractId: The contract's id, base58.
    /// - Throws: The FFI's `SDKError`; below protocol version 14 a refusal
    ///   of the SDK's protocol gate, before any request (see the top of this
    ///   file for telling its causes apart).
    public nonisolated func contractFeePots(contractId: String) async throws -> ContractFeePots? {
        try await performBlockingQuery { handle in
            let result = try withDecisionProposalCString(
                contractId, field: "contractId", invalid: SDKError.invalidParameter
            ) { contractPtr in
                dash_sdk_contract_get_fee_pots(handle, contractPtr)
            }
            return try ContractModerationFFI.decode(ContractFeePots.self, result)
        }
    }
}

// MARK: - FFI result handling

/// Decodes the results of the contract moderation functions; the owned
/// string and error are freed by ``DecisionProposalFFI/consumeString(_:)``.
enum ContractModerationFFI {

    /// The JSON the result carries, decoded; `nil` for no data.
    static func decode<T: Decodable>(_ type: T.Type, _ result: DashSDKResult) throws -> T? {
        try DecisionProposalFFI.consumeString(result).map {
            try ContractModerationJSON.decode(type, from: $0)
        }
    }
}

/// Decodes the JSON the contract moderation FFI functions return. Shared by
/// the wrappers and the hermetic decoder tests, so both read the same way.
enum ContractModerationJSON {
    static func decode<T: Decodable>(_ type: T.Type, from json: String) throws -> T {
        try DecisionProposalJSON.decode(type, from: json)
    }

}

/// Run `body` with `lists` as the `lists` pointer and count
/// `dash_sdk_contract_moderation_get_status` takes: a null pointer with a
/// count of 0 for `nil` (no selection: every list the contract keeps), a
/// non-null pointer with a count of 0 for `[]` (an explicit empty selection,
/// for the node to judge; an empty array's buffer may have no base address, so
/// a byte that is never read stands in), and the lists' bytes otherwise.
func withContractModerationLists<R>(
    _ lists: [ContractModerationList]?,
    _ body: (UnsafePointer<UInt8>?, Int) throws -> R
) rethrows -> R {
    guard let lists else { return try body(nil, 0) }
    let bytes = lists.map(\.ffiTag)
    if bytes.isEmpty {
        var placeholder: UInt8 = 0
        return try withUnsafePointer(to: &placeholder) { try body($0, 0) }
    }
    return try bytes.withUnsafeBufferPointer { try body($0.baseAddress, $0.count) }
}

/// Run `body` with a pointer to `limit`, or a null pointer for `nil` (the
/// page cap of the network queried).
func withContractModerationLimit<R>(
    _ limit: UInt32?,
    _ body: (UnsafePointer<UInt32>?) throws -> R
) rethrows -> R {
    guard var limit else { return try body(nil) }
    return try withUnsafePointer(to: &limit) { try body($0) }
}

/// Run `body` with `strings` as an array of NUL-terminated C strings and its
/// count. Each string is refused, as by
/// ``withDecisionProposalCString(_:field:invalid:_:)``, when it holds a NUL;
/// the copies are freed when `body` returns.
func withContractModerationCStrings<R>(
    _ strings: [String],
    field: String,
    _ body: (UnsafePointer<UnsafePointer<CChar>?>?, Int) throws -> R
) throws -> R {
    if strings.contains(where: { $0.utf8.contains(0) }) {
        throw SDKError.invalidParameter("\(field) contains a NUL character, which a C string cannot carry")
    }
    let copies: [UnsafeMutablePointer<CChar>?] = strings.map { strdup($0) }
    defer { copies.forEach { free($0) } }
    let pointers: [UnsafePointer<CChar>?] = copies.map { $0.map { UnsafePointer($0) } }
    return try pointers.withUnsafeBufferPointer { buffer in
        try body(buffer.baseAddress, buffer.count)
    }
}
