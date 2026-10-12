import Foundation
import SwiftData

/// SwiftData model for persisting a wallet transaction.
///
/// Stores the full transaction record including context (mempool,
/// confirmed, chain-locked), direction, amounts, and fee.
///
/// A transaction is intentionally **not** scoped to a single wallet
/// or account. The same on-chain tx can have outputs into account A
/// and account B inside one wallet (or — for cross-wallet flows —
/// into accounts under different wallets), and the transaction row
/// is shared across all of them. Per-wallet membership is recovered
/// by joining through the TXOs (`outputs` + `inputs`); see
/// `PersistentTxo.walletId` for the per-row denorm that makes those
/// joins index-friendly.
///
/// The TXO join is the canonical membership path for funds, but it
/// is empty for **payload-only** involvement: a special transaction
/// (e.g. a ProRegTx) can match an account purely through its payload
/// — a Provider Owner / Voting key address baked into the special-tx
/// fields — while creating **no** TXO in that account. Such a tx is
/// invisible to the TXO join yet is genuinely "this account's". The
/// explicit `involvedAccounts` many-to-many below is the only place
/// that involvement is representable; see it for the full semantics.
@Model
public final class PersistentTransaction {
    /// Index on `firstSeen` so per-wallet queries — which fetch
    /// `PersistentTxo` rows by `walletId` then sort their parent
    /// transactions by `firstSeen` — get a sorted scan instead of
    /// an in-memory O(N log N) pass. The unique `txid` index covers
    /// point-lookups; this one covers the timeline.
    #Index<PersistentTransaction>([\.firstSeen])

    /// Transaction ID (32-byte hash, raw little-endian wire bytes —
    /// the same orientation Rust hands us via the FFI `[u8; 32]`).
    /// Stored as raw `Data` so the unique index covers 32 bytes
    /// instead of a 64-char hex string, and the persistence
    /// handler avoids a hex round-trip on every write.
    @Attribute(.unique) public var txid: Data
    /// Raw transaction bytes (consensus-encoded — the same wire
    /// format `dashcore::consensus::encode::serialize` produces and
    /// `Transaction::consensus_decode` round-trips). The FFI write
    /// path always populates this; the persister-fallback read path
    /// (`PlatformWalletPersistence::get_core_tx_record`) hands it
    /// back over FFI so Rust can decode a real `Transaction`
    /// without a placeholder body.
    public var transactionData: Data
    /// Context: 0=mempool, 1=instantSend, 2=inBlock, 3=inChainLockedBlock.
    public var context: UInt32
    /// Block height (0 for mempool).
    public var blockHeight: UInt32
    /// Block hash (nil for mempool).
    public var blockHash: Data?
    /// Block timestamp.
    public var blockTimestamp: UInt32
    /// The transaction's index within its block (`block.vtx` order),
    /// meaningful only when [`hasBlockPosition`]. Pure storage of the
    /// Rust-stamped value (rust-dashcore#891): restored provider special
    /// transactions hand it back so the masternode aggregation keeps
    /// Core's same-block apply order across restarts. `false` on rows
    /// persisted before the field existed and on unconfirmed contexts.
    public var blockPosition: UInt32 = 0
    public var hasBlockPosition: Bool = false
    /// Direction: 0=incoming, 1=outgoing, 2=internal, 3=coinJoin.
    public var direction: UInt32
    /// Transaction type name (Standard, CoinJoin, etc.). Sourced
    /// from Rust's `Debug` repr of `TransactionType` for human
    /// display only — DO NOT use this string as a discriminant;
    /// match on [`transactionTypeKind`] instead. The string is
    /// not a stable wire contract (a `#[derive(Debug)]` rename on
    /// the Rust side would silently change it).
    public var transactionType: String
    /// Typed discriminant of Rust's
    /// `key_wallet::transaction_checking::transaction_router::TransactionType`,
    /// kept in lockstep with [`TransactionTypeKind`]. Use this byte
    /// (via [`typedKind`] / [`isAssetLock`] / [`isAssetUnlock`]) to
    /// branch on transaction kind in UI code; the parallel
    /// [`transactionType`] string is human-readable only and not
    /// stable.
    ///
    /// Sentinel `0xFF` means "pre-feature row whose discriminant
    /// hasn't been populated yet" — SPV's next upsert round
    /// replaces it with the real discriminant on touch. Accessors
    /// treat the sentinel as unknown (no branch fires).
    public var transactionTypeKind: UInt8 = 0xFF
    /// Net Core amount in duffs across locally owned TXOs (positive=received, negative=sent).
    public var netAmount: Int64
    /// Set when removing a wallet left no authoritative amount for the survivor.
    /// `nil` preserves the accounting of rows migrated from V3.
    public var netAmountUnavailable: Bool? = nil
    /// Fee in duffs (nil if unknown).
    public var fee: UInt64?
    /// User-assigned label.
    public var label: String
    /// Timestamp when first observed (Unix seconds).
    public var firstSeen: UInt64

