import Foundation

// MARK: - Contract moderation and fee pots (protocol version 14)
//
// The values the `dash_sdk_contract_moderation_*` and
// `dash_sdk_contract_get_fee_pots` reads of `rs-sdk-ffi` return, decoded from
// their JSON (written by `packages/rs-sdk-ffi/src/contract_moderation/json.rs`,
// whose field names follow the wasm bindings). Identifiers are base58
// strings, times are milliseconds since the Unix epoch, credits are whole
// credits. Nothing here decides anything about moderation: a field absent in
// the JSON is `nil` here.

// MARK: - Arguments

/// One of the moderation lists a moderated contract may keep.
///
/// Its raw value is the name the JSON writes (`"banlist"`, `"suspensions"`,
/// `"warnings"`); ``ffiTag`` is the byte the FFI takes.
public enum ContractModerationList: String, Codable, Sendable, Equatable, Hashable, CaseIterable, Identifiable {
    case banlist
    case suspensions
    case warnings

    public var id: String { rawValue }

    /// The list byte passed across the FFI. The C parameter is a plain
    /// `uint8_t` with no enum in the header, so the values are mirrored here
    /// and must agree with `ContractModerationListFFI` in
    /// `packages/rs-sdk-ffi/src/contract_moderation/mod.rs`:
    /// `Banlist = 0`, `Suspensions = 1`, `Warnings = 2`.
    var ffiTag: UInt8 {
        switch self {
        case .banlist: return 0
        case .suspensions: return 1
        case .warnings: return 2
        }
    }
}

/// A team action's status: active while its approvals fall short of its
/// rule, closed once they met it and the action ran.
public enum ContractTeamActionStatus: String, Codable, Sendable, Equatable, Hashable, CaseIterable, Identifiable {
    case active
    case closed

    public var id: String { rawValue }

    /// The status byte passed across the FFI; must agree with
    /// `ContractTeamActionStatusFFI` in
    /// `packages/rs-sdk-ffi/src/contract_moderation/mod.rs`:
    /// `Active = 0`, `Closed = 1`.
    var ffiTag: UInt8 {
        switch self {
        case .active: return 0
        case .closed: return 1
        }
    }
}

// MARK: - Reasons and warnings

/// A document a moderation reason is about.
public struct ContractModerationDocument: Codable, Sendable, Equatable, Hashable {
    /// The document type of the document, on the moderated contract.
    public let documentTypeName: String
    /// The document's id, base58.
    public let documentId: String
}

/// Why a moderator banned, suspended or warned an identity, or deleted a
/// document, as the moderator wrote it. Nothing checks what it says.
public struct ContractModerationReason: Codable, Sendable, Equatable, Hashable {
    /// Reserved for ban codes; `nil` when there is none.
    public let code: UInt16?
    /// Free text; may be empty.
    public let text: String
    /// The documents the reason is about; `nil` when it names none.
    public let documents: [ContractModerationDocument]?
    /// The `reason` document of the moderation charters contract, base58;
    /// `nil` when there is none.
    public let reasonDocumentId: String?
}

/// One warning an identity carries.
public struct ContractWarning: Codable, Sendable, Equatable, Hashable {
    /// The time of the block that issued it, milliseconds.
    public let warnedAtMs: UInt64
    /// Why, as the moderator wrote it.
    public let reason: ContractModerationReason

    private enum CodingKeys: String, CodingKey {
        case warnedAtMs = "warnedAt"
        case reason
    }

    /// When it was issued.
    public var warnedAt: Date { Date(millisecondsSince1970: warnedAtMs) }
}

// MARK: - Status

/// One identity's status on the lists of a moderated contract that were read
/// (`dash_sdk_contract_moderation_get_status`). A list that was not read says
/// nothing: its fields are `nil`, which is not "not banned".
public struct ContractModerationStatus: Codable, Sendable, Equatable {
    /// The lists this status covers.
    public let lists: [ContractModerationList]
    /// Set when ``lists`` includes the banlist.
    public let banned: Bool?
    /// Why the identity is banned, when it is.
    public let banReason: ContractModerationReason?
    /// When ``lists`` includes the suspension list and the identity is
    /// suspended: the block time until which it is, milliseconds. A lapsed
    /// suspension stays until it is swept.
    public let suspendedUntilMs: UInt64?
    /// Why the identity is suspended, when it is.
    public let suspensionReason: ContractModerationReason?
    /// When ``lists`` includes the warning list: the identity's warnings,
    /// oldest first, empty when it carries none.
    public let warnings: [ContractWarning]?

