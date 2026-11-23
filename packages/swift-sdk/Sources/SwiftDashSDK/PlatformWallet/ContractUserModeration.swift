import Foundation

// MARK: - Contract moderation writes (protocol version 14)
//
// The shapes the `platform_wallet_contract_moderation_offer`,
// `platform_wallet_contract_user_moderate`,
// `platform_wallet_contract_document_*` and
// `platform_wallet_contract_team_action_approve` functions write
// (`packages/rs-platform-wallet-ffi/src/contract_user_moderation.rs`):
// identifiers as base58 strings, times in milliseconds. A target's status and
// a removal record reuse the read types (`ContractModerationStatus`,
// `ContractDocumentRemovalEntry`), as the FFI writes them with the reads' own
// writers. Rust decides every value here; these types only decode them.

/// One of the six moderations of an identity on a contract's lists.
public enum ContractUserModerationAction: String, Codable, Sendable, CaseIterable, Identifiable, Hashable {
    case ban
    case unban
    case suspend
    case unsuspend
    case warn
    case clearWarnings

    public var id: String { rawValue }

    /// The `action` byte `platform_wallet_contract_user_moderate` takes; must
    /// agree with `CONTRACT_USER_MODERATION_BAN` to
    /// `CONTRACT_USER_MODERATION_CLEAR_WARNINGS` in the FFI. Any other byte is
    /// refused there.
    public var ffiTag: UInt8 {
        switch self {
        case .ban: return 0
        case .unban: return 1
        case .suspend: return 2
        case .unsuspend: return 3
        case .warn: return 4
        case .clearWarnings: return 5
        }
    }
}

/// How an identity moderates a contract.
public enum ContractModeratorRole: String, Codable, Sendable, Hashable {
    /// The contract owner.
    case owner
    /// A moderator the contract appoints beside its owner.
    case appointedModerator
    /// A moderator the interim of an elected contract appoints, until a team
    /// is seated.
    case interimModerator
    /// The leader of an elected contract's seated team.
    case teamLeader
    /// An active member of an elected contract's seated team.
    case teamMember
}

/// Who must approve the deletion of a settled document of a type
/// (`moderatorAbilities.deleteSettled`).
public struct ContractSettledDeletionRule: Codable, Sendable, Equatable, Hashable {
    /// How many members of the seated team must approve, the leader counted.
    public let approvals: UInt16
    /// Whether the leader must be among them.
    public let leader: Bool
    /// Whether a member the leader added counts only for documents created
    /// after its addition.
    public let approversPredateDocument: Bool
}

/// What a moderator may do to the documents of one document type, as Rust
/// read it from the contract.
public struct ContractDocumentTypeModerationOffer: Codable, Sendable, Equatable, Hashable, Identifiable {
    public let documentTypeName: String
    /// Delete one of its documents alone, within the window when there is
    /// one.
    public let delete: Bool
    /// The window in seconds after a document's last modification within
    /// which a moderator deletes it alone; `nil` when there is no limit.
    public let deleteWithinSeconds: UInt32?
    /// Restore one of its documents a moderator deleted.
    public let restore: Bool
    /// The fields only moderators write that this moderator may change.
    public let changeableFields: [String]
    /// For a seated team's member, the rule a proposed deletion of a settled
    /// document must meet; `nil` when it may propose none.
    public let settledDeletion: ContractSettledDeletionRule?

    public var id: String { documentTypeName }
}

/// What one identity of a wallet may do on a contract as its moderator.
public struct ContractIdentityModerationOffer: Codable, Sendable, Equatable, Hashable, Identifiable {
    /// The identity, base58.
    public let identityId: String
    public let role: ContractModeratorRole
    /// The moderations of an identity it may send.
    public let userActions: [ContractUserModerationAction]
    /// What it may do on each document type, only the types on which it may
    /// do something.
    public let documentTypes: [ContractDocumentTypeModerationOffer]
    /// Whether it may approve team actions other members proposed.
    public let approveTeamActions: Bool

    public var id: String { identityId }

    /// What it may do on `documentTypeName`, `nil` when nothing.
    public func documentType(named documentTypeName: String) -> ContractDocumentTypeModerationOffer? {
        documentTypes.first { $0.documentTypeName == documentTypeName }
    }
}

/// Which identities of one wallet may moderate a contract and what each may
/// do (`ManagedPlatformWallet.contractModerationOffer(contractId:)`), as Rust
/// read the contract and, for an elected contract, its seated team.
///
/// It names who may sign what. Whether a given target, document or team
/// action admits the moderation is Platform's check when it is sent, refused
/// with its consensus code.
public struct ContractModerationOffer: Codable, Sendable, Equatable {
    /// The contract asked about, base58.
    public let contractId: String
    /// Who moderates the contract.
    public let moderatorsTeam: ContractModeratorsPotTeam
    /// The wallet's identities that moderate it, in identity id order.
    public let identities: [ContractIdentityModerationOffer]

    /// Decode the JSON `platform_wallet_contract_moderation_offer` writes.
    ///
    /// - Throws: `PlatformWalletError.deserialization` when `json` is not
    ///   that shape.
    public static func decode(fromJSON json: String) throws -> ContractModerationOffer {
        try ContractFeeClaimJSON.decode(ContractModerationOffer.self, json, what: "contract moderation offer")
    }
}