    // MARK: - Provider (masternode) special-transaction payload

    /// Fields lifted by the Rust FFI from a ProRegTx / ProUpServTx
    /// DIP-3 payload (see `provider_payload_fields` in
    /// `rs-platform-wallet-ffi`). All optional — populated only when
    /// [`typedKind`] is `.providerRegistration` / `.providerUpdateService`.
    /// The Swift side never decodes the payload; these are pure storage.
    ///
    /// Masternode service endpoint as `"ip:port"`.
    public var providerServiceAddress: String? = nil
    /// ProUpServTx `proTxHash` (32 raw wire bytes) linking the update to
    /// its registration. `nil` for ProRegTx (whose own txid is the
    /// proTxHash).
    public var providerProTxHash: Data? = nil
    /// ProRegTx collateral outpoint txid (32 raw wire bytes); pair with
    /// [`providerCollateralVout`]. `nil` when not a ProRegTx.
    public var providerCollateralTxid: Data? = nil
    public var providerCollateralVout: UInt32 = 0
    /// ProRegTx owner / voting key hashes (hash160, 20 bytes each).
    public var providerOwnerKeyHash: Data? = nil
    public var providerVotingKeyHash: Data? = nil

    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Transaction outputs created by this transaction.
    ///
    /// Cascade-deletes the matching `PersistentTxo` rows when the
    /// transaction is removed — outputs cannot meaningfully exist
    /// without their containing transaction (the outpoint, script,
    /// amount, and address are all derived from it).
    @Relationship(deleteRule: .cascade, inverse: \PersistentTxo.transaction)
    public var outputs: [PersistentTxo] = []

    /// Transaction outputs spent *by* this transaction.
    ///
    /// Inverse of `PersistentTxo.spendingTransaction`. Default
    /// `.nullify` delete rule (do not pass `.cascade`!) — those TXOs
    /// are owned by their *creating* transaction, not this one.
    /// Cascading from the spending side would let a recent tx wipe
    /// outputs of an older tx on delete: a data-loss bug. Removing
    /// this transaction merely detaches the spend-link and the TXOs
    /// flip back to "unspent" until something else claims them.
    @Relationship(inverse: \PersistentTxo.spendingTransaction)
    public var inputs: [PersistentTxo] = []

    /// Pending input outpoints — entries this transaction's input
    /// list references but for which no `PersistentTxo` has been
    /// upserted yet. Filled by `PlatformWalletPersistenceHandler.
    /// upsertTransaction` via the FFI's `input_outpoints` slice;
    /// each entry is consumed (deleted) by `upsertUtxo` when the
    /// matching previous-output finally arrives. See
    /// `PersistentPendingInput` for the full reconciliation flow.
    /// Cascade-delete: removing the spending tx drops every pending
    /// row that hasn't resolved yet.
    @Relationship(deleteRule: .cascade, inverse: \PersistentPendingInput.spendingTransaction)
    public var pendingInputs: [PersistentPendingInput] = []

