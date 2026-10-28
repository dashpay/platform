import DashSDKFFI
import Foundation

// MARK: - Engine-side values

/// The seven-field identity the engine files a coin under — the same tuple
/// `AccountSpecFFI`, `AccountBalanceEntryFFI` and the persisted
/// `PersistentAccount` row carry, so a coin can be matched to its store
/// account without going through an address.
public struct CoreAccountKey: Hashable, Sendable {
    public var typeTag: UInt8
    public var standardTag: UInt8
    public var index: UInt32
    public var registrationIndex: UInt32
    public var keyClass: UInt32
    /// 32 bytes; all zero for accounts that carry no identity.
    public var userIdentityId: Data
    /// 32 bytes; all zero for accounts that carry no identity.
    public var friendIdentityId: Data

    public init(
        typeTag: UInt8,
        standardTag: UInt8,
        index: UInt32,
        registrationIndex: UInt32,
        keyClass: UInt32,
        userIdentityId: Data,
        friendIdentityId: Data
    ) {
        self.typeTag = typeTag
        self.standardTag = standardTag
        self.index = index
        self.registrationIndex = registrationIndex
        self.keyClass = keyClass
        self.userIdentityId = userIdentityId
        self.friendIdentityId = friendIdentityId
    }

}

/// One coin the engine holds, as one row of the paged inventory
/// (`platform_wallet_txo_reconcile_heal_page`).
public struct CoreEngineUtxo: Equatable, Sendable {
    public var account: CoreAccountKey
    /// 32-byte txid in wire orientation — the same bytes the changeset
    /// path hands the persister.
    public var txid: Data
    public var vout: UInt32
    public var amount: UInt64
    /// Base58Check address the engine derived for the output; empty when
    /// the script has no address form.
    public var address: String
    public var scriptPubKey: Data
    public var height: UInt32
    public var isConfirmed: Bool
    public var isInstantLocked: Bool
    public var isCoinbase: Bool
    public var isLocked: Bool

    public init(
        account: CoreAccountKey,
        txid: Data,
        vout: UInt32,
        amount: UInt64,
        address: String,
        scriptPubKey: Data,
        height: UInt32,
        isConfirmed: Bool,
        isInstantLocked: Bool,
        isCoinbase: Bool,
        isLocked: Bool
    ) {
        self.account = account
        self.txid = txid
        self.vout = vout
        self.amount = amount
        self.address = address
        self.scriptPubKey = scriptPubKey
        self.height = height
        self.isConfirmed = isConfirmed
        self.isInstantLocked = isInstantLocked
        self.isCoinbase = isCoinbase
        self.isLocked = isLocked
    }

    /// The 36-byte `PersistentTxo.outpoint` key of this coin.
    public var outpoint: Data {
        PersistentTxo.makeOutpoint(txid: txid, vout: vout)
    }
}

/// One store row the reconcile asks the engine about
/// (`platform_wallet_txo_reconcile_classify_outpoints`): the account the
/// store files the coin under and the script it recorded, which the engine
/// checks against its own pools rather than trusting.
public struct CoreOutpointOwnershipQuery: Equatable, Sendable {
    public var account: CoreAccountKey
    public var txid: Data
    public var vout: UInt32
    public var scriptPubKey: Data

    public init(account: CoreAccountKey, txid: Data, vout: UInt32, scriptPubKey: Data) {
        self.account = account
        self.txid = txid
        self.vout = vout
        self.scriptPubKey = scriptPubKey
    }

    public var outpoint: Data {
        PersistentTxo.makeOutpoint(txid: txid, vout: vout)
    }
}

