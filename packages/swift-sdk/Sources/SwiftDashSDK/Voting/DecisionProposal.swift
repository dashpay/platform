import Foundation
import DashSDKFFI

// MARK: - Constants

/// The decision proposals system contract (protocol version 15), as the
/// document functions name it.
///
/// Read from `rs-sdk-ffi` (`dash_sdk_decision_proposal_contract_id`,
/// `dash_sdk_decision_proposal_contract_id_bytes`,
/// `dash_sdk_decision_proposal_document_type_name`) so no Swift copy of a
/// protocol constant exists.
public enum DecisionProposals {
    /// Base58 id of the decision proposals system contract.
    public static let contractId: String = {
        // A static string owned by the FFI: copied, never freed.
        String(cString: dash_sdk_decision_proposal_contract_id())
    }()

    /// The 32 bytes of the contract id, as the platform wallet's document
    /// functions take it.
    public static let contractIdentifier: Identifier = {
        var bytes = [UInt8](repeating: 0, count: 32)
        // Returns false only for a null pointer, which this buffer is not.
        _ = bytes.withUnsafeMutableBufferPointer { buffer in
            dash_sdk_decision_proposal_contract_id_bytes(buffer.baseAddress)
        }
        return Data(bytes)
    }()

    /// The proposal document type name.
    public static let documentTypeName: String = {
        // A static string owned by the FFI: copied, never freed.
        String(cString: dash_sdk_decision_proposal_document_type_name())
    }()
}

// MARK: - Proposal

/// A decision proposal: a yes/no question an identity puts to the
/// masternodes. Decoded from the JSON every decision proposal read returns
/// (`dash_sdk_decision_proposal_fetch` and the three listings).
public struct DecisionProposal: Decodable, Sendable, Equatable, Hashable, Identifiable {
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
    /// What a yes and a no mean.
    public let description: String
    /// Where the proposal is discussed; `nil` when it has none.
    public let url: String?
    /// The proposal passes when the yes voting power is strictly more than
    /// this percent of the yes plus no voting power.
    public let passPercent: UInt8
    /// The least yes plus no voting power the proposal needs, as a percent of
    /// the masternode list's total voting power when it closes, rounded up.
    public let minimumTurnoutPercent: UInt8
    /// When voting closes, in milliseconds since the Unix epoch.
    public let endDateMs: UInt64

    private enum CodingKeys: String, CodingKey {
        case id
        case ownerId
        case revision
        case createdAtMs = "createdAt"
        case updatedAtMs = "updatedAt"
        case title
        case description
        case url
        case passPercent
        case minimumTurnoutPercent
        case endDateMs = "endDate"
    }

    /// When voting closes.
    public var endDate: Date { Date(millisecondsSince1970: endDateMs) }
    /// When the proposal was created, if the document carries it.
    public var createdAt: Date? { createdAtMs.map(Date.init(millisecondsSince1970:)) }
    /// When the proposal was last edited, if the document carries it.
    public var updatedAt: Date? { updatedAtMs.map(Date.init(millisecondsSince1970:)) }
}

/// The question and rule of a new decision proposal, as
/// `ManagedPlatformWallet.createDecisionProposal` passes them on
/// (`DecisionProposalParamsFFI`). Platform judges every value when the
/// proposal is created; nothing is checked here.
public struct DecisionProposalParams: Sendable, Equatable {
    /// The question, in one line.
    public var title: String
    /// What a yes and a no mean.
    public var description: String
    /// Where the proposal is discussed; `nil` for none.
    public var url: String?
    /// The proposal passes when the yes voting power is strictly more than
    /// this percent of the yes plus no voting power.
    public var passPercent: UInt8
    /// The least yes plus no voting power the proposal needs, as a percent of
    /// the masternode list's total voting power when it closes.
    public var minimumTurnoutPercent: UInt8
    /// When voting closes, in milliseconds since the Unix epoch.
    public var endDateMs: UInt64