    /// Every account whose changeset bucket carried this tx record.
    ///
    /// This is a **superset** of the TXO-derived membership: it
    /// includes payload-only involvement (special-tx payloads whose
    /// Provider Owner / Voting key addresses matched an account) where
    /// no `PersistentTxo` exists in the account, so the TXO join can
    /// never surface it. The persistence handler appends the matched
    /// account here for every record it upserts, mirroring how
    /// `WalletChangeSetFFI::from_changeset` buckets `cs.records` by
    /// `record.account_type` on the Rust side.
    ///
    /// The TXO join (`outputs` / `inputs` → `PersistentTxo.account`)
    /// remains the canonical path for **funds** — balances, spend
    /// tracking, per-address history all flow through it. This join
    /// exists only so payload-only involvement is representable at
    /// all; treat it as "account participation," not "account owns
    /// value in this tx."
    ///
    /// Inverse of `PersistentAccount.involvedTransactions`, declared
    /// on this side only (SwiftData needs the `inverse:` on exactly
    /// one end of a many-to-many pair). Default `.nullify` delete rule
    /// on both sides — deleting an account merely detaches it from the
    /// tx (and vice versa); neither end cascades, since the tx row is
    /// shared across accounts / wallets and the account outlives any
    /// single tx.
    @Relationship(inverse: \PersistentAccount.involvedTransactions)
    public var involvedAccounts: [PersistentAccount] = []

    public init(
        txid: Data,
        transactionData: Data,
        context: UInt32 = 0,
        blockHeight: UInt32 = 0,
        direction: UInt32 = 0,
        transactionType: String = "Standard",
        netAmount: Int64 = 0,
        firstSeen: UInt64 = 0
    ) {
        self.txid = txid
        self.transactionData = transactionData
        self.context = context
        self.blockHeight = blockHeight
        self.blockTimestamp = 0
        self.direction = direction
        self.transactionType = transactionType
        self.netAmount = netAmount
        self.firstSeen = firstSeen
        self.label = ""
        self.createdAt = Date()
        self.lastUpdated = Date()
    }

    // MARK: - Display Helpers

    /// Hex-encoded txid for UI / log sites. The on-disk row stores
    /// the raw 32 bytes in wire/internal order (matches what
    /// `dashcore::Txid::as_ref()` hands the FFI). The canonical
    /// Bitcoin/Dash display convention is the *reverse* of those
    /// bytes (the `Txid: Display` impl in dashcore-rust does the
    /// same flip), so block-explorer hex matches what users see
    /// here. Storage stays unflipped — predicate fetches compare
    /// wire-order `Data` directly without re-encoding.
    public var txidHex: String {
        txid.reversed().map { String(format: "%02x", $0) }.joined()
    }

    public var contextName: String {
        switch context {
        case 0: return "Mempool"
        case 1: return "InstantSend"
        case 2: return "In Block"
        case 3: return "Chain Locked"
        default: return "Unknown"
        }
    }

    /// Core value movement for one wallet; `nil` when it cannot be derived yet.
    ///
    /// The stored scalar is the last recording wallet's (Rust `net_amount`,
    /// or the reconciliation over its own TXOs), so it answers only when
    /// `walletId` is the sole participant. Pending inputs do not veto that
    /// answer: `upsertTransaction` writes one for every input whose prevout
    /// has no local TXO — every foreign input of an incoming payment — and
    /// never prunes them, so they cannot tell a late input of ours apart.
    public func netAmount(for walletId: Data) -> Int64? {
        func owned(_ rows: [PersistentTxo]) -> [PersistentTxo] {
            var seen = Set<Data>()
            return rows.filter {
                PlatformWalletPersistenceHandler.isWalletOwnedTxo($0)
                    && PlatformWalletPersistenceHandler.resolvedWalletId(of: $0) == walletId
                    && seen.insert($0.outpoint).inserted
            }
        }
        let hasUnownedTxos = (inputs + outputs).contains { !PlatformWalletPersistenceHandler.isWalletOwnedTxo($0) }
        if participatingWalletIds == [walletId], !hasUnownedTxos {
            return netAmountUnavailable == true ? nil : netAmount
        }
        // Computed from TXOs alone: a pending input this wallet recorded may be
        // one of its own still-unlinked coins, so the sum is only provisional.
        // TODO(wallet-scoped-accounting-from-rust): a foreign payment to two
        // local wallets leaves an unresolvable pending input per wallet, so both
        // show "Amount unavailable". Store Rust's per-wallet net amount instead
        // (tracked in #5226).
        guard !pendingInputs.contains(where: { $0.walletId == walletId }) else { return nil }
        let walletInputs = owned(inputs)
        let walletOutputs = owned(outputs)
        guard !walletInputs.isEmpty || !walletOutputs.isEmpty else { return nil }
        return Self.reconciledAccounting(
            inputs: walletInputs, ownedOutputAmounts: walletOutputs.map(\.amount),
            allOutputsOwned: false, previousDirection: direction, isAssetLock: isAssetLock
        )?.netAmount
    }

