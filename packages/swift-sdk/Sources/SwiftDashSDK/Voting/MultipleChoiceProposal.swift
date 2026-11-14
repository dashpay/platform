import Foundation
import DashSDKFFI

// MARK: - Proposal

/// A multiple choice proposal: a question an identity puts to the
/// masternodes, answered by the choices entered for it. Decoded from the JSON
/// every multiple choice proposal read returns
/// (`dash_sdk_multiple_choice_proposal_fetch` and the two listings).
public struct MultipleChoiceProposal: Decodable, Sendable, Equatable, Hashable, Identifiable {
    /// The document a proposal points at for what is being decided.
    public struct DescriptionDocument: Decodable, Sendable, Equatable, Hashable {
        /// The document's id, base58.
        public let id: String
        /// The id of the contract it lives in, base58.
        public let contractId: String
        /// Its document type.
        public let documentType: String
    }

    /// The proposal's id, base58.
    public let id: String
    /// The proposer's identity id, base58.
    public let ownerId: String
    /// `nil` when the document does not carry it.
    public let revision: UInt64?
    /// Milliseconds since the Unix epoch; `nil` when the document does not
    /// carry it.
    public let createdAtMs: UInt64?
    /// Milliseconds since the Unix epoch; `nil` when the document does not
    /// carry it.
    public let updatedAtMs: UInt64?
    /// The question, in one line.
    public let title: String
    /// The document saying what is being decided; `nil` when it has none.
    public let descriptionDocument: DescriptionDocument?
    /// Where what is being decided is described or discussed; `nil` when it
    /// has none.
    public let url: String?
    /// Whether anyone may enter choices; only the proposer may when `false`.
    public let open: Bool

    private enum CodingKeys: String, CodingKey {
        case id
        case ownerId
        case revision
        case createdAtMs = "createdAt"
        case updatedAtMs = "updatedAt"
        case title
        case descriptionDocument
        case url
        case open
    }

    /// When the proposal was created, if the document carries it.
    public var createdAt: Date? { createdAtMs.map(Date.init(millisecondsSince1970:)) }
    /// When the proposal was last edited, if the document carries it.
    public var updatedAt: Date? { updatedAtMs.map(Date.init(millisecondsSince1970:)) }
}

// MARK: - Choice

/// A choice entered for a multiple choice proposal: one answer the
/// masternodes may vote for. Decoded from the choice JSON
/// (`dash_sdk_multiple_choice_proposal_get_kept_choice`, and each running
/// choice of ``MultipleChoiceProposalVote``).
public struct ProposalChoice: Decodable, Sendable, Equatable, Hashable, Identifiable {
    /// The choice's document id, base58: the id a vote for it names.
    public let id: String
    /// The identity that entered it, base58.
    public let ownerId: String
    /// The proposal it answers, base58.
    public let proposalId: String
    /// `nil` when the document does not carry it.
    public let revision: UInt64?
    /// Milliseconds since the Unix epoch; `nil` when the document does not
    /// carry it.
    public let createdAtMs: UInt64?
    /// Milliseconds since the Unix epoch; `nil` when the document does not
    /// carry it.
    public let updatedAtMs: UInt64?
    /// The answer, in one line.
    public let title: String
    /// What voting for the answer means; `nil` when it has none.
    public let description: String?

    private enum CodingKeys: String, CodingKey {
        case id
        case ownerId
        case proposalId
        case revision
        case createdAtMs = "createdAt"
        case updatedAtMs = "updatedAt"
        case title
        case description
    }

    /// When the choice was entered, if the document carries it.
    public var createdAt: Date? { createdAtMs.map(Date.init(millisecondsSince1970:)) }
}

// MARK: - Vote

/// The vote among a multiple choice proposal's choices
/// (`dash_sdk_multiple_choice_proposal_get_vote`).
public struct MultipleChoiceProposalVote: Decodable, Sendable, Equatable {
    /// One choice in the vote.
    public struct ChoiceInVote: Decodable, Sendable, Equatable, Identifiable {
        /// The choice's document id, base58: the id a vote for it names.
        public let id: String
        /// The choice while the vote runs; `nil` once it ended (the choices
        /// not kept are gone, and the kept one is read with
        /// ``SDK/multipleChoiceProposalKeptChoice(proposalId:)``).
        public let choice: ProposalChoice?
        /// The votes for it; `nil` when Platform does not report it.
        public let voteTally: UInt32?
    }