/// The engine's answer for one `CoreOutpointOwnershipQuery` — the
/// `OUTPOINT_CLASS_*` constants of the FFI.
///
/// Only `knownUncredited` is positive evidence the reconcile acts on: the
/// owning account recorded the funding transaction, recognises the
/// output's script as its own, does not hold the coin, AND a funds account
/// holds a mined record whose transaction spends the outpoint. Absence
/// from the engine's `utxos` alone is not durable evidence: a mempool-only
/// or IS-locked spend, a released conflict loser, and a spender the engine
/// never recorded (the emit-time verdict covers that shape) all answer
/// `unknown`. `unknown` also covers every funding transaction this session
/// never processed (after a restart the engine's finalized set is empty),
/// so absence proves nothing and is never acted on.
public enum CoreOutpointClass: UInt8, Sendable {
    case unknown = 0
    case unspent = 1
    case knownUncredited = 2
    case notOwned = 3
}

/// Whether the store may heal one engine coin in — the `TXO_HEAL_VERDICT_*`
/// constants. The verdict is `platform-wallet`'s (`txo_heal_verdict`: a
/// script and an address to file the row under, engine-confirmed, and deep
/// enough at the run's scan tip); the store only follows it.
enum CoreTxoHealVerdict: Equatable, Sendable {
    /// No script or no address: the store could not file the row.
    case invalid
    /// Not confirmed by the engine, or not deep enough yet; it ages into a
    /// later run.
    case immature
    /// May be inserted unspent, if the store lacks the outpoint and
    /// resolves the account that owns it.
    case eligible

    /// A code this build does not know is nothing the engine vouched for.
    init(code: UInt8) {
        switch code {
        case UInt8(TXO_HEAL_VERDICT_ELIGIBLE):
            self = .eligible
        case UInt8(TXO_HEAL_VERDICT_IMMATURE):
            self = .immature
        default:
            self = .invalid
        }
    }
}

/// One row of a heal page: an engine coin and Rust's verdict on it.
struct CoreTxoHealRow: Equatable, Sendable {
    let utxo: CoreEngineUtxo
    let verdict: CoreTxoHealVerdict
}

/// The engine side of one reconcile run — one wallet, claimed by
/// `platform_wallet_manager_txo_reconcile_begin`, at the scan tip that call
/// returned. Every decision a run makes is asked here: which engine coins
/// may be healed, which store rows the engine proves spent, and whether a
/// stale page is classified again. Production wraps the C API
/// (`FFICoreTxoReconcileEngine`); tests inject a fake so every path of the
/// run is reproducible without a native engine.
///
/// The reads park the calling thread on the engine's wallet lock, so they
/// run on the manager's reconcile queue — never on the persistence queue (a
/// persistence callback can be waiting on the same lock from the other
/// side) and never on a Swift Concurrency pool thread.
protocol CoreTxoReconcileEngine: Sendable {
    /// One page of the wallet's UTXO inventory strictly after `after` (`nil`
    /// starts the walk), at most `limit` rows, each with its heal verdict,
    /// and whether more follow.
    func healPage(after: CoreEngineUtxo?, limit: Int) throws -> (rows: [CoreTxoHealRow], hasMore: Bool)

    /// Positional: `result[i]` answers `queries[i]`. The engine keeps its
    /// wallet lock held for reading when this returns, so no fork can drop
    /// a spender these verdicts name before the store has written them;
    /// `releaseVerdicts()` lets it go. A classification asked for while a
    /// previous one is still held frees that one first.
    func classifyHeld(_ queries: [CoreOutpointOwnershipQuery]) throws -> [CoreOutpointClass]

    /// Release the lock the last `classifyHeld` holds, if it still does.
    /// Idempotent, never blocks, and never resolves the manager handle, so
    /// it can run whatever is queued on the handle registry or the engine.
    func releaseVerdicts()

    /// Whether the run classifies a page again after it came back stale for
    /// the `staleInARow`-th time in a row (counting this one), or stops
    /// incomplete.
    func retryStalePage(staleInARow: Int) -> Bool
}

// MARK: - Outcome

