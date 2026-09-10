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
/// (`platform_wallet_wallet_utxos_page`).
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
/// (`platform_wallet_classify_outpoints`): the account the store files
/// the coin under and the script it recorded, which the engine checks
/// against its own pools rather than trusting.
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
/// output's script as its own, and does not hold the coin — under the
/// engine's `update_utxos` rules an owned output of a known record is
/// absent only because the engine skipped it for a spent reason or
/// consumed it. `unknown` includes every funding transaction this session
/// never processed (after a restart the engine's finalized set is empty),
/// so absence proves nothing and is never acted on.
public enum CoreOutpointClass: UInt8, Sendable {
    case unknown = 0
    case unspent = 1
    case knownUncredited = 2
    case notOwned = 3
}

/// The engine reads the store reconcile is built on. Production wraps the
/// two FFIs (`FFICoreTxoEngineInventory`); tests inject a fake so every
/// verdict the reconcile can reach is reproducible without a native
/// engine. Both reads park the calling thread on the engine's wallet lock,
/// so they run on the manager's reconcile queue — never on the persistence
/// queue (a persistence callback can be waiting on the same lock from the
/// other side) and never on a Swift Concurrency pool thread.
protocol CoreTxoEngineInventory: Sendable {
    /// One page of the wallet's UTXO inventory strictly after `after`
    /// (`nil` starts the walk), at most `limit` rows, and whether more
    /// follow.
    func utxoPage(after: CoreEngineUtxo?, limit: Int) throws -> (rows: [CoreEngineUtxo], hasMore: Bool)
    /// Positional: `result[i]` answers `queries[i]`.
    func classify(_ queries: [CoreOutpointOwnershipQuery]) throws -> [CoreOutpointClass]
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
    /// Engine rows below the confirmation gate.
    public var skippedImmature = 0
    /// Engine rows whose account has no store row to file them under.
    public var skippedUnresolvedAccount = 0
    /// Engine rows the store could not validate (malformed txid, no
    /// script, no address).
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
    public var unknown = 0
    /// Store rows whose script the engine does not monitor — left
    /// untouched, reported.
    public var notOwned = 0

    // Run shape.
    /// Steps deferred because a Rust persistence round was open.
    public var retries = 0
    /// Pages classified again because a persistence round committed between
    /// their read and their apply.
    public var staleRetries = 0
    /// Engine reads that failed; the run stops at the first.
    public var transportFailures = 0
    /// Store writes that failed to save; the run stops at the first.
    public var storeFailures = 0
    /// `false` when the run stopped early (cancelled, a failed read or
    /// write, or too many deferrals); what landed before the stop stays.
    public var completed = true

    public init() {}

    /// Rows this run wrote: healed, flipped, and healed rows the drain wrote
    /// spent on insert — the last is a divergence recorded, not repaired,
    /// but it is a row the store did not have before.
    public var mutations: Int { inserted + flipped + healedSpent }
}

/// Why `reconcileCoreTxoStore(for:)` did not run.
public enum CoreTxoReconcileSkipReason: String, Sendable {
    case notConfigured
    case shutdownRequested
    case walletUnknown
    case alreadyRunning
    case spvNotRunning
    case notSteadyState
    case tipUnavailable
    case syncFaultDetected
    case walletBehindTip
}

public enum CoreTxoReconcileOutcome: Equatable, Sendable {
    case skipped(CoreTxoReconcileSkipReason)
    case reconciled(CoreTxoReconcileReport)
}

// MARK: - FFI adapter

/// The production `CoreTxoEngineInventory`: the two paged FFI reads on
/// one manager handle, for one wallet.
struct FFICoreTxoEngineInventory: CoreTxoEngineInventory {
    let handle: Handle
    let walletId: Data

    func utxoPage(after: CoreEngineUtxo?, limit: Int) throws -> (rows: [CoreEngineUtxo], hasMore: Bool) {
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
        var rowsPtr: UnsafePointer<WalletUtxoEntryFFI>?
        var count: UInt = 0
        var hasMore = false
        let result = walletId.withUnsafeBytes { raw -> PlatformWalletFFIResult in
            withUnsafePointer(to: &cursor) { cursorPtr in
                platform_wallet_wallet_utxos_page(
                    handle,
                    raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    hasCursor ? cursorPtr : nil,
                    UInt(max(limit, 0)),
                    &rowsPtr,
                    &count,
                    &hasMore
                )
            }
        }
        try result.check()
        guard let rowsPtr, count > 0 else { return ([], hasMore) }
        defer { platform_wallet_wallet_utxos_page_free(UnsafeMutablePointer(mutating: rowsPtr), count) }
        var rows: [CoreEngineUtxo] = []
        rows.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            var entry = rowsPtr[i]
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
            rows.append(CoreEngineUtxo(
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
            ))
        }
        return (rows, hasMore)
    }

    func classify(_ queries: [CoreOutpointOwnershipQuery]) throws -> [CoreOutpointClass] {
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
        var classes = [UInt8](repeating: 0, count: queries.count)
        let result = walletId.withUnsafeBytes { raw -> PlatformWalletFFIResult in
            ffiQueries.withUnsafeBufferPointer { queriesPtr in
                classes.withUnsafeMutableBufferPointer { classesPtr in
                    platform_wallet_classify_outpoints(
                        handle,
                        raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        queriesPtr.baseAddress,
                        UInt(queries.count),
                        classesPtr.baseAddress
                    )
                }
            }
        }
        try result.check()
        return classes.map { CoreOutpointClass(rawValue: $0) ?? .unknown }
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