/// Why a moderator acts: free text and, for a seated team, a `reason`
/// document of the moderation charters contract its proposal lists.
public struct ContractModerationReasonInput: Sendable, Equatable {
    public var text: String
    /// The reason document's 32-byte id, or `nil`.
    public var reasonDocumentId: Identifier?

    public init(text: String = "", reasonDocumentId: Identifier? = nil) {
        self.text = text
        self.reasonDocumentId = reasonDocumentId
    }
}

/// A target's status after a moderation, as its proof shows it
/// (`ManagedPlatformWallet.moderateContractUser`). A ban proves every barring
/// list the contract keeps; the other moderations prove the one list they
/// edit, so ``status`` says nothing of the others.
public struct ModeratedContractUser: Codable, Sendable, Equatable {
    public let contractId: String
    public let identityId: String
    /// In the shape the status read answers with.
    public let status: ContractModerationStatus

    /// Decode the JSON `platform_wallet_contract_user_moderate` writes.
    public static func decode(fromJSON json: String) throws -> ModeratedContractUser {
        try ContractFeeClaimJSON.decode(ModeratedContractUser.self, json, what: "contract user moderation")
    }
}

/// What a moderator's deletion left (`ManagedPlatformWallet.deleteContractDocument`).
public struct ContractDocumentDeletion: Codable, Sendable, Equatable {
    public let contractId: String
    public let documentTypeName: String
    public let documentId: String
    /// The removal record, as the removals read writes one; `nil` on a type
    /// whose deletions keep no record.
    public let removal: ContractDocumentRemovalEntry?
    /// The document as it was just before the deletion, serialized under its
    /// type, base64: what a restore takes back. The wallet always writes it
    /// (a document it cannot read is refused before the deletion is sent);
    /// optional so JSON without it still decodes.
    public let documentBytes: String?

    /// ``documentBytes`` decoded, `nil` when absent or not base64.
    public var documentData: Data? { documentBytes.flatMap { Data(base64Encoded: $0) } }

    /// Decode the JSON `platform_wallet_contract_document_delete` writes.
    public static func decode(fromJSON json: String) throws -> ContractDocumentDeletion {
        try ContractFeeClaimJSON.decode(ContractDocumentDeletion.self, json, what: "contract document deletion")
    }
}

/// The removal record a restore marked
/// (`ManagedPlatformWallet.restoreContractDocument`).
public struct ContractDocumentRestore: Codable, Sendable, Equatable {
    public let contractId: String
    public let documentTypeName: String
    public let documentId: String
    /// The record, now marked restored.
    public let removal: ContractDocumentRemovalEntry

    /// Decode the JSON `platform_wallet_contract_document_restore` writes.
    public static func decode(fromJSON json: String) throws -> ContractDocumentRestore {
        try ContractFeeClaimJSON.decode(ContractDocumentRestore.self, json, what: "contract document restore")
    }
}

/// A team action after a proposal or an approval, as the proof shows it.
public struct ContractTeamActionSignature: Codable, Sendable, Equatable {
    public let contractId: String
    /// The action's id, base58: what the other members approve it by.
    public let actionId: String
    /// `active` while the approvals fall short of the rule, `closed` once
    /// the action ran.
    public let status: ContractTeamActionStatus

    /// Decode the JSON `platform_wallet_contract_document_propose_settled_deletion`
    /// and `platform_wallet_contract_team_action_approve` write.
    public static func decode(fromJSON json: String) throws -> ContractTeamActionSignature {
        try ContractFeeClaimJSON.decode(ContractTeamActionSignature.self, json, what: "contract team action signature")
    }
}

/// The document a moderator's field change left, as its proof shows it
/// (`ManagedPlatformWallet.changeContractDocumentFields`).
public struct ContractDocumentFieldsChange: Sendable, Equatable {
    public let contractId: String
    public let documentTypeName: String
    /// The document's id (`$id`), base58.
    public let documentId: String
    /// Its revision now (`$revision`), `nil` on a type that keeps none.
    public let revision: UInt64?
    /// The document as it now stands, in the canonical JSON a document
    /// create returns.
    public let documentJSON: String

    /// Decode the JSON `platform_wallet_contract_document_change_fields`
    /// writes.
    ///
    /// - Throws: `PlatformWalletError.deserialization` when `json` is not
    ///   that shape.
    public static func decode(fromJSON json: String) throws -> ContractDocumentFieldsChange {
        guard let object = try? JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any],
              let contractId = object["contractId"] as? String,
              let documentTypeName = object["documentTypeName"] as? String,
              let document = object["document"] as? [String: Any],
              let documentId = document["$id"] as? String,
              let documentData = try? JSONSerialization.data(
                  withJSONObject: document,
                  options: [.sortedKeys]
              ),
              let documentJSON = String(data: documentData, encoding: .utf8) else {
            throw PlatformWalletError.deserialization("Failed to decode the contract document field change")
        }
        let revision = (document["$revision"] as? NSNumber)?.uint64Value
        return ContractDocumentFieldsChange(
            contractId: contractId,
            documentTypeName: documentTypeName,
            documentId: documentId,
            revision: revision,
            documentJSON: documentJSON
        )
    }
}
