import Foundation

/// One evonode's DAPI `getStatus` self-report, as returned by
/// `SDK.getEvonodeStatus(address:)` — every field the node sent, typed.
///
/// Mirrors the Rust `EvoNodeStatus` (drive-proof-verifier) one-to-one; the
/// JSON wire shape is produced by `dash_sdk_evonode_get_status`. Optional
/// protobuf fields the node omitted decode as `nil` — callers should say
/// "not reported" rather than show a zero. The raw `Time` values are passed
/// through as sent; use `Time.localDate` / `blockDate` / `genesisDate` for
/// unit-correct dates (see `Time`).
///
/// The report is unproved by nature — it is the node describing itself — so
/// treat it as diagnostics, not as chain state.
public struct EvonodeStatus: Codable, Equatable, Sendable {
    public let version: Version
    public let node: Node
    public let chain: Chain
    public let network: Network
    public let stateSync: StateSync
    public let time: Time

    /// Software and protocol versions the node runs.
    public struct Version: Codable, Equatable, Sendable {
        public let software: SoftwareVersions?
        public let `protocol`: ProtocolVersions?

        public init(software: SoftwareVersions?, protocol: ProtocolVersions?) {
            self.software = software
            self.protocol = `protocol`
        }
    }

    /// Software component versions (semver strings).
    public struct SoftwareVersions: Codable, Equatable, Sendable {
        public let dapi: String
        public let drive: String?
        public let tenderdash: String?

        public init(dapi: String, drive: String?, tenderdash: String?) {
            self.dapi = dapi
            self.drive = drive
            self.tenderdash = tenderdash
        }
    }

    /// Protocol-level versions.
    public struct ProtocolVersions: Codable, Equatable, Sendable {
        public let tenderdash: TenderdashProtocol?
        public let drive: DriveProtocol?

        public init(tenderdash: TenderdashProtocol?, drive: DriveProtocol?) {
            self.tenderdash = tenderdash
            self.drive = drive
        }
    }

    public struct TenderdashProtocol: Codable, Equatable, Sendable {
        /// Tenderdash P2P protocol version.
        public let p2p: UInt32
        /// Tenderdash block protocol version.
        public let block: UInt32

        public init(p2p: UInt32, block: UInt32) {
            self.p2p = p2p
            self.block = block
        }
    }

    public struct DriveProtocol: Codable, Equatable, Sendable {
        /// Latest protocol version the node supports.
        public let latest: UInt32
        /// Protocol version the node currently runs.
        public let current: UInt32
        /// Protocol version scheduled for the next epoch.
        public let nextEpoch: UInt32

        public init(latest: UInt32, current: UInt32, nextEpoch: UInt32) {
            self.latest = latest
            self.current = current
            self.nextEpoch = nextEpoch
        }
    }

    /// Node identification.
    public struct Node: Codable, Equatable, Sendable {
        /// Tenderdash node id, hex.
        public let id: String
        /// proTxHash of the masternode, hex; `nil` for a full node.
        public let proTxHash: String?

        public init(id: String, proTxHash: String?) {
            self.id = id
            self.proTxHash = proTxHash
        }
    }

    /// Layer-2 chain state as the node sees it.
    public struct Chain: Codable, Equatable, Sendable {
        /// Whether the node is still catching up with the network.
        public let catchingUp: Bool
        /// Hex hashes of the latest / earliest blocks the node holds.
        public let latestBlockHash: String
        public let latestAppHash: String
        public let earliestBlockHash: String
        public let earliestAppHash: String
        public let latestBlockHeight: UInt64
        public let earliestBlockHeight: UInt64
        /// Highest block height among the node's connected peers.
        public let maxPeerBlockHeight: UInt64
        /// Core height the chain is locked to, when reported.
        public let coreChainLockedHeight: UInt32?

        public init(
            catchingUp: Bool,
            latestBlockHash: String,
            latestAppHash: String,
            earliestBlockHash: String,
            earliestAppHash: String,
            latestBlockHeight: UInt64,
            earliestBlockHeight: UInt64,
            maxPeerBlockHeight: UInt64,
            coreChainLockedHeight: UInt32?
        ) {
            self.catchingUp = catchingUp
            self.latestBlockHash = latestBlockHash
            self.latestAppHash = latestAppHash
            self.earliestBlockHash = earliestBlockHash
            self.earliestAppHash = earliestAppHash
            self.latestBlockHeight = latestBlockHeight
            self.earliestBlockHeight = earliestBlockHeight
            self.maxPeerBlockHeight = maxPeerBlockHeight
            self.coreChainLockedHeight = coreChainLockedHeight
        }
    }

