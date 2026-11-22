import Foundation

// The answers of the block and transition status reads (protocol version 15), decoded one to
// one from the JSON the rs-sdk-ffi functions write. Rust decides what is proved; these types
// only carry its answer. Keys arrive in snake_case and are read with
// `JSONDecoder.KeyDecodingStrategy.convertFromSnakeCase`.

/// A state transition a block holds, with the result of its execution.
public struct PlatformBlockTransaction: Codable, Equatable, Sendable {
    /// The SHA-256 of the serialized transition, hex: the hash Tenderdash knows it by.
    public let hash: String
    /// 0 if it executed, otherwise the code of the error it paid for.
    public let code: UInt32
    public let gasUsed: Int64
    /// The data of its execution result, hex.
    public let resultData: String
    /// The transition type, e.g. `IdentityCreditTransfer`.
    public let type: String
    /// The identity the transition reports as its own, base58; nil when it has none.
    public let ownerId: String?

    /// Whether the transition executed (`code` 0).
    public var executed: Bool { code == 0 }
}

/// A block of the Platform chain with every state transition it holds, in block order.
public struct PlatformBlock: Codable, Equatable, Sendable {
    public let height: UInt64
    /// The block hash, hex.
    public let hash: String
    /// The block time, in milliseconds since the Unix epoch.
    public let timeMs: UInt64
    public let coreChainLockedHeight: UInt32
    /// Platform's state root after this block, hex.
    public let appHash: String
    /// The masternode that proposed the block, hex as the block carries it.
    public let proposerProTxHash: String
    public let transactions: [PlatformBlockTransaction]
}

/// Consecutive blocks from a start height (`dash_sdk_system_get_blocks`).
///
/// When `proved`, each block is the one a validator quorum committed at its height. There can
/// be fewer blocks than asked for, and nothing proves a block left off the end does not exist.
public struct PlatformBlocks: Codable, Equatable, Sendable {
    public let proved: Bool
    public let blocks: [PlatformBlock]
}

/// The state transitions one identity owns in one block
/// (`dash_sdk_system_get_owner_transactions_in_block`).
///
/// When `proved`, each transition is in that block with that result and is the owner's, but
/// the list is not proved complete: a node can leave some out.
public struct OwnerTransactionsInBlock: Codable, Equatable, Sendable {
    public let proved: Bool
    public let height: UInt64
    public let transactions: [PlatformBlockTransaction]
}

/// The error a node reports for a committed transition, unproved.
public struct NodeReportedTransitionError: Codable, Equatable, Sendable {
    public let code: UInt32
    public let message: String
    /// The consensus error the node attached, as text; nil when none decoded.
    public let consensusError: String?
}

/// One node's view of a transition (`dash_sdk_system_get_recent_state_transition_status`).
/// Always unproved: never decide anything about funds on it.
public struct NodeTransitionStatus: Codable, Equatable, Sendable {
    public enum Status: String, Codable, Sendable {
        case committed
        case pending
        case unknown
    }

    /// Always false.
    public let proved: Bool
    public let status: Status
    /// The height of the block the node says holds it, when committed.
    public let height: UInt64?
    /// The error the node reports it failed with, when committed and failed.
    public let error: NodeReportedTransitionError?
    /// The node's latest block height when it answered.
    public let nodeHeight: UInt64?
}

/// A transition's status from its hash, proved where the network can prove it
/// (`dash_sdk_system_get_state_transition_status_by_hash`). Only `.inBlock` is proved.
public struct TransitionStatusByHash: Codable, Equatable, Sendable {
    public enum Status: String, Codable, Sendable {
        /// Proved: the block a validator quorum committed at `blockHeight` holds it.
        case inBlock = "in_block"
        /// The node says committed at `height`, and no proved block came back for it.
        case committedNotProved = "committed_not_proved"
        /// The node says committed at `height`, and the proved block there does not hold it.
        case committedRefuted = "committed_refuted"
        /// The node says it is in its mempool.
        case pending
        /// The node knows nothing of it.
        case unknown
    }

    public let proved: Bool
    public let status: Status
    /// `.inBlock` only.
    public let blockHeight: UInt64?
    /// `.inBlock` only, hex.
    public let blockHash: String?
    /// `.inBlock` only, milliseconds since the Unix epoch.
    public let blockTimeMs: UInt64?
    /// `.inBlock` only: the transition with its result.
    public let transaction: PlatformBlockTransaction?
    /// The height the node named (`.committedNotProved`, `.committedRefuted`).
    public let height: UInt64?
    /// `.committedNotProved` only: the error the node reports, unproved.
    public let error: NodeReportedTransitionError?
    /// The node's latest block height when it answered (every case but `.inBlock`).
    public let nodeHeight: UInt64?
}

extension JSONDecoder {
    /// The decoder for the snake_case JSON of the block and transition status reads.
    static var platformBlocks: JSONDecoder {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return decoder
    }
}