/// What one reconcile run did. Counts only — no outpoint, address or
/// amount of an individual coin leaves the store through this type or
/// the events built from it.
public struct CoreTxoReconcileReport: Equatable, Sendable {
    // Heal pass (engine → store, insert-only).
    /// Engine inventory rows walked.
    public var engineRows = 0
    /// Rows the store already held.
    public var alreadyPresent = 0
    /// Rows inserted.
    public var inserted = 0
    public var insertedDuffs: UInt64 = 0
    /// Engine coins the store lacked whose row the pending-input drain wrote
    /// spent on insert: the divergence is recorded, not repaired.
    public var healedSpent = 0
    /// Engine rows the engine calls immature: not confirmed, or not deep
    /// enough yet. They age into a later run.
    public var skippedImmature = 0
    /// Engine rows whose account has no store row to file them under.
    public var skippedUnresolvedAccount = 0
    /// Engine rows the engine calls invalid (no script, no address), or
    /// whose txid is not 32 bytes.
    public var skippedInvalid = 0

    // Classify pass (store → engine, flip-only).
    /// Unspent store rows of this wallet handed to the engine.
    public var storeRows = 0
    /// Store rows the engine reported unspent — consistent.
    public var unspent = 0
    /// Store rows flipped to spent on the engine's positive verdict.
    public var flipped = 0
    public var flippedDuffs: UInt64 = 0
    /// Store rows the engine has no opinion about — left untouched.
    /// Watch-only DIP-15 contact rows land here: the engine never gives a
    /// contact's coin a verdict.
    public var unknown = 0
    /// Store rows whose script the engine does not monitor — left
    /// untouched, reported.
    public var notOwned = 0
    /// Unspent store rows of this wallet with no account to name to the
    /// engine (or no well-formed txid). They carry no claim the engine
    /// could check, so they are counted, not asked about.
    public var unresolvedAccount = 0

    // Run shape.
    /// Store steps deferred because a Rust persistence round was open.
    public var retries = 0
    /// Pages classified again because a persistence round committed between
    /// their read and their apply, or was still open at the apply.
    public var staleRetries = 0
    /// Engine reads that failed; the run stops at the first.
    public var transportFailures = 0
    /// Store reads or writes that failed; the run stops at the first.
    public var storeFailures = 0
    /// `false` when the run stopped early (cancelled, a failed read or
    /// write, too many deferrals, or a page that stayed stale for longer
    /// than the engine allows); what landed before the stop stays.
    public var completed = true

    public init() {}

    /// Rows this run wrote: healed, flipped, and healed rows the drain wrote
    /// spent on insert — the last is a divergence recorded, not repaired,
    /// but it is a row the store did not have before.
    public var mutations: Int { inserted + flipped + healedSpent }
}

/// Why `reconcileCoreTxoStore(for:)` did not run. Every reason but
/// `notConfigured`, `shutdownRequested` and a wallet this manager has not
/// loaded is `platform-wallet`'s start gate answering
/// (`platform_wallet_manager_txo_reconcile_begin`).
public enum CoreTxoReconcileSkipReason: String, Sendable {
    case notConfigured
    case shutdownRequested
    /// The manager has not loaded the wallet, or the engine does not hold
    /// it, or it was removed while the run waited to start.
    case walletUnknown
    case alreadyRunning
    /// No longer reported: the start gate answers `notSteadyState` for a
    /// client that is not running. Kept so existing `switch` statements
    /// still compile.
    case spvNotRunning
    /// SPV is not running, or not in its steady state.
    case notSteadyState
    case tipUnavailable
    case syncFaultDetected
    case walletBehindTip