    /// Node networking information.
    public struct Network: Codable, Equatable, Sendable {
        /// Identifier of the chain the node is a member of (e.g. `dash-testnet-51`).
        public let chainId: String
        /// Number of peers in the node's address book.
        public let peersCount: UInt32
        /// Whether the node is listening for incoming connections.
        public let listening: Bool

        public init(chainId: String, peersCount: UInt32, listening: Bool) {
            self.chainId = chainId
            self.peersCount = peersCount
            self.listening = listening
        }
    }

    /// State-sync (snapshot) progress.
    public struct StateSync: Codable, Equatable, Sendable {
        public let totalSyncedTime: UInt64
        public let remainingTime: UInt64
        public let totalSnapshots: UInt32
        public let chunkProcessAvgTime: UInt64
        public let snapshotHeight: UInt64
        public let snapshotChunksCount: UInt64
        public let backfilledBlocks: UInt64
        public let backfillBlocksTotal: UInt64

        public init(
            totalSyncedTime: UInt64,
            remainingTime: UInt64,
            totalSnapshots: UInt32,
            chunkProcessAvgTime: UInt64,
            snapshotHeight: UInt64,
            snapshotChunksCount: UInt64,
            backfilledBlocks: UInt64,
            backfillBlocksTotal: UInt64
        ) {
            self.totalSyncedTime = totalSyncedTime
            self.remainingTime = remainingTime
            self.totalSnapshots = totalSnapshots
            self.chunkProcessAvgTime = chunkProcessAvgTime
            self.snapshotHeight = snapshotHeight
            self.snapshotChunksCount = snapshotChunksCount
            self.backfilledBlocks = backfilledBlocks
            self.backfillBlocksTotal = backfillBlocksTotal
        }
    }

    /// Clocks as the node sees them, raw as sent. Units differ by field and
    /// by DAPI implementation: `block` / `genesis` are Unix milliseconds
    /// (Drive's `time_ms`; Drive sends `0` for an unknown genesis), while
    /// `local` is Unix seconds from rs-dapi and milliseconds from the legacy
    /// JS DAPI. The `*Date` accessors resolve that — prefer them.
    public struct Time: Codable, Equatable, Sendable {
        /// The node's local wall clock at response time (seconds or ms — see above).
        public let local: UInt64
        /// Time of the latest block, ms.
        public let block: UInt64?
        /// Genesis time, ms; `0` when the node doesn't know it.
        public let genesis: UInt64?
        /// Current epoch index.
        public let epoch: UInt32?

        public init(local: UInt64, block: UInt64?, genesis: UInt64?, epoch: UInt32?) {
            self.local = local
            self.block = block
            self.genesis = genesis
            self.epoch = epoch
        }

        /// The node's wall clock, or `nil` when it sent `0`.
        public var localDate: Date? { Self.date(fromUnix: local) }
        /// Latest block time, or `nil` when absent / `0`.
        public var blockDate: Date? { block.flatMap(Self.date(fromUnix:)) }
        /// Genesis time, or `nil` when absent / `0` (unknown to the node).
        public var genesisDate: Date? { genesis.flatMap(Self.date(fromUnix:)) }

        /// Unix seconds and Unix milliseconds never overlap in magnitude for
        /// any date a node can report: seconds stay below 10^11 until the
        /// year 5138, and milliseconds passed 10^11 in 1973. `0` is "not
        /// reported", never the epoch.
        static func date(fromUnix value: UInt64) -> Date? {
            guard value > 0 else { return nil }
            let seconds = value >= 100_000_000_000
                ? TimeInterval(value) / 1000
                : TimeInterval(value)
            return Date(timeIntervalSince1970: seconds)
        }
    }

    public init(version: Version, node: Node, chain: Chain, network: Network, stateSync: StateSync, time: Time) {
        self.version = version
        self.node = node
        self.chain = chain
        self.network = network
        self.stateSync = stateSync
        self.time = time
    }
}