    /// How a finished vote ended.
    public enum Outcome: String, Decodable, Sendable, Equatable {
        /// The choice ``Result/keptChoiceId`` names was kept.
        case keptChoice
        /// The votes for none of the choices outnumbered the leading choice;
        /// no choice was kept.
        case noneOfTheChoices
        /// No choice won; none was kept.
        case noWinner
    }

    /// The record of a finished vote.
    public struct Result: Decodable, Sendable, Equatable {
        public let outcome: Outcome
        /// The kept choice's id, base58; `nil` unless ``outcome`` is
        /// ``Outcome/keptChoice``.
        public let keptChoiceId: String?
        /// The block the vote was closed in.
        public let finalizationBlock: PlatformBlockInfo
    }

    /// Every choice, ordered by choice id; empty for a proposal no choice
    /// was entered for.
    public let choices: [ChoiceInVote]
    /// The abstaining votes; `nil` when Platform does not report them.
    public let abstainVoteTally: UInt32?
    /// The votes for none of the choices; `nil` when Platform does not
    /// report them.
    public let noneOfTheChoicesVoteTally: UInt32?
    /// How the vote ended; `nil` while it runs.
    public let result: Result?
}

/// A masternode's answer in the vote among a multiple choice proposal's
/// choices, as rs-sdk's `MultipleChoiceVoteChoice`.
///
/// Decodes the masternode-answer JSON
/// (`dash_sdk_multiple_choice_proposal_get_masternode_vote`):
/// `{"choice": "choice" | "noneOfTheChoices" | "abstain", "choiceId": ...}`.
/// ``ffiTag`` and ``choiceId`` are the `vote_choice` byte and `choice_id`
/// string `dash_sdk_multiple_choice_proposal_cast_vote` takes.
public enum MultipleChoiceVoteChoice: Decodable, Sendable, Equatable, Hashable {
    /// For the choice whose document id (base58) is `id`, not the id of the
    /// identity that entered it.
    case choice(id: String)
    /// For none of the choices: keeps no choice when these votes outnumber
    /// the leading choice.
    case noneOfTheChoices
    case abstain

    /// The `vote_choice` byte passed across the FFI. The C parameter is a
    /// plain `uint8_t` with no enum in the header, so the three values are
    /// mirrored here and must agree with `MultipleChoiceVoteChoiceFFI` in
    /// `packages/rs-sdk-ffi/src/decision_proposal/multiple_choice/mod.rs`:
    /// `Choice = 0`, `NoneOfTheChoices = 1`, `Abstain = 2`.
    var ffiTag: UInt8 {
        switch self {
        case .choice: return 0
        case .noneOfTheChoices: return 1
        case .abstain: return 2
        }
    }

    /// The choice voted for, base58; `nil` for the answers that name none
    /// (the FFI requires a null `choice_id` then).
    public var choiceId: String? {
        guard case .choice(let id) = self else { return nil }
        return id
    }

    private enum CodingKeys: String, CodingKey {
        case choice
        case choiceId
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let name = try container.decode(String.self, forKey: .choice)
        switch name {
        case "choice":
            self = .choice(id: try container.decode(String.self, forKey: .choiceId))
        case "noneOfTheChoices":
            self = .noneOfTheChoices
        case "abstain":
            self = .abstain
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .choice,
                in: container,
                debugDescription: "Unknown multiple choice vote answer \(name)"
            )
        }
    }
}

// MARK: - Params

/// The document a new or edited multiple choice proposal points at
/// (`DescriptionDocumentFFI`). Platform judges whether it may be pointed at;
/// nothing is checked here.
public struct DescriptionDocumentParams: Sendable, Equatable {
    /// The document's 32-byte id.
    public var id: Identifier
    /// The 32-byte id of the contract it lives in.
    public var contractId: Identifier
    /// Its document type.
    public var documentType: String