    /// The reason a non-run code from the start gate names; `nil` for a
    /// code this build does not know.
    init?(gateCode: UInt8) {
        switch gateCode {
        case UInt8(TXO_RECONCILE_SKIP_WALLET_UNKNOWN):
            self = .walletUnknown
        case UInt8(TXO_RECONCILE_SKIP_ALREADY_RUNNING):
            self = .alreadyRunning
        case UInt8(TXO_RECONCILE_SKIP_NOT_STEADY_STATE):
            self = .notSteadyState
        case UInt8(TXO_RECONCILE_SKIP_TIP_UNAVAILABLE):
            self = .tipUnavailable
        case UInt8(TXO_RECONCILE_SKIP_SYNC_FAULT_DETECTED):
            self = .syncFaultDetected
        case UInt8(TXO_RECONCILE_SKIP_WALLET_BEHIND_TIP):
            self = .walletBehindTip
        default:
            return nil
        }
    }
}

public enum CoreTxoReconcileOutcome: Equatable, Sendable {
    case skipped(CoreTxoReconcileSkipReason)
    case reconciled(CoreTxoReconcileReport)
}

// MARK: - Native calls

/// What `platform_wallet_manager_txo_reconcile_begin` answered.
enum CoreTxoReconcileGate: Equatable, Sendable {
    /// The run may start at `tipHeight`; the wallet is claimed until
    /// `platform_wallet_manager_txo_reconcile_finish`.
    case run(tipHeight: UInt32)
    /// The gate's `TXO_RECONCILE_SKIP_*` code. Nothing is claimed.
    case skip(code: UInt8)
}

/// The native calls that bracket one reconcile run; same seam shape as
/// [`PlatformWalletNativePollCalls`]. Tests with a fake manager handle must
/// inject this table: handles come from a process-global registry.
struct PlatformWalletNativeTxoReconcileCalls: @unchecked Sendable {
    /// Claim the wallet and run the start gate.
    let begin: @Sendable (Handle, Data) throws -> CoreTxoReconcileGate
    /// Release a `.run` claim; the flag is whether the run went ahead.
    let finish: @Sendable (Handle, Data, Bool) throws -> Void
    /// The engine side of one run, at the tip `begin` returned.
    let makeEngine: @Sendable (Handle, Data, UInt32) -> any CoreTxoReconcileEngine

    static let live = PlatformWalletNativeTxoReconcileCalls(
        begin: CoreTxoReconcileFFI.begin,
        finish: CoreTxoReconcileFFI.finish,
        makeEngine: { handle, walletId, tipHeight in
            FFICoreTxoReconcileEngine(handle: handle, walletId: walletId, tipHeight: tipHeight)
        }
    )
}

/// Thin wrappers over the reconcile C API's schedule and claim calls. Each
/// parks the calling thread — call off-main.
enum CoreTxoReconcileFFI {
    /// The loaded wallets whose reconcile is due now
    /// (`platform_wallet_manager_txo_reconcile_due_wallets`). Called on every
    /// progress poll, running or not: the poll is the schedule's clock.
    static func dueWallets(_ handle: Handle) throws -> [Data] {
        var array = IdentifierArray()
        try platform_wallet_manager_txo_reconcile_due_wallets(handle, &array).check()
        defer { platform_wallet_identifier_array_free(&array) }
        guard array.items != nil, array.count > 0 else { return [] }
        return (0..<Int(array.count)).map { identifierFromFFIArray(array, at: $0) }
    }

    static func begin(_ handle: Handle, walletId: Data) throws -> CoreTxoReconcileGate {
        guard walletId.count == 32 else {
            return .skip(code: UInt8(TXO_RECONCILE_SKIP_WALLET_UNKNOWN))
        }
        var gate = UInt8(TXO_RECONCILE_SKIP_WALLET_UNKNOWN)
        var tipHeight: UInt32 = 0
        let result = walletId.withUnsafeBytes { raw -> PlatformWalletFFIResult in
            platform_wallet_manager_txo_reconcile_begin(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &gate,
                &tipHeight
            )
        }
        try result.check()
        return gate == UInt8(TXO_RECONCILE_GATE_RUN) ? .run(tipHeight: tipHeight) : .skip(code: gate)
    }

