import Foundation

/// Constants identifying the DPNS contested-username vote poll.
///
/// A contested-resource vote poll is addressed by
/// `(contract_id, document_type_name, index_name, index_values)`. For DPNS
/// username contests every component except the label is fixed, so the read
/// queries and
/// ``SDK/castContestedResourceVote(dataContractId:documentTypeName:indexName:indexValues:choice:proTxHash:votingPrivateKey:)``
/// share one definition instead of each caller re-spelling the strings.
public enum DPNSVotePoll {
  /// Base58 id of the DPNS system data contract (`dpns_contract::ID_BYTES`).
  public static let contractId = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
  /// The contested document type.
  public static let documentTypeName = "domain"
  /// The contested index on `domain`
  /// (`normalizedParentDomainName` + `normalizedLabel`).
  public static let indexName = "parentNameAndLabel"
  /// First index value — the parent domain every DPNS username sits under.
  public static let parentDomain = "dash"

  /// Index values addressing one label's poll: `["dash", <normalized label>]`.
  ///
  /// - Important: `normalizedLabel` must already be homograph-normalized
  ///   (`alice` → `a11ce`). Platform indexes polls under the normalized form
  ///   only, so passing a raw label silently addresses a poll that does not
  ///   exist. Use ``SDK/dpnsNormalizeLabel(_:)``.
  public static func indexValues(normalizedLabel: String) -> [String] {
    [parentDomain, normalizedLabel]
  }
}

/// One open DPNS username contest, as returned by
/// ``SDK/dpnsActiveContests(limit:)``.
public struct DPNSContest: Sendable, Identifiable, Equatable {
  /// The contested label in its normalized (homograph-safe) form — this is
  /// the form Platform indexes, and the form to pass back when voting.
  public let normalizedLabel: String
  /// End of the voting period, or `nil` when the FFI reported no end time.
  /// Modeled as optional rather than defaulted so callers can render
  /// "unknown" instead of a fabricated deadline.
  public let endTime: Date?
  /// Whether Platform has already resolved this contest. Active listings
  /// normally report `false`; a `true` here means the contest resolved
  /// between the poll list and the tally read.
  public let hasWinner: Bool
  /// Masternode votes cast to abstain.
  public let abstainVotes: UInt32
  /// Masternode votes cast to lock the name (nobody wins).
  public let lockVotes: UInt32
  /// Contenders and their tallies, in the order the FFI returned them.
  public let contenders: [DPNSContender]
  /// The distinct spellings the contenders requested, in contender order —
  /// `["pizza", "p1zza"]` for a contest normalized to `"p1zza"`.
  ///
  /// Ordered and de-duplicated **in Rust** and decoded here verbatim: which
  /// spellings to show is display policy, and the SDK is a thin bridge, not a
  /// place for policy loops.
  ///
  /// Empty when no contender document could be decoded; render
  /// ``normalizedLabel`` in that case rather than inventing a spelling.
  public let requestedLabels: [String]

  public var id: String { normalizedLabel }

  public init(
    normalizedLabel: String,
    endTime: Date?,
    hasWinner: Bool,
    abstainVotes: UInt32,
    lockVotes: UInt32,
    contenders: [DPNSContender],
    requestedLabels: [String] = []
  ) {
    self.normalizedLabel = normalizedLabel
    self.endTime = endTime
    self.hasWinner = hasWinner
    self.abstainVotes = abstainVotes
    self.lockVotes = lockVotes
    self.contenders = contenders
    self.requestedLabels = requestedLabels
  }

  /// Total masternode voting weight recorded so far across contenders,
  /// abstain and lock. Tallies are weighted by node type on the Platform
  /// side (regular masternode 1, evonode 4), so this is a weight, not a
  /// count of nodes.
  public var totalVotes: UInt32 {
    contenders.reduce(UInt32(0)) { $0 &+ $1.voteTally } &+ abstainVotes &+ lockVotes
  }

  /// The contender with the highest tally, or `nil` when there are none.
  /// Ties resolve to the first in FFI order — callers that care about tie
  /// display should inspect ``contenders`` directly.
  public var leadingContender: DPNSContender? {
    contenders.max { $0.voteTally < $1.voteTally }
  }
}

/// One contender for a contested DPNS label.
public struct DPNSContender: Sendable, Identifiable, Equatable {
  /// Base58 identity id of the contender.
  public let identityId: String
  /// Masternode voting weight cast towards this contender.
  public let voteTally: UInt32
  /// The label exactly as this contender typed it — `"pizza"`, where the
  /// contest's normalized form is `"p1zza"`.
  ///
  /// Decoded by the FFI from the contender's `domain` document. `nil` when the
  /// label data was unavailable or decoding that contender's document failed —
  /// show ``identityId`` or the contest's normalized label rather than
  /// inventing a spelling.
  public let displayLabel: String?

  public var id: String { identityId }

  public init(identityId: String, voteTally: UInt32, displayLabel: String? = nil) {
    self.identityId = identityId
    self.voteTally = voteTally
    self.displayLabel = displayLabel
  }
}

/// How a contest stands, as reported by the vote-state query.
///
/// Platform attaches winner info to a contest **only once the poll has
/// finished** — `Contenders.winner` is `Option<(winner_info,
/// finished_at_block_info)>`, and drive-abci wraps every variant, `NoWinner`
/// included, in a `FinishedVoteInfo`. So the presence of winner info, not its
/// value, is what separates a live contest from a settled one.
public enum DPNSContestOutcome: Sendable, Equatable {
  /// Voting is still open — Platform reported no winner info at all.
  case ongoing
  /// The poll **finished** without awarding the name to anyone and without
  /// being locked (drive-abci's `FinishedVoteOutcome::NoPreviousWinner`).
  /// This is a settled contest, not a running one.
  case noWinner
  /// The label was awarded to this base58 identity id.
  case wonBy(String)
  /// Masternodes locked the label; nobody wins it.
  case locked
}

/// Full vote state for a single contest, from
/// ``SDK/dpnsContestVoteState(normalizedLabel:limit:)``.
///
/// Unlike ``DPNSContest`` this carries the resolution outcome, at the cost of
/// one query per contest.
public struct DPNSContestVoteState: Sendable, Equatable {
  /// The normalized label this state describes.
  public let normalizedLabel: String
  public let contenders: [DPNSContender]
  public let abstainVotes: UInt32
  public let lockVotes: UInt32
  public let outcome: DPNSContestOutcome
  /// Block time at which the contest resolved. Present only alongside a
  /// resolved ``outcome``.
  public let resolvedAt: Date?

  public init(
    normalizedLabel: String,
    contenders: [DPNSContender],
    abstainVotes: UInt32,
    lockVotes: UInt32,
    outcome: DPNSContestOutcome,
    resolvedAt: Date?
  ) {
    self.normalizedLabel = normalizedLabel
    self.contenders = contenders
    self.abstainVotes = abstainVotes
    self.lockVotes = lockVotes
    self.outcome = outcome
    self.resolvedAt = resolvedAt
  }

  /// `true` once the poll has finished, whatever the outcome.
  ///
  /// ``DPNSContestOutcome/noWinner`` counts as resolved: Platform only emits
  /// winner info for a finished poll, so treating it as unresolved would let
  /// ``SDK/dpnsContestIsOpen(normalizedLabel:)`` report a settled contest as
  /// still open and send a vote into a poll that can no longer accept it.
  public var isResolved: Bool {
    switch outcome {
    case .ongoing: return false
    case .noWinner, .wonBy, .locked: return true
    }
  }
}