    public init(id: Identifier, contractId: Identifier, documentType: String) {
        self.id = id
        self.contractId = contractId
        self.documentType = documentType
    }
}

/// The question of a new multiple choice proposal, as
/// `ManagedPlatformWallet.createMultipleChoiceProposal` passes it on
/// (`MultipleChoiceProposalParamsFFI`). Platform judges every value when the
/// proposal is created; nothing is checked here.
public struct MultipleChoiceProposalParams: Sendable, Equatable {
    /// The question, in one line.
    public var title: String
    /// The document saying what is being decided; `nil` for none.
    public var descriptionDocument: DescriptionDocumentParams?
    /// Where what is being decided is described or discussed; `nil` for
    /// none.
    public var url: String?
    /// Whether anyone may enter choices; only the proposer may when `false`.
    /// It never changes.
    public var open: Bool

    public init(
        title: String,
        descriptionDocument: DescriptionDocumentParams? = nil,
        url: String? = nil,
        open: Bool
    ) {
        self.title = title
        self.descriptionDocument = descriptionDocument
        self.url = url
        self.open = open
    }
}

/// What a stored multiple choice proposal changes to, as
/// `ManagedPlatformWallet.editMultipleChoiceProposal` passes it on
/// (`MultipleChoiceProposalEditFFI`). A field left `nil` (or `.keep`) keeps
/// its value. rs-sdk refuses an edit that changes nothing; Platform accepts
/// an edit only within its window after the proposal was created, and never
/// a change to whether it is open.
public struct MultipleChoiceProposalEdit: Sendable, Equatable {
    /// What happens to an optional field.
    public enum Change<Value: Sendable & Equatable>: Sendable, Equatable {
        /// Keep the value as it is (or keep having none).
        case keep
        /// Set the value.
        case set(Value)
        /// Remove the value.
        case remove
    }

    /// The new title; `nil` keeps it.
    public var title: String?
    /// The description document change.
    public var descriptionDocument: Change<DescriptionDocumentParams>
    /// The url change.
    public var url: Change<String>

    public init(
        title: String? = nil,
        descriptionDocument: Change<DescriptionDocumentParams> = .keep,
        url: Change<String> = .keep
    ) {
        self.title = title
        self.descriptionDocument = descriptionDocument
        self.url = url
    }
}

/// An answer to enter for a multiple choice proposal, as
/// `ManagedPlatformWallet.enterMultipleChoiceProposalChoice` passes it on
/// (`ChoiceParamsFFI`). Platform judges every value; nothing is checked here.
public struct ProposalChoiceParams: Sendable, Equatable {
    /// The answer, in one line.
    public var title: String
    /// What voting for the answer means; `nil` for none.
    public var description: String?

    public init(title: String, description: String? = nil) {
        self.title = title
        self.description = description
    }
}

// MARK: - FFI marshalling

/// Run `body` with `document` as a `DescriptionDocumentFFI`, or with `nil`
/// when `document` is `nil`. The FFI reads exactly 32 bytes through each id
/// pointer, so an id of another length is refused before any pointer is
/// made; the document type goes through ``withDecisionProposalCString(_:field:invalid:_:)``.
func withDescriptionDocumentFFI<R>(
    _ document: DescriptionDocumentParams?,
    _ body: (UnsafePointer<DescriptionDocumentFFI>?) throws -> R
) throws -> R {
    guard let document else { return try body(nil) }
    guard document.id.count == 32 else {
        throw PlatformWalletError.invalidParameter("descriptionDocument.id must be exactly 32 bytes")
    }
    guard document.contractId.count == 32 else {
        throw PlatformWalletError.invalidParameter(
            "descriptionDocument.contractId must be exactly 32 bytes"
        )
    }
    let idBytes = [UInt8](document.id)
    let contractIdBytes = [UInt8](document.contractId)
    return try idBytes.withUnsafeBufferPointer { idBp in
        try contractIdBytes.withUnsafeBufferPointer { contractBp in
            try withDecisionProposalCString(
                document.documentType,
                field: "descriptionDocument.documentType",
                invalid: PlatformWalletError.invalidParameter
            ) { documentTypePtr in
                let ffi = DescriptionDocumentFFI(
                    id: idBp.baseAddress,
                    contract_id: contractBp.baseAddress,
                    document_type: documentTypePtr
                )
                return try withUnsafePointer(to: ffi) { try body($0) }
            }
        }
    }
}