    private enum CodingKeys: String, CodingKey {
        case lists
        case banned
        case banReason
        case suspendedUntilMs = "suspendedUntil"
        case suspensionReason
        case warnings
    }

    /// The end of the suspension, when there is one.
    public var suspendedUntil: Date? { suspendedUntilMs.map(Date.init(millisecondsSince1970:)) }
}

// MARK: - List entries

/// One entry of a moderation list.
public struct ContractModerationEntry: Codable, Sendable, Equatable, Hashable, Identifiable {
    /// The identity on the list, base58.
    public let identityId: String
    /// For a suspension list entry: when it lapses, milliseconds.
    public let untilMs: UInt64?
    /// Why the identity is on the list: the ban's reason, the suspension's,
    /// or for a warning list entry the latest warning's.
    public let reason: ContractModerationReason
    /// For a warning list entry: every warning, oldest first.
    public let warnings: [ContractWarning]?

    private enum CodingKeys: String, CodingKey {
        case identityId
        case untilMs = "until"
        case reason
        case warnings
    }

    public var id: String { identityId }

    /// When a suspension lapses.
    public var until: Date? { untilMs.map(Date.init(millisecondsSince1970:)) }
}

/// One page of a moderation list, in identity id order
/// (`dash_sdk_contract_moderation_get_entries`).
public struct ContractModerationEntriesPage: Codable, Sendable, Equatable {
    public let entries: [ContractModerationEntry]
    /// The cursor of the next page, to pass back as `startAfter`; `nil` on
    /// the last page.
    public let nextStartAfter: String?
}

// MARK: - Document removals

/// A value a removal record keeps of its document, in the tagged form the
/// FFI writes and `Codable` reads and writes: `{"type": ..., "value": ...}`.
/// The tag carries the platform value's kind, so nothing is inferred from a
/// JSON number: a signed and an unsigned integer of the same magnitude stay
/// apart, and `1.0` and `-0.0` stay doubles, sign included.
///
/// The types: `null` (no `value`), `bool`, `int` (a signed integer up to 64
/// bits), `uint` (an unsigned one), `int128` and `uint128` (a 128-bit value
/// outside that range, as a decimal string), `double`, `string`,
/// `identifier` (base58), `bytes` (base64), `array` and `object`.
public indirect enum ContractKeptFieldValue: Codable, Sendable, Equatable, Hashable {
    case null
    case bool(Bool)
    case int(Int64)
    case uint(UInt64)
    /// A signed 128-bit value outside `Int64`, in decimal.
    case int128(String)
    /// An unsigned 128-bit value outside `UInt64`, in decimal.
    case uint128(String)
    case double(Double)
    case string(String)
    /// An identifier, base58.
    case identifier(String)
    /// Bytes, base64.
    case bytes(String)
    case array([ContractKeptFieldValue])
    case object([String: ContractKeptFieldValue])

    private enum CodingKeys: String, CodingKey {
        case type
        case value
    }

    /// Reads the tagged form ``encode(to:)`` writes.
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(Swift.String.self, forKey: .type)
        switch type {
        case "null": self = .null
        case "bool": self = .bool(try container.decode(Bool.self, forKey: .value))
        case "int": self = .int(try container.decode(Int64.self, forKey: .value))
        case "uint": self = .uint(try container.decode(UInt64.self, forKey: .value))
        case "int128": self = .int128(try container.decode(Swift.String.self, forKey: .value))
        case "uint128": self = .uint128(try container.decode(Swift.String.self, forKey: .value))
        case "identifier": self = .identifier(try container.decode(Swift.String.self, forKey: .value))
        case "bytes": self = .bytes(try container.decode(Swift.String.self, forKey: .value))
        case "double": self = .double(try container.decode(Double.self, forKey: .value))
        case "string": self = .string(try container.decode(Swift.String.self, forKey: .value))
        case "array": self = .array(try container.decode([ContractKeptFieldValue].self, forKey: .value))
        case "object":
            self = .object(try container.decode([Swift.String: ContractKeptFieldValue].self, forKey: .value))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type,
                in: container,
                debugDescription: "Unknown kept field value type \(type)"
            )
        }
    }

    /// Writes the tagged form: `{"type": ..., "value": ...}`, no `value` for
    /// `null`.
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .null:
            try container.encode("null", forKey: .type)
        case .bool(let value):
            try container.encode("bool", forKey: .type)
            try container.encode(value, forKey: .value)
        case .int(let value):
            try container.encode("int", forKey: .type)
            try container.encode(value, forKey: .value)
        case .uint(let value):
            try container.encode("uint", forKey: .type)
            try container.encode(value, forKey: .value)
        case .int128(let value):
            try container.encode("int128", forKey: .type)
            try container.encode(value, forKey: .value)
        case .uint128(let value):
            try container.encode("uint128", forKey: .type)
            try container.encode(value, forKey: .value)
        case .identifier(let value):
            try container.encode("identifier", forKey: .type)
            try container.encode(value, forKey: .value)
        case .bytes(let value):
            try container.encode("bytes", forKey: .type)
            try container.encode(value, forKey: .value)
        case .double(let value):
            try container.encode("double", forKey: .type)
            try container.encode(value, forKey: .value)
        case .string(let value):
            try container.encode("string", forKey: .type)
            try container.encode(value, forKey: .value)
        case .array(let value):
            try container.encode("array", forKey: .type)
            try container.encode(value, forKey: .value)
        case .object(let value):
            try container.encode("object", forKey: .type)
            try container.encode(value, forKey: .value)
        }
    }
}