    /// Direction relative to one wallet for transactions shared by multiple local wallets.
    public func direction(for walletId: Data) -> UInt32 {
        guard participatingWalletIds.count > 1, direction != CoreDirectionCode.coinJoin,
              typedKind != .coinJoin else { return direction }
        let spendsOurs = inputs.contains {
            PlatformWalletPersistenceHandler.isWalletOwnedTxo($0)
                && PlatformWalletPersistenceHandler.resolvedWalletId(of: $0) == walletId
        }
        if isAssetLock {
            let ownedOutputs = outputs.filter(PlatformWalletPersistenceHandler.isWalletOwnedTxo)
            if spendsOurs {
                return ownedOutputs.contains {
                    PlatformWalletPersistenceHandler.resolvedWalletId(of: $0) != walletId
                } ? CoreDirectionCode.outgoing : direction
            }
            return ownedOutputs.contains {
                PlatformWalletPersistenceHandler.resolvedWalletId(of: $0) == walletId
            } ? CoreDirectionCode.incoming : direction
        }
        return spendsOurs ? CoreDirectionCode.outgoing : CoreDirectionCode.incoming
    }

    /// Retain the sole surviving wallet's accounting before ownership links disappear.
    func preserveAccounting(removingWallet walletId: Data) {
        let participants = participatingWalletIds
        guard participants.count == 2, participants.contains(walletId),
              let survivor = participants.first(where: { $0 != walletId }) else { return }
        let amount = netAmount(for: survivor)
        let survivorDirection = direction(for: survivor)
        if let amount { netAmount = amount }
        netAmountUnavailable = amount == nil
        direction = survivorDirection
    }

    /// Format the wallet's Core value movement in DASH.
    public func formattedAmount(for walletId: Data) -> String {
        guard let amount = netAmount(for: walletId) else { return "Amount unavailable" }
        return Self.format(duffs: amount)
    }

    /// Net amount for `walletId`, or the stored scalar when no wallet scope is given.
    public func displayNetAmount(for walletId: Data?) -> Int64? {
        walletId.map { netAmount(for: $0) } ?? (netAmountUnavailable == true ? nil : netAmount)
    }

    /// `CoreDirectionCode` for `walletId`, or the stored direction when no wallet scope is given.
    public func displayDirectionCode(for walletId: Data?) -> UInt32 {
        walletId.map { direction(for: $0) } ?? direction
    }

    /// Formatted amount for `walletId`, or the stored scalar's when no wallet scope is given.
    public func displayFormattedAmount(for walletId: Data?) -> String {
        walletId.map { formattedAmount(for: $0) } ?? formattedAmount
    }

    /// Signed DASH text for a duff amount; `magnitude` cannot trap on `Int64.min`.
    static func format(duffs: Int64) -> String {
        String(format: "%@%.8f DASH", duffs >= 0 ? "+" : "-", Double(duffs.magnitude) / 100_000_000)
    }