    public init(
        title: String,
        description: String,
        url: String? = nil,
        passPercent: UInt8,
        minimumTurnoutPercent: UInt8,
        endDateMs: UInt64
    ) {
        self.title = title
        self.description = description
        self.url = url
        self.passPercent = passPercent
        self.minimumTurnoutPercent = minimumTurnoutPercent
        self.endDateMs = endDateMs
    }

    /// The same, with the end date given as a `Date`. A date before the Unix
    /// epoch is written as 0, which Platform refuses.
    public init(
        title: String,
        description: String,
        url: String? = nil,
        passPercent: UInt8,
        minimumTurnoutPercent: UInt8,
        endDate: Date
    ) {
        self.init(
            title: title,
            description: description,
            url: url,
            passPercent: passPercent,
            minimumTurnoutPercent: minimumTurnoutPercent,
            endDateMs: endDate.millisecondsSince1970
        )
    }
}

/// New text for a stored decision proposal, as
/// `ManagedPlatformWallet.editDecisionProposal` passes it on
/// (`DecisionProposalEditFFI`). A field left `nil` (or `.keep`) keeps its
/// value. rs-sdk refuses an edit that changes nothing; Platform accepts an
/// edit only within its window after the proposal was created, and never a
/// change to the rule.
public struct DecisionProposalEdit: Sendable, Equatable {
    /// What happens to the proposal's url.
    public enum URLChange: Sendable, Equatable {
        /// Keep the url as it is (or keep having none).
        case keep
        /// Set the url.
        case set(String)
        /// Remove the url.
        case remove
    }

    /// The new title; `nil` keeps it.
    public var title: String?
    /// The new description; `nil` keeps it.
    public var description: String?
    /// The url change.
    public var url: URLChange

    public init(title: String? = nil, description: String? = nil, url: URLChange = .keep) {
        self.title = title
        self.description = description
        self.url = url
    }
}

// MARK: - Vote choice

/// A masternode's answer on a yes/no poll, as rs-dpp's
/// `YesNoAbstainVoteChoice`.
///
/// Its raw value is the name the masternode-answer JSON writes (`"yes"`,
/// `"no"`, `"abstain"`); ``ffiTag`` is the `vote_choice` byte
/// `dash_sdk_decision_proposal_cast_vote` takes.
public enum YesNoAbstainVoteChoice: String, Sendable, Equatable, CaseIterable, Identifiable, Decodable {
    case yes
    case no
    case abstain

    public var id: String { rawValue }

    /// The `vote_choice` byte passed across the FFI. The C parameter is a
    /// plain `uint8_t` with no enum in the header, so the three values are
    /// mirrored here and must agree with `YesNoVoteChoiceFFI` in
    /// `packages/rs-sdk-ffi/src/decision_proposal/transitions.rs`:
    /// `Yes = 0`, `No = 1`, `Abstain = 2`.
    var ffiTag: UInt8 {
        switch self {
        case .yes: return 0
        case .no: return 1
        case .abstain: return 2
        }
    }
}

/// A masternode's answer on a decision proposal
/// (`dash_sdk_decision_proposal_get_masternode_vote`).
public struct MasternodeYesNoVote: Decodable, Sendable, Equatable {
    /// The masternode's current answer.
    public let choice: YesNoAbstainVoteChoice
    /// How many times the masternode has voted on the poll, its first vote
    /// included.
    public let timesVoted: UInt16
}

// MARK: - Poll state

/// A block, as the poll state writes it.
public struct PlatformBlockInfo: Decodable, Sendable, Equatable {
    public let height: UInt64
    public let coreHeight: UInt32
    /// Block time, milliseconds since the Unix epoch.
    public let timeMs: UInt64
    /// The epoch index.
    public let epoch: UInt16

    /// Block time.
    public var time: Date { Date(millisecondsSince1970: timeMs) }
}