    static func finish(_ handle: Handle, walletId: Data, ran: Bool) throws {
        guard walletId.count == 32 else { return }
        let result = walletId.withUnsafeBytes { raw -> PlatformWalletFFIResult in
            platform_wallet_manager_txo_reconcile_finish(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                ran
            )
        }
        try result.check()
    }
}

// MARK: - FFI adapter

/// The production `CoreTxoReconcileEngine`: the reconcile C API on one
/// manager handle, for one claimed wallet and the tip its begin returned.
///
/// Holds at most one engine guard — the token the last classification
/// handed out — and frees it in `releaseVerdicts()`, in the next
/// classification before it asks, or as a last resort in `deinit`. The free
/// takes no manager handle, so it is safe whatever else is queued.
final class FFICoreTxoReconcileEngine: CoreTxoReconcileEngine, @unchecked Sendable {
    let handle: Handle
    let walletId: Data
    let tipHeight: UInt32
    private let lock = NSLock()
    private var held: OpaquePointer?

    init(handle: Handle, walletId: Data, tipHeight: UInt32) {
        self.handle = handle
        self.walletId = walletId
        self.tipHeight = tipHeight
    }

    deinit {
        releaseVerdicts()
    }

    func healPage(after: CoreEngineUtxo?, limit: Int) throws -> (rows: [CoreTxoHealRow], hasMore: Bool) {
        var cursor = WalletUtxoCursorFFI()
        if let after {
            cursor.type_tag = after.account.typeTag
            cursor.standard_tag = after.account.standardTag
            cursor.index = after.account.index
            cursor.registration_index = after.account.registrationIndex
            cursor.key_class = after.account.keyClass
            Self.copy(after.account.userIdentityId, into: &cursor.user_identity_id)
            Self.copy(after.account.friendIdentityId, into: &cursor.friend_identity_id)
            Self.copy(after.txid, into: &cursor.outpoint.txid)
            cursor.outpoint.vout = after.vout
        }
        let hasCursor = after != nil
        var rowsPtr: UnsafePointer<TxoHealRowFFI>?
        var count: UInt = 0
        var hasMore = false
        let result = walletId.withUnsafeBytes { raw -> PlatformWalletFFIResult in
            withUnsafePointer(to: &cursor) { cursorPtr in
                platform_wallet_txo_reconcile_heal_page(
                    handle,
                    raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    hasCursor ? cursorPtr : nil,
                    UInt(max(limit, 0)),
                    tipHeight,
                    &rowsPtr,
                    &count,
                    &hasMore
                )
            }
        }
        try result.check()
        guard let rowsPtr, count > 0 else { return ([], hasMore) }
        defer { platform_wallet_txo_reconcile_heal_page_free(UnsafeMutablePointer(mutating: rowsPtr), count) }
        var rows: [CoreTxoHealRow] = []
        rows.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let row = rowsPtr[i]
            var entry = row.utxo
            let account = CoreAccountKey(
                typeTag: entry.type_tag,
                standardTag: entry.standard_tag,
                index: entry.index,
                registrationIndex: entry.registration_index,
                keyClass: entry.key_class,
                userIdentityId: withUnsafeBytes(of: &entry.user_identity_id) { Data($0) },
                friendIdentityId: withUnsafeBytes(of: &entry.friend_identity_id) { Data($0) }
            )
            let script = entry.script_pubkey.map {
                Data(bytes: $0, count: Int(entry.script_pubkey_len))
            } ?? Data()
            let utxo = CoreEngineUtxo(
                account: account,
                txid: withUnsafeBytes(of: &entry.outpoint.txid) { Data($0) },
                vout: entry.outpoint.vout,
                amount: entry.value_duffs,
                address: entry.address.map { String(cString: $0) } ?? "",
                scriptPubKey: script,
                height: entry.height,
                isConfirmed: entry.is_confirmed,
                isInstantLocked: entry.is_instantlocked,
                isCoinbase: entry.is_coinbase,
                isLocked: entry.is_locked
            )
            rows.append(CoreTxoHealRow(utxo: utxo, verdict: CoreTxoHealVerdict(code: row.verdict)))
        }
        return (rows, hasMore)
    }

    func classifyHeld(_ queries: [CoreOutpointOwnershipQuery]) throws -> [CoreOutpointClass] {
        guard !queries.isEmpty else { return [] }
        // Every script must stay addressable for the whole call: copy each
        // into its own allocation rather than nesting `withUnsafeBytes`
        // closures `queries.count` deep.
        var buffers: [UnsafeMutablePointer<UInt8>] = []
        defer { buffers.forEach { $0.deallocate() } }
        var ffiQueries: [OutpointOwnershipQueryFFI] = []
        ffiQueries.reserveCapacity(queries.count)
        for query in queries {
            var entry = OutpointOwnershipQueryFFI()
            entry.type_tag = query.account.typeTag
            entry.standard_tag = query.account.standardTag
            entry.index = query.account.index
            entry.registration_index = query.account.registrationIndex
            entry.key_class = query.account.keyClass
            Self.copy(query.account.userIdentityId, into: &entry.user_identity_id)
            Self.copy(query.account.friendIdentityId, into: &entry.friend_identity_id)
            Self.copy(query.txid, into: &entry.outpoint.txid)
            entry.outpoint.vout = query.vout
            let scriptCount = query.scriptPubKey.count
            let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: max(scriptCount, 1))
            query.scriptPubKey.copyBytes(to: buffer, count: scriptCount)
            buffers.append(buffer)
            entry.script_pubkey = scriptCount > 0 ? UnsafePointer(buffer) : nil
            entry.script_pubkey_len = UInt(scriptCount)
            ffiQueries.append(entry)
        }
        // A guard the caller failed to release goes before this asks for a
        // new one; Rust would drop it too, but the token is ours to free.
        releaseVerdicts()
        var classes = [UInt8](repeating: 0, count: queries.count)
        var token: OpaquePointer?
        let result = walletId.withUnsafeBytes { raw -> PlatformWalletFFIResult in
            ffiQueries.withUnsafeBufferPointer { queriesPtr in
                classes.withUnsafeMutableBufferPointer { classesPtr in
                    platform_wallet_txo_reconcile_classify_outpoints(
                        handle,
                        raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        queriesPtr.baseAddress,
                        UInt(queries.count),
                        classesPtr.baseAddress,
                        &token
                    )
                }
            }
        }
        // Kept before the result is checked, so a token handed out by a call
        // that then failed is still released, never dropped.
        keep(token)
        try result.check()
        return classes.map { CoreOutpointClass(rawValue: $0) ?? .unknown }
    }

    func releaseVerdicts() {
        let token = lock.withLock { () -> OpaquePointer? in
            defer { held = nil }
            return held
        }
        if let token {
            platform_wallet_txo_reconcile_held_verdicts_free(token)
        }
    }

    func retryStalePage(staleInARow: Int) -> Bool {
        platform_wallet_txo_reconcile_retry_stale_page(UInt32(clamping: staleInARow))
    }

    /// Hold `token` as the run's one guard. A guard already held is freed,
    /// never overwritten.
    private func keep(_ token: OpaquePointer?) {
        let previous = lock.withLock { () -> OpaquePointer? in
            defer { held = token }
            return held
        }
        if let previous {
            platform_wallet_txo_reconcile_held_verdicts_free(previous)
        }
    }

    /// Copy up to 32 bytes of `data` into a C `uint8_t[32]` field,
    /// zero-filling the rest.
    private static func copy<T>(_ data: Data, into field: inout T) {
        withUnsafeMutableBytes(of: &field) { raw in
            raw.initializeMemory(as: UInt8.self, repeating: 0)
            let count = min(raw.count, data.count)
            data.copyBytes(to: raw.bindMemory(to: UInt8.self), count: count)
        }
    }
}
