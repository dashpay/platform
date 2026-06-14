import Foundation
import DashSDKFFI

/// Snapshot of a DPNS contest's current vote state.
///
/// Produced on-demand by
/// `ManagedPlatformWallet.fetchContestVoteState(identityId:label:)`.
/// Never cached — vote tallies, winner resolution, and the
/// contender set all change throughout the voting period, so
/// callers that need fresh details query this every time (e.g.
/// pull-to-refresh).
public struct ContestVoteState: Sendable, Equatable {
    /// The contested label (e.g. "alice").
    public let label: String
    /// Voting-period end time (wall clock).
    public let endTime: Date
    /// Contenders + their vote tallies. Sorted ascending by
    /// identity id as returned by the Rust side; callers that need
    /// vote-descending order should re-sort.
    public let contenders: [ContestContender]
    /// Abstain votes cast by masternodes.
    public let abstainVotes: UInt32
    /// Lock votes (contest → no winner, label becomes unavailable).
    public let lockVotes: UInt32
    /// Resolution status. `.none` while voting is ongoing.
    public let winner: ContestWinner

    /// Total vote count across contenders + abstain + lock.
    public var totalVotes: UInt32 {
        let contenderTotal = contenders.reduce(UInt32(0)) { $0 &+ $1.voteTally }
        return contenderTotal &+ abstainVotes &+ lockVotes
    }
}

/// One contender's summary within a `ContestVoteState`.
public struct ContestContender: Sendable, Equatable, Identifiable {
    public let identityId: Identifier
    public let voteTally: UInt32

    /// `Identifiable` conformance for SwiftUI `ForEach` use.
    public var id: Identifier { identityId }
}

/// Contest resolution status. Matches the Rust `ContestWinner` enum.
public enum ContestWinner: Sendable, Equatable {
    /// Contest is still in the voting period.
    case none
    /// Contest resolved — this identity won the name.
    case wonByIdentity(Identifier)
    /// Contest resolved as locked — nobody wins.
    case locked
}

/// A masternode's vote choice on a contested resource. Matches the Rust
/// `ResourceVoteChoice` enum and the `vote_choice` discriminant accepted by
/// `dash_sdk_contested_resource_cast_vote`:
/// `0` = TowardsIdentity, `1` = Abstain, `2` = Lock.
public enum ContestedResourceVoteChoice: Sendable, Equatable {
    /// Vote for a specific contender, identified by its base58 identity id.
    case towardsIdentity(String)
    /// Abstain — counts toward the abstain tally.
    case abstain
    /// Lock — vote that nobody should win the contested resource.
    case lock

    /// The `vote_choice` discriminant byte passed across the FFI. The
    /// `dash_sdk_contested_resource_cast_vote` parameter is a plain `u8`, so
    /// this is a `UInt8`. The values must agree with the
    /// `ContestedResourceVoteChoiceFFI` enum in
    /// `packages/rs-sdk-ffi/src/contested_resource/transitions/cast_vote.rs`:
    /// `TowardsIdentity = 0`, `Abstain = 1`, `Lock = 2`.
    var ffiTag: UInt8 {
        switch self {
        case .towardsIdentity: return 0
        case .abstain: return 1
        case .lock: return 2
        }
    }
}

// MARK: - FFI decoding

extension ContestVoteState {
    /// Copy a `ContestVoteStateFFI` into a Swift-owned value. The
    /// caller retains ownership of the FFI struct and is
    /// responsible for `contest_vote_state_ffi_free` afterward;
    /// this initializer only reads.
    init(ffi: ContestVoteStateFFI) {
        let label: String = {
            guard let ptr = ffi.label else { return "" }
            return String(cString: ptr)
        }()
        let end = Date(timeIntervalSince1970: Double(ffi.end_time_ms) / 1000)

        var contenders: [ContestContender] = []
        if let ptr = ffi.contenders_ptr, ffi.contenders_count > 0 {
            contenders.reserveCapacity(Int(ffi.contenders_count))
            for i in 0..<Int(ffi.contenders_count) {
                let c = ptr[i]
                var tuple = c.identity_id
                let identityId = Swift.withUnsafeBytes(of: &tuple) { Data($0) }
                contenders.append(
                    ContestContender(
                        identityId: identityId,
                        voteTally: c.vote_tally
                    )
                )
            }
        }

        let winner: ContestWinner = {
            switch ffi.winner_kind {
            case 1:
                var tuple = ffi.winner_identity_id
                let id = Swift.withUnsafeBytes(of: &tuple) { Data($0) }
                return .wonByIdentity(id)
            case 2:
                return .locked
            default:
                return .none
            }
        }()

        self.label = label
        self.endTime = end
        self.contenders = contenders
        self.abstainVotes = ffi.abstain_votes
        self.lockVotes = ffi.lock_votes
        self.winner = winner
    }
}