    /// Local wallets owning one of this transaction's TXOs or having recorded it
    /// (`involvedAccounts`), whose last writer's accounting is the stored scalar.
    private var participatingWalletIds: Set<Data> {
        let owning = (inputs + outputs).filter(PlatformWalletPersistenceHandler.isWalletOwnedTxo)
            .compactMap { PlatformWalletPersistenceHandler.resolvedWalletId(of: $0) }
        let recording = involvedAccounts.compactMap { account -> Data? in
            let wallet: PersistentWallet? = account.wallet
            return wallet?.walletId
        }
        return Set(owning + recording)
    }

    static func reconciledAccounting(
        inputs: [PersistentTxo], ownedOutputAmounts: [UInt64],
        allOutputsOwned: Bool, previousDirection: UInt32, isAssetLock: Bool
    ) -> (netAmount: Int64, direction: UInt32)? {
        func total(_ amounts: [UInt64]) -> Int64? {
            var sum: Int64 = 0
            for amount in amounts {
                guard let value = Int64(exactly: amount) else { return nil }
                let addition = sum.addingReportingOverflow(value)
                guard !addition.overflow else { return nil }
                sum = addition.partialValue
            }
            return sum
        }
        guard let received = total(ownedOutputAmounts), let spent = total(inputs.map(\.amount)) else {
            return nil
        }
        // Same rule as Rust (`platform_wallet::changeset::wallet_direction`):
        // internal only when nothing leaves the wallet and something stays in
        // it, or an asset lock burns into Platform. Both sides test one table.
        let direction: UInt32
        if previousDirection == CoreDirectionCode.coinJoin { direction = CoreDirectionCode.coinJoin }
        else if inputs.isEmpty { direction = CoreDirectionCode.incoming }
        else if allOutputsOwned && (!ownedOutputAmounts.isEmpty || isAssetLock) {
            direction = CoreDirectionCode.internalTransfer
        } else { direction = CoreDirectionCode.outgoing }
        return (received - spent, direction)
    }

    public var directionName: String {
        switch direction {
        case CoreDirectionCode.incoming: return "Incoming"
        case CoreDirectionCode.outgoing: return "Outgoing"
        case CoreDirectionCode.internalTransfer: return "Internal"
        case CoreDirectionCode.coinJoin: return "CoinJoin"
        default: return "Unknown"
        }
    }

    /// Typed view onto [`transactionTypeKind`]. `nil` only for the
    /// `0xFF` sentinel (pre-feature row not yet re-persisted by SPV)
    /// or for a future Rust-side variant addition Swift hasn't
    /// learned about yet — both treated as "unknown" by the
    /// `isAssetLock` / `isAssetUnlock` accessors so an unexpected
    /// byte never silently fires the wrong branch.
    public var typedKind: TransactionTypeKind? {
        TransactionTypeKind(rawValue: transactionTypeKind)
    }

    /// `true` when this transaction is a Dash Platform asset-lock
    /// funding tx — a Layer-1 burn that mints Layer-2 credits. The
    /// wallet's `direction` classifier reports `Internal` because the
    /// credit output is derived from this wallet's identity-funding
    /// account, but the *intent* is conversion to L2 credits, not
    /// "transaction to myself."
    public var isAssetLock: Bool {
        typedKind == .assetLock
    }

    /// Companion to [`isAssetLock`] — withdrawal back to L1.
    public var isAssetUnlock: Bool {
        typedKind == .assetUnlock
    }

    /// `true` for a masternode provider-registration (ProRegTx).
    public var isProviderRegistration: Bool {
        typedKind == .providerRegistration
    }

    /// `true` for a masternode provider-update-service (ProUpServTx).
    public var isProviderUpdateService: Bool {
        typedKind == .providerUpdateService
    }

    /// ProUpServTx proTxHash in block-explorer (reversed) hex, or `nil`.
    /// Matches [`txidHex`]'s display-order convention.
    public var providerProTxHashHex: String? {
        providerProTxHash.map { $0.reversed().map { String(format: "%02x", $0) }.joined() }
    }