extension MultipleChoiceProposalParams {
    /// Run `body` with these params as a `MultipleChoiceProposalParamsFFI`
    /// whose pointers stay valid for the call. A string holding a NUL or an
    /// id that is not 32 bytes is refused with
    /// `PlatformWalletError.invalidParameter` before `body` runs.
    func withFFI<R>(_ body: (UnsafePointer<MultipleChoiceProposalParamsFFI>) throws -> R) throws -> R {
        try withDecisionProposalCString(
            title, field: "title", invalid: PlatformWalletError.invalidParameter
        ) { titlePtr in
            try withDescriptionDocumentFFI(descriptionDocument) { documentPtr in
                try withDecisionProposalCString(
                    url, field: "url", invalid: PlatformWalletError.invalidParameter
                ) { urlPtr in
                    let ffi = MultipleChoiceProposalParamsFFI(
                        title: titlePtr,
                        description_document: documentPtr,
                        url: urlPtr,
                        open: open
                    )
                    return try withUnsafePointer(to: ffi) { try body($0) }
                }
            }
        }
    }
}

extension MultipleChoiceProposalEdit.Change {
    /// The change as the FFI writes it: the value to set (`nil` to keep or
    /// remove) beside the remove flag.
    var ffiParts: (value: Value?, remove: Bool) {
        switch self {
        case .keep: return (nil, false)
        case .set(let value): return (value, false)
        case .remove: return (nil, true)
        }
    }
}

extension MultipleChoiceProposalEdit {
    /// Run `body` with this edit as a `MultipleChoiceProposalEditFFI` whose
    /// pointers stay valid for the call. `MultipleChoiceProposalEditFFI`
    /// says "remove" with a flag beside a null value, so a `.set` never
    /// raises the flag and a `.remove` never carries a value. A string
    /// holding a NUL or an id that is not 32 bytes is refused with
    /// `PlatformWalletError.invalidParameter` before `body` runs.
    func withFFI<R>(_ body: (UnsafePointer<MultipleChoiceProposalEditFFI>) throws -> R) throws -> R {
        let documentChange = self.descriptionDocument.ffiParts
        let urlChange = self.url.ffiParts
        return try withDecisionProposalCString(
            title, field: "title", invalid: PlatformWalletError.invalidParameter
        ) { titlePtr in
            try withDescriptionDocumentFFI(documentChange.value) { documentPtr in
                try withDecisionProposalCString(
                    urlChange.value, field: "url", invalid: PlatformWalletError.invalidParameter
                ) { urlPtr in
                    let ffi = MultipleChoiceProposalEditFFI(
                        title: titlePtr,
                        description_document: documentPtr,
                        remove_description_document: documentChange.remove,
                        url: urlPtr,
                        remove_url: urlChange.remove
                    )
                    return try withUnsafePointer(to: ffi) { try body($0) }
                }
            }
        }
    }
}

extension ProposalChoiceParams {
    /// Run `body` with these params as a `ChoiceParamsFFI` whose pointers
    /// stay valid for the call. A string holding a NUL is refused with
    /// `PlatformWalletError.invalidParameter` before `body` runs.
    func withFFI<R>(_ body: (UnsafePointer<ChoiceParamsFFI>) throws -> R) throws -> R {
        try withDecisionProposalCString(
            title, field: "title", invalid: PlatformWalletError.invalidParameter
        ) { titlePtr in
            try withDecisionProposalCString(
                description, field: "description", invalid: PlatformWalletError.invalidParameter
            ) { descriptionPtr in
                let ffi = ChoiceParamsFFI(title: titlePtr, description: descriptionPtr)
                return try withUnsafePointer(to: ffi) { try body($0) }
            }
        }
    }
}