/// Which way a fraction of the total voting power rounds to a whole voting
/// power (rs-dpp `VotingPowerRounding`).
public enum VotingPowerRounding: String, Decodable, Sendable, Equatable {
    case down
    case up
    case downPlusOne
    case upMinusOne
}

/// The least yes plus no voting power a yes/no poll needs to pass (rs-dpp
/// `YesNoMinimumVotingPower`), tagged with `$type` in its JSON.
public enum YesNoMinimumVotingPower: Decodable, Sendable, Equatable {
    /// A fixed voting power.
    case absolute(votingPower: UInt64)
    /// A share of the total voting power of the masternode list in the block
    /// that closes the poll, rounded as given.
    case fractionOfTotal(numerator: UInt8, denominator: UInt8, rounding: VotingPowerRounding)

    private enum CodingKeys: String, CodingKey {
        case type = "$type"
        case votingPower
        case numerator
        case denominator
        case rounding
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)
        switch type {
        case "absolute":
            self = .absolute(votingPower: try container.decode(UInt64.self, forKey: .votingPower))
        case "fractionOfTotal":
            self = .fractionOfTotal(
                numerator: try container.decode(UInt8.self, forKey: .numerator),
                denominator: try container.decode(UInt8.self, forKey: .denominator),
                rounding: try container.decode(VotingPowerRounding.self, forKey: .rounding)
            )
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type,
                in: container,
                debugDescription: "Unknown minimum voting power type \(type)"
            )
        }
    }
}

/// What a yes/no poll decides (rs-dpp `YesNoVotePollPurpose`), tagged with
/// `$type` in its JSON.
public enum YesNoVotePollPurpose: Decodable, Sendable, Equatable {
    /// The poll a decision proposal opens; `documentId` is the proposal's id,
    /// base58.
    case decisionProposal(documentId: String)

    private enum CodingKeys: String, CodingKey {
        case type = "$type"
        case documentId
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)
        switch type {
        case "decisionProposal":
            self = .decisionProposal(documentId: try container.decode(String.self, forKey: .documentId))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type,
                in: container,
                debugDescription: "Unknown yes/no vote poll purpose \(type)"
            )
        }
    }
}

/// A yes/no poll as it was opened, rule included (rs-dpp `YesNoVotePoll`).
public struct YesNoVotePoll: Decodable, Sendable, Equatable {
    public let purpose: YesNoVotePollPurpose
    /// With ``supermajorityDenominator``, the share of yes plus no the yes
    /// voting power must strictly exceed.
    public let supermajorityNumerator: UInt8
    public let supermajorityDenominator: UInt8
    public let minimumVotingPower: YesNoMinimumVotingPower
}

/// The record a finished yes/no poll keeps (rs-dpp `YesNoVotePollResult`).
public struct YesNoVotePollResult: Decodable, Sendable, Equatable {
    /// Whether the poll passed.
    public let passed: Bool
    /// The yes voting power at the end.
    public let yesVotingPower: UInt64
    /// The no voting power at the end.
    public let noVotingPower: UInt64
    /// The abstaining voting power at the end.
    public let abstainVotingPower: UInt64
    /// The yes plus no voting power the poll needed, resolved in the block
    /// that closed it.
    public let requiredVotingPower: UInt64
    /// The block the poll opened in.
    public let startBlock: PlatformBlockInfo
    /// The block the poll was closed in.
    public let finalizationBlock: PlatformBlockInfo
}

/// The status and tallies of a yes/no poll
/// (`dash_sdk_decision_proposal_get_vote_poll_state`).
public struct YesNoVotePollState: Decodable, Sendable, Equatable {
    /// Whether the poll is open or has finished.
    public enum Status: Sendable, Equatable {
        /// Voting is open; the poll opened in `startBlock`.
        case started(startBlock: PlatformBlockInfo)
        /// The poll has finished; its record.
        case finished(YesNoVotePollResult)
    }