/// The record a contract keeps of one document a moderator deleted.
public struct ContractDocumentRemovalEntry: Codable, Sendable, Equatable, Hashable, Identifiable {
    /// The removed document's id, base58.
    public let documentId: String
    /// The identity that owned it, base58.
    public let documentOwnerId: String
    /// The contract owner or moderator that removed it, base58.
    public let moderatorId: String
    /// Why, as the moderator wrote it.
    public let reason: ContractModerationReason
    /// The time of the block that removed it, milliseconds.
    public let removedAtMs: UInt64
    /// A double SHA-256 of the document as it was, 64 hex characters.
    public let documentHash: String
    /// The moderator that restored the document, base58; `nil` while the
    /// removal stands.
    public let restoredBy: String?
    /// The time of the block that restored it, milliseconds; `nil` while the
    /// removal stands.
    public let restoredAtMs: UInt64?
    /// The values the record keeps of the document, by property path; empty
    /// when its type keeps none.
    public let keptFields: [String: ContractKeptFieldValue]

    private enum CodingKeys: String, CodingKey {
        case documentId
        case documentOwnerId
        case moderatorId
        case reason
        case removedAtMs = "removedAt"
        case documentHash
        case restoredBy
        case restoredAtMs = "restoredAt"
        case keptFields
    }

    public var id: String { documentId }

    /// When the document was removed.
    public var removedAt: Date { Date(millisecondsSince1970: removedAtMs) }
    /// When the document was restored, if it was.
    public var restoredAt: Date? { restoredAtMs.map(Date.init(millisecondsSince1970:)) }
}

/// Removal records of one document type, in document id order
/// (`dash_sdk_contract_moderation_get_document_removals` and
/// `dash_sdk_contract_moderation_get_document_removals_by_ids`).
public struct ContractDocumentRemovalsPage: Codable, Sendable, Equatable {
    public let removals: [ContractDocumentRemovalEntry]
    /// The cursor of the next page, to pass back as `startAfter`; `nil` on
    /// the last page and for a read by ids.
    public let nextStartAfter: String?
}

// MARK: - Team actions

/// The deletion of a settled document, which the seated team decides
/// together.
public struct ContractTeamActionDeleteSettledDocument: Codable, Sendable, Equatable, Hashable {
    /// The document's type.
    public let documentTypeName: String
    /// The document, base58.
    public let documentId: String
    /// The document's last modification when proposed, milliseconds.
    public let documentLastModifiedAtMs: UInt64
    /// The document's revision when proposed; `nil` on a type whose
    /// documents carry none.
    public let documentRevision: UInt64?
    /// Why, as the proposer gave it.
    public let reason: ContractModerationReason

    private enum CodingKeys: String, CodingKey {
        case documentTypeName
        case documentId
        case documentLastModifiedAtMs = "documentLastModifiedAt"
        case documentRevision
        case reason
    }
}

/// What a team action does once approved, tagged with `type` in its JSON.
public enum ContractTeamActionEvent: Codable, Sendable, Equatable, Hashable {
    case deleteSettledDocument(ContractTeamActionDeleteSettledDocument)