    /// ProRegTx collateral outpoint as `"txidHex:vout"` in display order,
    /// or `nil` when there's no collateral field.
    public var providerCollateralDisplay: String? {
        guard let txid = providerCollateralTxid else { return nil }
        let hex = txid.reversed().map { String(format: "%02x", $0) }.joined()
        return "\(hex):\(providerCollateralVout)"
    }

    /// ProRegTx owner key hash (hash160) in hex — key hashes are shown
    /// in their natural forward byte order, unlike txids.
    public var providerOwnerKeyHashHex: String? {
        providerOwnerKeyHash.map { $0.map { String(format: "%02x", $0) }.joined() }
    }

    /// ProRegTx voting key hash (hash160) in forward-order hex.
    public var providerVotingKeyHashHex: String? {
        providerVotingKeyHash.map { $0.map { String(format: "%02x", $0) }.joined() }
    }

    /// `true` for masternode provider special transactions (ProRegTx
    /// and the three ProUp*Tx kinds). Like asset locks, these get
    /// classified `Internal` by the wallet's direction logic (the
    /// wallet only sees its own owner/voting/payout keys referenced
    /// in the payload), so direction-derived labels like
    /// "Self-Transfer" are misleading for them.
    public var isProviderSpecial: Bool {
        providerSpecialName != nil
    }

    /// Human-readable name for provider special transactions, `nil`
    /// for every other kind.
    public var providerSpecialName: String? {
        switch typedKind {
        case .providerRegistration: return "Provider Registration"
        case .providerUpdateRegistrar: return "Provider Update Registrar"
        case .providerUpdateService: return "Provider Update Service"
        case .providerUpdateRevocation: return "Provider Update Revocation"
        default: return nil
        }
    }

    /// Direction text for UI surfaces, overridden for asset-lock /
    /// asset-unlock txs (the L1 DASH isn't going "to myself" — it's
    /// being converted to / from L2 platform credits) and for
    /// provider special txs (the payload references our keys but no
    /// value moves "to myself").
    ///
    /// Use this anywhere a human-readable "what happened" label is
    /// needed; fall back to [`directionName`] only when the consumer
    /// genuinely needs the raw direction (e.g. the filter dropdown).
    public var displayDirection: String {
        if isAssetLock { return "Asset Lock" }
        if isAssetUnlock { return "Asset Unlock" }
        if let name = providerSpecialName { return name }
        return directionName
    }

    public var formattedAmount: String {
        netAmountUnavailable == true ? "Amount unavailable" : Self.format(duffs: netAmount)
    }
}

/// Wire values of `PersistentTransaction.direction`, matching `directionName`
/// and the FFI's `TransactionDirection` discriminants.
public enum CoreDirectionCode {
    public static let incoming: UInt32 = 0
    public static let outgoing: UInt32 = 1
    public static let internalTransfer: UInt32 = 2
    public static let coinJoin: UInt32 = 3
}

/// Typed mirror of Rust's
/// `key_wallet::transaction_checking::transaction_router::TransactionType`,
/// pinned to the `u8` discriminants emitted by
/// `transaction_type_to_u8` in `rs-platform-wallet-ffi`'s
/// `core_wallet_types.rs`. The Rust side has a comment requiring
/// any new variant to gain a Swift case here in the same commit;
/// the reverse holds too (Swift is the consumer's source of truth
/// for the discriminant byte).
///
/// Note: `transactionTypeKind == 0xFF` is the
/// "pre-feature / not-populated" sentinel and is NOT a case in this
/// enum — `TransactionTypeKind(rawValue: 0xFF)` returns `nil`, which
/// the accessors treat as "unknown" so no branch fires falsely.
public enum TransactionTypeKind: UInt8 {
    case standard = 0
    case coinJoin = 1
    case providerRegistration = 2
    case providerUpdateRegistrar = 3
    case providerUpdateService = 4
    case providerUpdateRevocation = 5
    case assetLock = 6
    case assetUnlock = 7
    case coinbase = 8
    case ignored = 9
}