    /// The poll as it was opened, rule included.
    public let votePoll: YesNoVotePoll
    public let status: Status
    /// The yes voting power so far. 0 once the poll has finished, as its
    /// votes are removed; read ``Status/finished(_:)`` then.
    public let yesVotingPower: UInt64
    /// The no voting power so far; 0 once the poll has finished.
    public let noVotingPower: UInt64
    /// The abstaining voting power so far; 0 once the poll has finished.
    public let abstainVotingPower: UInt64

    /// The poll's record once it has finished, `nil` while it is open.
    public var result: YesNoVotePollResult? {
        guard case .finished(let result) = status else { return nil }
        return result
    }

    private enum CodingKeys: String, CodingKey {
        case votePoll
        case status
        case startBlock
        case result
        case yesVotingPower
        case noVotingPower
        case abstainVotingPower
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        votePoll = try container.decode(YesNoVotePoll.self, forKey: .votePoll)
        yesVotingPower = try container.decode(UInt64.self, forKey: .yesVotingPower)
        noVotingPower = try container.decode(UInt64.self, forKey: .noVotingPower)
        abstainVotingPower = try container.decode(UInt64.self, forKey: .abstainVotingPower)
        let status = try container.decode(String.self, forKey: .status)
        switch status {
        case "started":
            self.status = .started(
                startBlock: try container.decode(PlatformBlockInfo.self, forKey: .startBlock)
            )
        case "finished":
            self.status = .finished(try container.decode(YesNoVotePollResult.self, forKey: .result))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .status,
                in: container,
                debugDescription: "Unknown yes/no vote poll status \(status)"
            )
        }
    }
}

// MARK: - C strings

/// Run `body` with `string` as a NUL-terminated C string, or with `nil` when
/// `string` is `nil`. Every string a decision proposal function hands to C
/// (title, description, url, base58 ids) goes through here.
///
/// A Swift `String` can hold U+0000, and `withCString` keeps that byte, but
/// the Rust side reads each string with `CStr::from_ptr`, which stops at the
/// first NUL: it would act on a prefix of the string passed, such as a
/// shortened title or a different `startAfter` id. A string with an embedded
/// NUL has no C representation, so it is refused with `invalid` before any
/// pointer is made. This checks only that the string can cross the
/// boundary; what a proposal may say is Platform's to judge.
///
/// - Parameters:
///   - field: Names the string in the refusal.
///   - invalid: The invalid-parameter error of the calling API
///     (`SDKError.invalidParameter`, `PlatformWalletError.invalidParameter`).
func withDecisionProposalCString<R, E: Error>(
    _ string: String?,
    field: String,
    invalid: (String) -> E,
    _ body: (UnsafePointer<CChar>?) throws -> R
) throws -> R {
    guard let string else { return try body(nil) }
    guard !string.utf8.contains(0) else {
        throw invalid("\(field) contains a NUL character, which a C string cannot carry")
    }
    return try string.withCString { try body($0) }
}

// MARK: - JSON decoding

/// Decodes the JSON the decision proposal FFI functions return. Shared by
/// the wrappers and the hermetic decoder tests, so both read the same way.
enum DecisionProposalJSON {
    static func decode<T: Decodable>(_ type: T.Type, from json: String) throws -> T {
        do {
            return try JSONDecoder().decode(type, from: Data(json.utf8))
        } catch {
            throw SDKError.serializationError("Failed to decode \(T.self): \(error)")
        }
    }
}

// MARK: - Millisecond timestamps

extension Date {
    /// The date `milliseconds` after the Unix epoch.
    init(millisecondsSince1970 milliseconds: UInt64) {
        self.init(timeIntervalSince1970: TimeInterval(milliseconds) / 1000)
    }

    /// Whole milliseconds since the Unix epoch, rounded down; 0 for a date
    /// before the epoch.
    var millisecondsSince1970: UInt64 {
        let milliseconds = (timeIntervalSince1970 * 1000).rounded(.down)
        return UInt64(exactly: milliseconds) ?? 0
    }
}