    private enum CodingKeys: String, CodingKey {
        case type
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)
        switch type {
        case "deleteSettledDocument":
            self = .deleteSettledDocument(try ContractTeamActionDeleteSettledDocument(from: decoder))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type,
                in: container,
                debugDescription: "Unknown team action event type \(type)"
            )
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .deleteSettledDocument(let event):
            try container.encode("deleteSettledDocument", forKey: .type)
            try event.encode(to: encoder)
        }
    }
}

/// One action an elected contract's seated moderation team votes on.
public struct ContractTeamActionEntry: Codable, Sendable, Equatable, Hashable, Identifiable {
    /// The action's id, base58: what the other members approve it by.
    public let actionId: String
    /// The member that proposed it, base58.
    public let proposerId: String
    /// The time of the block of the proposal, milliseconds.
    public let proposedAtMs: UInt64
    /// What it does once approved.
    public let event: ContractTeamActionEvent
    /// How many approvals it holds, the proposer's among them; an active
    /// action's is an upper bound.
    public let approvalCount: UInt32

    private enum CodingKeys: String, CodingKey {
        case actionId
        case proposerId
        case proposedAtMs = "proposedAt"
        case event
        case approvalCount
    }

    public var id: String { actionId }

    /// When it was proposed.
    public var proposedAt: Date { Date(millisecondsSince1970: proposedAtMs) }
}

/// One page of team actions, in action id order
/// (`dash_sdk_contract_moderation_get_team_actions`).
public struct ContractTeamActionsPage: Codable, Sendable, Equatable {
    public let actions: [ContractTeamActionEntry]
    /// The cursor of the next page, to pass back as `startAtActionId`;
    /// `nil` on the last page.
    public let nextStartAtActionId: String?
    /// Whether the next page includes the ``nextStartAtActionId`` action, to
    /// pass back as `startAtActionIdIncluded`; set with
    /// ``nextStartAtActionId``.
    public let nextStartAtActionIdIncluded: Bool?
}

/// The members that approved a team action, in identity id order
/// (`dash_sdk_contract_moderation_get_team_action_signers`); empty when the
/// contract holds no action of that id with the status asked.
public struct ContractTeamActionSigners: Codable, Sendable, Equatable {
    public let signerIds: [String]
}

// MARK: - Action counts

/// How many counted moderation actions one member signed since the
/// moderators pot was last claimed or settled.
public struct ContractModerationActionCount: Codable, Sendable, Equatable, Hashable, Identifiable {
    /// The member, base58.
    public let identityId: String
    /// Its bans, suspensions, warnings and document deletions.
    public let count: UInt32

    public var id: String { identityId }
}

/// The moderation action counts of an elected contract's seated team since
/// the moderators pot was last claimed or settled, in identity id order
/// (`dash_sdk_contract_moderation_get_action_counts`).
public struct ContractModerationActionCounts: Codable, Sendable, Equatable {
    public let counts: [ContractModerationActionCount]
}

// MARK: - Fee pots

/// What one fee pot of a contract holds.
public struct ContractFeePotState: Codable, Sendable, Equatable {
    /// The credits in the pot: its balance, not a promise of what a claim
    /// will distribute.
    public let credits: UInt64
    /// The epoch of the pot's last explicit claim; `nil` when it was never
    /// claimed. Not the last payout: a membership change of an elected team
    /// can also pay out the moderators pot (a moderators pot settlement),
    /// which leaves these fields as they were.
    public let lastClaimEpoch: UInt16?
    /// The time of the block of the last explicit claim, milliseconds; set
    /// with ``lastClaimEpoch``.
    public let lastClaimTimeMs: UInt64?
    /// The identity that signed the last claim, base58; set with
    /// ``lastClaimEpoch``.
    public let lastClaimantId: String?

    /// When the pot was last claimed, if it ever was.
    public var lastClaimTime: Date? { lastClaimTimeMs.map(Date.init(millisecondsSince1970:)) }
}

/// The fee pots of a contract (`dash_sdk_contract_get_fee_pots`).
public struct ContractFeePots: Codable, Sendable, Equatable {
    /// The pot the contract owner claims.
    public let owner: ContractFeePotState
    /// The pot the contract's moderation team shares.
    public let moderators: ContractFeePotState
}
