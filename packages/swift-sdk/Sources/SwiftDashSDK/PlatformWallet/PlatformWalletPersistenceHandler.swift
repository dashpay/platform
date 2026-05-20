import Foundation
import SwiftData
import DashSDKFFI

/// Bridges FFI persistence callbacks to SwiftData storage.
///
/// Allocated as a class so its pointer can be passed as the opaque `context`
/// to the Rust persistence callbacks. Must be retained for the lifetime of
/// the `PlatformWalletManager`.
public class PlatformWalletPersistenceHandler {
    let modelContainer: ModelContainer

    /// Network this handler's owning `PlatformWalletManager` is bound
    /// to. When set, `loadWalletList` filters out persisted wallets
    /// from other networks so a per-network manager only restores its
    /// own wallets. `nil` keeps the legacy "load every wallet"
    /// behavior for callers that don't yet thread network through —
    /// once the example app's `WalletManagerStore` is the only
    /// caller, the `nil` path can be retired.
    let network: Network?

    /// Background context for writing from callback threads.
    ///
    /// `ModelContext` is not thread-safe — touching it from the
    /// Tokio worker threads that drive the Rust persistence
    /// callbacks corrupts SwiftData's internal state and crashes
    /// inside `fetch`/`save`. The context is therefore confined to
    /// `serialQueue`: every public entry point wraps its body in
    /// `onQueue { … }`, and internal helpers (`upsertTransaction`,
    /// `markUtxoSpent`, …) assume they are already on the queue.
    private let backgroundContext: ModelContext

    /// Serial queue that owns `backgroundContext` and any other
    /// non-Sendable handler state (`loadAllocations`). All public
    /// entry points — both the FFI callback shims and the
    /// app-facing accessors — funnel through `onQueue` so the
    /// context is only ever touched on this queue.
    private let serialQueue = DispatchQueue(
        label: "org.dash.platform-wallet.persistence",
        qos: .userInitiated
    )

    /// True while inside a begin/end changeset bracket. When set,
    /// per-kind helpers skip their own `backgroundContext.save()` and
    /// let `endChangeset` commit (or rollback) the whole round
    /// atomically.
    private var inChangeset = false

    public init(modelContainer: ModelContainer, network: Network? = nil) {
        self.modelContainer = modelContainer
        self.network = network
        self.backgroundContext = ModelContext(modelContainer)
        self.backgroundContext.autosaveEnabled = true
    }

    /// Synchronously run `body` on `serialQueue`.
    ///
    /// All public methods that read or write `backgroundContext`
    /// (or `loadAllocations`) must call through this helper.
    /// `sync` matches the synchronous FFI contract — the C shims
    /// need a return value before yielding back to Rust — and
    /// turns the queue into the handler's de-facto actor: only
    /// one thread runs SwiftData operations at a time.
    ///
    /// Do not call `onQueue` from another method that already
    /// runs on the queue; `DispatchQueue.sync` will deadlock on
    /// recursive entry. The internal helpers in this file all
    /// assume they are already on the queue and call
    /// `backgroundContext` directly.
    private func onQueue<T>(_ body: () throws -> T) rethrows -> T {
        try serialQueue.sync(execute: body)
    }

    // MARK: - Platform Address Balances

    /// Apply an incremental BLAST balance changeset to SwiftData.
    ///
    /// BLAST sync identifies each address by its 20-byte
    /// `addressHash`. `PersistentPlatformAddress` rows are seeded by
    /// the address-emit path (`persistAccountAddresses` for
    /// PlatformPayment accounts), which knows the full DIP-0018
    /// bech32m form plus derivation metadata. This callback only
    /// refreshes the volatile fields (balance, nonce, `isUsed`). If
    /// BLAST reports balances for an address we never emitted (e.g.
    /// cache wipe between runs), we skip it — the next
    /// address-emit pass will bring the row back and the next sync
    /// will fill in the balance.
    func persistAddressBalances(
        walletId: Data,
        entries: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32)]
    ) {
        onQueue {
            for (_, addressHash, balance, nonce, accountIndex, addressIndex) in entries {
                let descriptor = FetchDescriptor<PersistentPlatformAddress>(
                    predicate: #Predicate { $0.addressHash == addressHash }
                )
                guard let existing = try? backgroundContext.fetch(descriptor).first else {
                    continue
                }
                existing.accountIndex = accountIndex
                existing.addressIndex = addressIndex
                existing.balance = balance
                existing.nonce = nonce
                if balance > 0 || nonce > 0 {
                    existing.isUsed = true
                }
                existing.lastUpdated = Date()
            }

            // No save() here — this handler runs inside the Rust-side
            // changeset round, which is bracketed by changesetBegin /
            // changesetEnd; the atomic save fires in endChangeset.
        }
    }

    // MARK: - Asset locks

    /// Apply an `AssetLockChangeSet` projection to SwiftData.
    ///
    /// The Rust-side asset-lock manager emits a changeset on every
    /// status transition (`Built → Broadcast → InstantSendLocked →
    /// ChainLocked`) and on consumption (the registration flow drops
    /// the row once the IdentityCreate state transition lands). Each
    /// `upsert` maps onto a `PersistentAssetLock` row keyed by
    /// `outPointHex` (the 36-byte outpoint encoded as
    /// `<txid_display_hex>:<vout>`); each `removed` entry deletes the
    /// matching row. `RegistrationProgressView` watches these rows
    /// via `@Query` to drive the stage progress bar.
    ///
    /// No `save()` here — bracketed by `beginChangeset` /
    /// `endChangeset` from the Rust `store()` round.
    func persistAssetLocks(
        walletId: Data,
        upserts: [AssetLockEntrySnapshot],
        removed: [Data]
    ) {
        onQueue {
            for entry in upserts {
                let outPointHex = entry.outPointHex
                let descriptor = FetchDescriptor<PersistentAssetLock>(
                    predicate: #Predicate { $0.outPointHex == outPointHex }
                )
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    existing.walletId = walletId
                    existing.transactionBytes = entry.transactionBytes
                    existing.fundingTypeRaw = entry.fundingTypeRaw
                    existing.identityIndexRaw = entry.identityIndexRaw
                    existing.accountIndexRaw = entry.accountIndexRaw
                    existing.amountDuffs = entry.amountDuffs
                    existing.statusRaw = entry.statusRaw
                    existing.proofBytes = entry.proofBytes
                    existing.updatedAt = Date()
                } else {
                    let record = PersistentAssetLock(
                        outPointHex: outPointHex,
                        walletId: walletId,
                        transactionBytes: entry.transactionBytes,
                        fundingTypeRaw: entry.fundingTypeRaw,
                        identityIndexRaw: entry.identityIndexRaw,
                        accountIndexRaw: entry.accountIndexRaw,
                        amountDuffs: entry.amountDuffs,
                        statusRaw: entry.statusRaw,
                        proofBytes: entry.proofBytes
                    )
                    backgroundContext.insert(record)
                }
            }

            for outPointHex in removed {
                let hex = PersistentAssetLock.encodeOutPoint(rawBytes: outPointHex)
                let descriptor = FetchDescriptor<PersistentAssetLock>(
                    predicate: #Predicate { $0.outPointHex == hex }
                )
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    backgroundContext.delete(existing)
                }
            }
        }
    }

    /// Load all persisted tracked asset locks for a wallet — used by
    /// the wallet load path to rebuild `unused_asset_locks` on the
    /// Rust side so an in-flight registration that was interrupted by
    /// an app kill can resume from the latest status without
    /// rebroadcasting the asset-lock transaction.
    public func loadCachedAssetLocks(walletId: Data) -> [AssetLockEntrySnapshot] {
        onQueue { loadCachedAssetLocksOnQueue(walletId: walletId) }
    }

    /// On-queue implementation reused by the load-wallet-list path
    /// without re-entering `onQueue`.
    func loadCachedAssetLocksOnQueue(walletId: Data) -> [AssetLockEntrySnapshot] {
        let descriptor = FetchDescriptor<PersistentAssetLock>(
            predicate: PersistentAssetLock.predicate(walletId: walletId)
        )
        guard let records = try? backgroundContext.fetch(descriptor) else {
            return []
        }
        return records.map { record in
            AssetLockEntrySnapshot(
                outPointHex: record.outPointHex,
                transactionBytes: record.transactionBytes,
                fundingTypeRaw: record.fundingTypeRaw,
                identityIndexRaw: record.identityIndexRaw,
                accountIndexRaw: record.accountIndexRaw,
                amountDuffs: record.amountDuffs,
                statusRaw: record.statusRaw,
                proofBytes: record.proofBytes
            )
        }
    }

    /// Owned snapshot of an `AssetLockEntryFFI` row. Same lifetime
    /// rationale as `IdentityEntrySnapshot` — the callback copies
    /// every byte buffer into owned `Data` before invoking the
    /// handler, so the handler runs against pure-Swift values
    /// regardless of when the Rust-side allocation gets reclaimed.
    public struct AssetLockEntrySnapshot {
        public let outPointHex: String
        public let transactionBytes: Data
        public let fundingTypeRaw: Int
        public let identityIndexRaw: Int32
        public let accountIndexRaw: Int32
        public let amountDuffs: Int64
        public let statusRaw: Int
        public let proofBytes: Data?
    }

    /// Load all cached platform-address balances for a wallet. Tuple
    /// shape matches the Rust-side `AddressBalanceEntryFFI` layout so
    /// the load-wallet-list path can re-seed the provider on startup
    /// without a full rescan.
    public func loadCachedBalances(walletId: Data) -> [(UInt8, [UInt8], UInt64, UInt32, UInt32, UInt32)] {
        onQueue { loadCachedBalancesOnQueue(walletId: walletId) }
    }

    /// Implementation for `loadCachedBalances` that assumes it is
    /// already running on `serialQueue`. Lets internal on-queue
    /// callers (`loadWalletList`) reuse the body without recursing
    /// through `onQueue`, which would deadlock.
    private func loadCachedBalancesOnQueue(walletId: Data) -> [(UInt8, [UInt8], UInt64, UInt32, UInt32, UInt32)] {
        let descriptor = FetchDescriptor<PersistentPlatformAddress>(
            predicate: PersistentPlatformAddress.predicate(walletId: walletId)
        )

        guard let records = try? backgroundContext.fetch(descriptor) else {
            return []
        }

        return records.map { record in
            (
                record.addressType,
                Array(record.addressHash),
                record.balance,
                record.nonce,
                record.accountIndex,
                record.addressIndex
            )
        }
    }

    // MARK: - Sync State

    /// Upsert sync state into SwiftData.
    ///
    /// The BLAST watermark is network-scoped, not wallet-scoped: every
    /// wallet on the same network shares one merged checkpoint.
    func persistSyncState(
        walletId: Data,
        syncHeight: UInt64,
        syncTimestamp: UInt64,
        lastKnownRecentBlock: UInt64
    ) {
        onQueue {
            guard let network = walletNetwork(walletId: walletId) else {
                return
            }
            let scopeId = syncStateScopeId(for: network)
            let descriptor = FetchDescriptor<PersistentPlatformAddressesSyncState>(
                predicate: #Predicate { $0.walletId == scopeId }
            )

            if let existing = try? backgroundContext.fetch(descriptor).first {
                existing.network = network
                existing.syncHeight = syncHeight
                existing.syncTimestamp = syncTimestamp
                existing.lastKnownRecentBlock = lastKnownRecentBlock
                existing.lastUpdated = Date()
            } else {
                let record = PersistentPlatformAddressesSyncState(
                    walletId: scopeId,
                    network: network,
                    syncHeight: syncHeight,
                    syncTimestamp: syncTimestamp,
                    lastKnownRecentBlock: lastKnownRecentBlock
                )
                backgroundContext.insert(record)
            }
            // No save() — bracketed by changesetBegin/End from the
            // Rust store() round.
        }
    }

    /// Load cached sync state for a wallet's network.
    public func loadCachedSyncState(walletId: Data) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        onQueue {
            guard let network = walletNetwork(walletId: walletId) else {
                return nil
            }
            return loadCachedSyncStateOnQueue(network: network)
        }
    }

    /// Load cached sync state for a specific network.
    public func loadCachedSyncState(network: Network) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        onQueue { loadCachedSyncStateOnQueue(network: network) }
    }

    /// Implementation for `loadCachedSyncState` that assumes it is
    /// already running on `serialQueue`. Both public overloads
    /// route through this so the `(walletId:)` variant can resolve
    /// the network and read the row in a single queue hop without
    /// recursing into `onQueue`, which would deadlock.
    private func loadCachedSyncStateOnQueue(network: Network) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        let scopeId = syncStateScopeId(for: network)
        let descriptor = FetchDescriptor<PersistentPlatformAddressesSyncState>(
            predicate: #Predicate { $0.walletId == scopeId }
        )

        guard let record = try? backgroundContext.fetch(descriptor).first else {
            return nil
        }

        return (record.syncHeight, record.syncTimestamp, record.lastKnownRecentBlock)
    }

    // MARK: - Wallet Changeset (transactions, utxos, accounts, balance, chain)

    /// Apply a full `WalletChangeSetFFI` to SwiftData.
    ///
    /// Called from the Rust persister when an SPV round produces core-
    /// wallet state changes. Upserts PersistentAccount / Transaction /
    /// Utxo records so views observing via `@Query` update automatically.
    func persistWalletChangeset(walletId: Data, changeset: UnsafePointer<WalletChangeSetFFI>) {
        onQueue {
            guard let wallet = findWalletRecord(walletId: walletId) else { return }
            let cs = changeset.pointee

            // Chain update.
            if cs.has_chain {
                if cs.chain.has_synced_height {
                    wallet.syncedHeight = cs.chain.synced_height
                }
                wallet.lastUpdated = Date()
            }

            // Persisted `last_applied_chain_lock` — bincode bytes
            // from the FFI carry the wallet's
            // `WalletMetadata::last_applied_chain_lock` snapshot for
            // restart roundtrip. Stored as opaque `Data` (decoded on
            // the Rust load side); SPV persists its own
            // `best_chainlock` independently so this column is the
            // wallet-side mirror, not a duplicate of SPV state.
            // Pre-feature rows / wallets that have never observed a
            // ChainLock carry `null` from Rust and stay `nil` here.
            if cs.last_applied_chain_lock_bytes_len > 0,
               let clPtr = cs.last_applied_chain_lock_bytes {
                let bytes = Data(
                    bytes: clPtr,
                    count: Int(cs.last_applied_chain_lock_bytes_len)
                )
                wallet.lastAppliedChainLockBytes = bytes
                wallet.lastUpdated = Date()
            }

            // Balance delta — Rust still emits per-round deltas, but the
            // PersistentWallet `balance*` fields they used to update were
            // removed (canonical source is now the in-memory account
            // totals via `walletManager.accountBalances(for:)`). Bump the
            // updated timestamp so the row reflects the persistence round
            // and discard the payload itself.
            if cs.has_balance {
                wallet.lastUpdated = Date()
            }

            // Per-account: transactions, UTXOs, pool state.
            if cs.accounts_count > 0, let accountsPtr = cs.accounts {
                for i in 0..<Int(cs.accounts_count) {
                    let acc = accountsPtr[i]
                    applyAccountChangeset(walletRecord: wallet, acc: acc)
                }
            }

            // No save() — bracketed by changesetBegin/End.
        }
    }

    /// Find or create the `PersistentWallet` row for `walletId`.
    /// Used only by `persistWalletMetadata`; every other write path
    /// fetches via `findWalletRecord` and drops on missing so that
    /// stale post-deletion callbacks can't resurrect a wiped wallet.
    private func ensureWalletRecord(walletId: Data) -> PersistentWallet {
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        if let existing = try? backgroundContext.fetch(descriptor).first {
            return existing
        }
        let record = PersistentWallet(walletId: walletId, network: nil)
        backgroundContext.insert(record)
        return record
    }

    /// Find the `PersistentWallet` row for `walletId`. Returns `nil`
    /// when no row exists.
    private func findWalletRecord(walletId: Data) -> PersistentWallet? {
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        return try? backgroundContext.fetch(descriptor).first
    }

    /// Look up a `PersistentWallet` to hang on
    /// `PersistentIdentity.wallet`. Non-creating — returns `nil` if
    /// no row exists (an identity may arrive before its owning
    /// wallet row under weird restore orderings) or if the caller
    /// passed `nil`. Kept separate from `ensureWalletRecord` so a
    /// stray identity upsert never creates a placeholder wallet.
    private func fetchWalletForLink(walletId: Data?) -> PersistentWallet? {
        guard let walletId else { return nil }
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        return try? backgroundContext.fetch(descriptor).first
    }

    /// Apply a single account changeset to SwiftData.
    private func applyAccountChangeset(
        walletRecord: PersistentWallet,
        acc: AccountChangeSetFFI
    ) {
        let accountIndex = acc.account_index
        // Stable account-type discriminants from the FFI. Used as the
        // upsert key so a load-path emit and a sync-path emit for the
        // same account collapse onto a single row — the legacy
        // `account_type_name` string was Rust's `Debug` output, which
        // differs from the canonical name the load path emits ("BIP44
        // Account" vs "Standard { index: 0, … }") and made the
        // string-keyed predicate produce duplicate rows.
        // `AccountTypeTagFFI` / `StandardAccountTypeTagFFI` come over
        // as plain `UInt8` aliases (cbindgen flat-enum projection).
        let typeTag = UInt32(acc.type_tag)
        let standardTag = UInt8(acc.standard_tag)
        let registrationIndex = acc.registration_index
        let keyClass = acc.key_class
        let userIdentityId = withUnsafeBytes(of: acc.user_identity_id) { Data($0) }
        let friendIdentityId = withUnsafeBytes(of: acc.friend_identity_id) { Data($0) }
        let typeName = accountTypeName(for: acc.type_tag, standardTag: acc.standard_tag)

        // Upsert keyed by the full account identity. We can't easily
        // express the identity tuple in a #Predicate with local `Data`
        // captures, so fetch by (walletId, accountType, accountIndex)
        // and verify the richer fields in Swift — same pattern the
        // load path uses for `applyAccountSpec`.
        let walletId = walletRecord.walletId
        let accountDescriptor = FetchDescriptor<PersistentAccount>(
            predicate: #Predicate {
                $0.wallet.walletId == walletId
                    && $0.accountType == typeTag
                    && $0.accountIndex == accountIndex
            }
        )
        let existing = (try? backgroundContext.fetch(accountDescriptor)) ?? []
        let match = existing.first { row in
            row.standardTag == standardTag
                && row.registrationIndex == registrationIndex
                && row.keyClass == keyClass
                && row.userIdentityId == userIdentityId
                && row.friendIdentityId == friendIdentityId
        }
        let account: PersistentAccount
        if let match = match {
            account = match
            account.lastUpdated = Date()
        } else {
            account = PersistentAccount(
                wallet: walletRecord,
                accountType: typeTag,
                accountIndex: accountIndex,
                accountTypeName: typeName
            )
            backgroundContext.insert(account)
        }
        // Refresh the variant-specific fields so the row stays in
        // sync with the latest emit (matches the load-path apply).
        account.standardTag = standardTag
        account.registrationIndex = registrationIndex
        account.keyClass = keyClass
        account.userIdentityId = userIdentityId
        account.friendIdentityId = friendIdentityId

        // Highest-used address pool indices.
        if acc.has_external_highest_used {
            account.externalHighestUsed = acc.external_highest_used
        }
        if acc.has_internal_highest_used {
            account.internalHighestUsed = acc.internal_highest_used
        }

        // Transactions.
        if acc.transactions_count > 0, let txsPtr = acc.transactions {
            for i in 0..<Int(acc.transactions_count) {
                upsertTransaction(account: account, tx: txsPtr[i])
            }
        }

        // UTXOs added.
        if acc.utxos_added_count > 0, let utxosPtr = acc.utxos_added {
            for i in 0..<Int(acc.utxos_added_count) {
                upsertUtxo(account: account, utxo: utxosPtr[i])
            }
        }

        // UTXOs spent — mark them spent (keep for history).
        if acc.utxos_spent_count > 0, let spentPtr = acc.utxos_spent {
            for i in 0..<Int(acc.utxos_spent_count) {
                markUtxoSpent(spentPtr[i])
            }
        }

        // UTXOs became InstantSend-locked — update flag.
        if acc.utxos_instant_locked_count > 0, let ilPtr = acc.utxos_instant_locked {
            for i in 0..<Int(acc.utxos_instant_locked_count) {
                markUtxoInstantLocked(ilPtr[i])
            }
        }
    }

    private func upsertTransaction(account: PersistentAccount, tx: TransactionRecordFFI) {
        // The `account` parameter scopes the wallet-id used for the
        // input-reconciliation pass at the bottom of this method.
        // The transaction row itself stays account-agnostic — a
        // single tx can land in multiple accounts (or wallets), and
        // per-wallet membership is recovered through the TXO graph
        // (`outputs` / `inputs`) rather than a denormalized column.
        let resolvedWalletId: Data = account.wallet.walletId
        let txidData = hashData(tx.txid)
        let descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == txidData }
        )

        // The FFI projection always serializes the transaction body
        // (`dashcore::consensus::encode::serialize` upstream), so
        // `tx.tx_data` is non-null and `tx.tx_data_len > 0` in
        // practice. Fall back to empty `Data()` only as a defensive
        // guard against a future projection change — the
        // persister-fallback read path treats empty bytes as miss
        // (the Rust side can't decode an empty consensus buffer).
        let transactionData: Data = {
            guard let dataPtr = tx.tx_data, tx.tx_data_len > 0 else { return Data() }
            return Data(bytes: dataPtr, count: Int(tx.tx_data_len))
        }()

        let record: PersistentTransaction
        if let existing = try? backgroundContext.fetch(descriptor).first {
            record = existing
        } else {
            record = PersistentTransaction(
                txid: txidData,
                transactionData: transactionData,
                context: tx.context,
                blockHeight: tx.block_height,
                direction: tx.direction,
                transactionType: tx.transaction_type.map { String(cString: $0) } ?? "Standard",
                netAmount: tx.net_amount,
                firstSeen: tx.first_seen
            )
            backgroundContext.insert(record)
        }

        record.context = tx.context
        record.blockHeight = tx.block_height
        record.blockTimestamp = tx.block_timestamp
        let blockHashBytes = hashData(tx.block_hash)
        record.blockHash = blockHashBytes.allSatisfy { $0 == 0 } ? nil : blockHashBytes
        record.direction = tx.direction
        if let typeName = tx.transaction_type {
            record.transactionType = String(cString: typeName)
        }
        record.transactionTypeKind = tx.transaction_type_kind
        record.netAmount = tx.net_amount
        record.fee = tx.has_fee ? tx.fee : nil
        if let labelPtr = tx.label {
            record.label = String(cString: labelPtr)
        }
        record.firstSeen = tx.first_seen
        record.transactionData = transactionData
        record.lastUpdated = Date()

        // Walk every input in this transaction and reconcile it
        // against the `PersistentTxo` table. The FFI populates
        // `input_outpoints` from `tx.input.iter()` directly, so the
        // list survives even when the wallet's in-memory `self.utxos`
        // didn't classify the input as ours at processing time —
        // that gap was the silent-drop path that left
        // `PersistentTxo.isSpent` stuck at false on out-of-order
        // arrival. For each input outpoint:
        //   1. Look up the matching `PersistentTxo`.
        //   2. If found → set `isSpent` and link `spendingTransaction`.
        //   3. If not found → write a `PersistentPendingInput` row;
        //      the matching `upsertUtxo` will pick it up later and
        //      delete the pending row in the same pass.
        if let inPtr = tx.input_outpoints, tx.input_outpoints_count > 0 {
            for i in 0..<Int(tx.input_outpoints_count) {
                let entry = inPtr[i]
                let prevTxid = withUnsafeBytes(of: entry.txid) { Data($0) }
                let outpoint = PersistentTxo.makeOutpoint(txid: prevTxid, vout: entry.vout)
                resolveInputOutpoint(
                    outpoint: outpoint,
                    inputIndex: UInt32(i),
                    spendingTransaction: record,
                    spendingTxid: txidData,
                    walletId: resolvedWalletId
                )
            }
        }
    }

    /// Mark the `PersistentTxo` whose 36-byte `outpoint` matches the
    /// given input as spent and link it to `spendingTransaction`.
    /// If no matching TXO exists yet (in-Swift out-of-order, or
    /// load_from_persistor missed it), write a
    /// `PersistentPendingInput` row so the next `upsertUtxo` for
    /// that outpoint can resolve the linkage.
    private func resolveInputOutpoint(
        outpoint: Data,
        inputIndex: UInt32,
        spendingTransaction: PersistentTransaction,
        spendingTxid: Data,
        walletId: Data
    ) {
        let txoDescriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        if let txo = try? backgroundContext.fetch(txoDescriptor).first {
            // Only touch the row if the linkage actually changes —
            // an idempotent re-upsert of the same tx must not
            // gratuitously bump `lastUpdated` and trigger a follow-on
            // changeset emit.
            let linkageChanged =
                !txo.isSpent
                || txo.spendingTransaction?.txid != spendingTxid
                || txo.spendingInputIndex != inputIndex
            if linkageChanged {
                txo.isSpent = true
                if txo.spendingTransaction?.txid != spendingTxid {
                    txo.spendingTransaction = spendingTransaction
                }
                // Capture the canonical vin index so the detail
                // view can render inputs in serialized order.
                txo.spendingInputIndex = inputIndex
                txo.lastUpdated = Date()
            }
            // A pending entry from an earlier write is now stale —
            // resolved by this fetch. Drop it.
            removePendingInputs(for: outpoint)
        } else {
            // Defer: record a pending row so a future `upsertUtxo`
            // can complete the link. Writing one row per input is
            // cheap; the cascade-delete relationship + the resolve
            // path in `upsertUtxo` keep the table from growing
            // unbounded.
            //
            // Skip the write if a pending row for this exact
            // (outpoint, spending-tx) pair already exists — re-upserts
            // of the same transaction would otherwise produce
            // duplicate pending rows that all resolve to the same
            // TXO, wasting fetch work on the resolve side.
            let pendingDescriptor = FetchDescriptor<PersistentPendingInput>(
                predicate: #Predicate { $0.outpoint == outpoint && $0.spendingTxid == spendingTxid }
            )
            if (try? backgroundContext.fetch(pendingDescriptor).first) == nil {
                let pending = PersistentPendingInput(
                    outpoint: outpoint,
                    inputIndex: inputIndex,
                    spendingTxid: spendingTxid,
                    spendingTransaction: spendingTransaction,
                    walletId: walletId
                )
                backgroundContext.insert(pending)
            }
        }
    }

    /// Drop every `PersistentPendingInput` row keyed on `outpoint`.
    /// Called after a successful `PersistentTxo` mark-spent so the
    /// pending entries don't linger as orphans, and from
    /// `upsertUtxo`'s resolve path so a freshly-arrived TXO doesn't
    /// keep its corresponding pending row alive.
    private func removePendingInputs(for outpoint: Data) {
        let descriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        guard let rows = try? backgroundContext.fetch(descriptor), !rows.isEmpty else {
            return
        }
        for row in rows {
            backgroundContext.delete(row)
        }
    }

    private func upsertUtxo(account: PersistentAccount, utxo: UtxoEntryFFI) {
        // Pull the per-account wallet id once. Used both for the new
        // `PersistentTxo.walletId` denorm (so per-wallet predicates
        // can hit a single column) and for stub-tx routing below.
        let resolvedWalletId: Data = account.wallet.walletId

        let txidData = hashData(utxo.outpoint.txid)
        let outpoint = PersistentTxo.makeOutpoint(txid: txidData, vout: utxo.outpoint.vout)
        let descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        let record: PersistentTxo
        if let existing = try? backgroundContext.fetch(descriptor).first {
            record = existing
            // Backfill if the account or wallet linkage is missing —
            // the per-wallet query path filters on TXO.walletId, so
            // an empty value would silently hide the row.
            if record.account == nil { record.account = account }
            if record.walletId.isEmpty, !resolvedWalletId.isEmpty {
                record.walletId = resolvedWalletId
            }
        } else {
            // Look up the containing transaction. Upstream sends the
            // transaction record before its TXOs in the same flush,
            // so it should already be in the context. If not, create
            // a stub keyed by txid so the cascade-delete invariant
            // (TXO cannot exist without its creating transaction)
            // holds; the real record will overwrite the stub when it
            // arrives. Note we no longer set `parentTx.account` —
            // transactions don't carry account linkage anymore (they
            // can span multiple accounts).
            let txDescriptor = FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == txidData }
            )
            let parentTx: PersistentTransaction
            if let existingTx = try? backgroundContext.fetch(txDescriptor).first {
                parentTx = existingTx
            } else {
                // Stub row — `transactionData` is left as empty
                // `Data()` on purpose. The real upsert (which has the
                // tx bytes) overwrites every field including
                // `transactionData` when it arrives. An orphaned
                // stub (real upsert never lands) reads back as empty
                // bytes, which the persister-fallback decode path
                // treats as miss.
                parentTx = PersistentTransaction(txid: txidData, transactionData: Data())
                backgroundContext.insert(parentTx)
            }

            let script: Data = {
                guard let p = utxo.script_pubkey, utxo.script_pubkey_len > 0 else { return Data() }
                return Data(bytes: p, count: Int(utxo.script_pubkey_len))
            }()
            let addressStr = utxo.address.map { String(cString: $0) } ?? ""
            record = PersistentTxo(
                transaction: parentTx,
                vout: utxo.outpoint.vout,
                amount: utxo.amount,
                address: addressStr,
                scriptPubKey: script,
                height: utxo.height
            )
            record.account = account
            record.walletId = resolvedWalletId
            backgroundContext.insert(record)
        }

        record.amount = utxo.amount
        record.height = utxo.height
        record.isCoinbase = utxo.is_coinbase
        record.isConfirmed = utxo.is_confirmed
        record.isInstantLocked = utxo.is_instantlocked
        record.isLocked = utxo.is_locked
        record.lastUpdated = Date()

        // Attach the `PersistentCoreAddress` row, if we have one. The
        // address-emit pass typically runs ahead of the SPV-utxo pass
        // within a flush, so the row should exist; if it doesn't (TXO
        // paid to an address outside our pool, or out-of-order flush),
        // leave the relationship nil — `record.address` stays as the
        // authoritative identifier.
        if record.coreAddress == nil, !record.address.isEmpty {
            let addressLookup = record.address
            let coreAddressDescriptor = FetchDescriptor<PersistentCoreAddress>(
                predicate: #Predicate { $0.address == addressLookup }
            )
            if let coreAddr = try? backgroundContext.fetch(coreAddressDescriptor).first {
                record.coreAddress = coreAddr
            }
        }

        // Resolve any deferred spend signal that landed before this
        // TXO existed. `upsertTransaction` writes a
        // `PersistentPendingInput` row for every input outpoint
        // whose previous-output isn't in SwiftData yet; the matching
        // upsert here drains those rows and stamps `isSpent` on the
        // TXO. Symmetric with the resolve path in
        // `upsertTransaction`, so the spend signal is order-
        // independent at this layer regardless of which side arrives
        // first.
        let outpointKey = record.outpoint
        let pendingDescriptor = FetchDescriptor<PersistentPendingInput>(
            predicate: #Predicate { $0.outpoint == outpointKey }
        )
        if let pendingRows = try? backgroundContext.fetch(pendingDescriptor),
           !pendingRows.isEmpty {
            // Pick the freshest pending entry — under normal sync
            // there's only one, but a chain reorg or double-spend
            // observation could leave multiple. Newest wins so the
            // visible spendingTransaction matches the most recent
            // observation; the rest are dropped.
            let chosen = pendingRows.max(by: { $0.createdAt < $1.createdAt }) ?? pendingRows[0]
            record.isSpent = true
            // Carry the vin index forward so the spending tx's
            // detail view can render its inputs in the canonical
            // serialized order. Same source as the linkage write
            // in `resolveInputOutpoint` — the only path that creates
            // pending rows captures the index from FFI's
            // `input_outpoints` slice, which mirrors `tx.input.iter()`.
            record.spendingInputIndex = chosen.inputIndex
            if let spending = chosen.spendingTransaction {
                if record.spendingTransaction?.txid != spending.txid {
                    record.spendingTransaction = spending
                }
            } else {
                // Pending row's parent tx wasn't faulted in; fall
                // back to a txid lookup so the linkage still lands.
                let spendingTxid = chosen.spendingTxid
                let txDescriptor = FetchDescriptor<PersistentTransaction>(
                    predicate: #Predicate { $0.txid == spendingTxid }
                )
                if let spending = try? backgroundContext.fetch(txDescriptor).first,
                   record.spendingTransaction?.txid != spending.txid {
                    record.spendingTransaction = spending
                }
            }
            record.lastUpdated = Date()
            for row in pendingRows {
                backgroundContext.delete(row)
            }
        }
    }

    private func markUtxoSpent(_ entry: SpentOutPointFFI) {
        let outpoint = PersistentTxo.makeOutpoint(
            txid: hashData(entry.outpoint.txid),
            vout: entry.outpoint.vout
        )
        let descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        guard let txo = try? backgroundContext.fetch(descriptor).first else {
            return
        }
        txo.isSpent = true
        // Link the spending transaction. The FFI now carries
        // `spending_txid` alongside the outpoint (the txid of the
        // `TransactionRecord` whose inputs included this outpoint),
        // so we can resolve the parent and set the relationship.
        // If the spending tx hasn't landed in SwiftData yet (rare
        // — same-flush ordering normally upserts the tx before
        // its spent-outpoint emit) leave the relationship nil; the
        // next flush carrying that tx triggers another upsert
        // round and eventually catches up.
        let spendingTxid = hashData(entry.spending_txid)
        if !spendingTxid.isEmpty,
           !spendingTxid.allSatisfy({ $0 == 0 }),
           txo.spendingTransaction?.txid != spendingTxid {
            let txDescriptor = FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == spendingTxid }
            )
            if let spendingTx = try? backgroundContext.fetch(txDescriptor).first {
                txo.spendingTransaction = spendingTx
            }
        }
        txo.lastUpdated = Date()
        // The spend signal landed both via the legacy
        // `utxos_spent` slice (this path) and — assuming the
        // spending tx's record was emitted in the same flush —
        // through `upsertTransaction`'s reconciliation pass. Both
        // resolve to the same TXO row, but the latter may have
        // written a `PersistentPendingInput` row when the TXO
        // didn't yet exist. Drain any leftover pending rows for
        // this outpoint so they don't linger as orphans.
        removePendingInputs(for: outpoint)
    }

    private func markUtxoInstantLocked(_ op: OutPointFFI) {
        let outpoint = PersistentTxo.makeOutpoint(txid: hashData(op.txid), vout: op.vout)
        let descriptor = FetchDescriptor<PersistentTxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        if let txo = try? backgroundContext.fetch(descriptor).first {
            txo.isInstantLocked = true
            txo.lastUpdated = Date()
        }
    }

    private func addDelta(_ base: UInt64, _ delta: Int64) -> UInt64 {
        if delta >= 0 {
            return base.addingReportingOverflow(UInt64(delta)).0
        }
        let sub = UInt64(-delta)
        return base >= sub ? base - sub : 0
    }

    // MARK: - Callbacks

    /// Build `PersistenceCallbacks` that point to this handler.
    ///
    /// The returned struct must not outlive `self`.
    func makeCallbacks() -> PersistenceCallbacks {
        let contextPtr = Unmanaged.passUnretained(self).toOpaque()
        var cb = PersistenceCallbacks()
        cb.context = contextPtr
        cb.on_changeset_begin_fn = changesetBeginCallback
        cb.on_changeset_end_fn = changesetEndCallback
        cb.on_persist_address_balances_fn = persistAddressBalancesCallback
        cb.on_persist_wallet_changeset_fn = persistWalletChangesetCallback
        cb.on_persist_sync_state_fn = persistSyncStateCallback
        // `on_persist_wallet_root_xpub_fn` intentionally unassigned.
        // Root xpub is redundant with `wallet_id` for identity /
        // verification; Rust-side will stop requiring it once the
        // upstream rust-dashcore PR lands.
        cb.on_persist_account_registrations_fn = persistAccountRegistrationsCallback
        cb.on_load_wallet_list_fn = loadWalletListCallback
        cb.on_load_wallet_list_free_fn = loadWalletListFreeCallback
        cb.on_persist_wallet_metadata_fn = persistWalletMetadataCallback
        cb.on_persist_account_address_pools_fn = persistAccountAddressPoolsCallback
        cb.on_persist_identities_fn = persistIdentitiesCallback
        cb.on_persist_identity_keys_fn = persistIdentityKeysCallback
        cb.on_persist_token_balances_fn = persistTokenBalancesCallback
        cb.on_persist_contacts_fn = persistContactsCallback
        cb.on_persist_asset_locks_fn = persistAssetLocksCallback
        cb.on_get_core_tx_record_fn = getCoreTxRecordCallback
        cb.on_get_core_tx_record_free_fn = getCoreTxRecordFreeCallback
        return cb
    }

    // MARK: - Changeset atomicity

    /// Opens a persistence round. Paired with
    /// [`endChangeset(walletId:success:)`]. Every per-kind handler
    /// (`persistIdentities`, `persistIdentityKeys`,
    /// `persistAccountChangeset`, …) fires between begin and end and
    /// only mutates `backgroundContext`; `save()` happens at the end.
    ///
    /// Currently a no-op beyond the tag — `ModelContext`'s pending-
    /// change buffer already gives us the batching we need. Kept as
    /// a named hook so future work (explicit transaction scoping,
    /// instrumented timing, etc.) has an obvious seam.
    func beginChangeset(walletId: Data) {
        onQueue {
            _ = walletId
            self.inChangeset = true
        }
    }

    /// Closes a persistence round. Commits all per-kind writes
    /// accumulated in `backgroundContext` since the matching
    /// `beginChangeset` in one `save()` (success path), or discards
    /// them via `rollback()` (failure path — any per-kind callback
    /// returned non-zero).
    ///
    /// One fsync per Rust `store()` round instead of one per per-
    /// kind callback, and the whole round is atomic from SwiftData's
    /// perspective: a crash between callbacks leaves the store in
    /// its pre-round state rather than half-applied.
    func endChangeset(walletId: Data, success: Bool) {
        onQueue {
            _ = walletId
            defer { self.inChangeset = false }
            if success {
                do {
                    try backgroundContext.save()
                } catch {
                    // The context still has the pending changes on
                    // its dirty list after a failed save; drop them so
                    // the next round starts clean. SQLite's WAL will
                    // only have committed data prior to this save, so
                    // the user-visible store is consistent.
                    print("⚠️ endChangeset: save failed: \(error.localizedDescription)")
                    backgroundContext.rollback()
                }
            } else {
                backgroundContext.rollback()
            }
        }
    }

    // MARK: - Identity scalar persistence

    /// Upsert / remove rows from `PersistentIdentity` in response to
    /// an `IdentityChangeSet` forwarded by the Rust side.
    ///
    /// Mapping:
    /// - Each `upsert.identity_id` gets an upsert on
    ///   `PersistentIdentity` keyed by that unique column.
    /// - Each `removed` id drops the matching row.
    ///
    /// Public keys are written by `persistIdentityKeys` on a paired
    /// callback; this path only touches the identity row itself.
    /// Both callbacks run under the same Rust-side wallet lock so
    /// the two-step apply is atomic from Swift's perspective.
    ///
    /// Primary-identity selection and the gap-limit scan watermark
    /// were dropped from the Rust side — the former moved to the UI
    /// layer, the latter is now derived from
    /// `IdentityManager.highestRegistrationIndex(...)` at read time.
    func persistIdentities(
        walletId: Data,
        upserts: [IdentityEntrySnapshot],
        removed: [Data]
    ) {
        onQueue {
        for entry in upserts {
            let identityId = entry.identityId
            let descriptor = FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == identityId }
            )
            let row: PersistentIdentity
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
            } else {
                // Resolve the network from the owning wallet row.
                // `persistWalletMetadata` always fires before the
                // first `persistIdentities` call for a new wallet, so
                // the row's network is populated by now; if for some
                // reason the lookup comes up empty, fall back to
                // `.testnet` so we never block the write path on a
                // missing network column (the CreateIdentity flow
                // restamps the network on return anyway).
                let resolvedWalletId = entry.walletId ?? walletId
                let network = walletNetwork(walletId: resolvedWalletId) ?? .testnet
                // `isLocal` is the "Local Only" badge in the UI —
                // identities the user created locally but Platform
                // hasn't confirmed yet. The persister fires *after*
                // Platform has confirmed, so any row created here
                // is by definition on-network. Wallet ownership
                // travels on `row.wallet` (the relationship set
                // below), not on this flag.
                row = PersistentIdentity(
                    identityId: entry.identityId,
                    balance: Int64(bitPattern: entry.balance),
                    revision: Int64(bitPattern: entry.revision),
                    isLocal: false,
                    network: network
                )
                backgroundContext.insert(row)
                // Back-fill any contracts in the local store that
                // already name this identity as their owner. Runs on
                // the background context — `ContractIdentityLinker`
                // is context-agnostic and isolation-free for exactly
                // this reason. The atomic save at `endChangeset`
                // persists the relationship.
                ContractIdentityLinker.linkIdentityToOwnedContracts(
                    identity: row,
                    modelContext: backgroundContext
                )
            }
            // Scalars that ride every upsert — Rust guarantees
            // monotonic revision + paired balance/revision updates
            // by the merge gate in `IdentityChangeSet::merge`, so
            // overwriting unconditionally here is safe.
            row.balance = Int64(bitPattern: entry.balance)
            row.revision = Int64(bitPattern: entry.revision)
            // Only write the index when the snapshot actually carries
            // one (wallet-owned identities). Out-of-wallet entries
            // arrive with `nil` — leave the existing column untouched
            // rather than overwriting with the placeholder `0`, which
            // would collide with a real wallet-owned identity at
            // index 0 if the row were ever rebound.
            if let idx = entry.identityIndex {
                row.identityIndex = idx
            }
            if let label = entry.label {
                row.alias = label
            }
            row.lastUpdated = Date()

            // Upsert the DPNS-label cache for this identity.
            //
            // The Rust changeset's merge policy is append-only
            // (`IdentityChangeSet::merge` only adds labels not
            // already present on the existing entry), so a label
            // missing from this flush does NOT mean it was removed
            // — we mirror that by inserting new rows but never
            // deleting existing ones here. DPNS doesn't expose a
            // user-driven "delete name" today; if/when it does, the
            // removal must arrive via a separate signal so we know
            // it's intentional.
            //
            // `acquiredAt` is informational on the existing row —
            // we refresh it on upsert so a later sync that fills in
            // the timestamp wins over an earlier `0` placeholder.
            upsertDPNSNames(
                identityRow: row,
                names: entry.dpnsNames
            )

            // Upsert the DashPay profile cache for this identity.
            //
            // Gated on `entry.dashpayProfile != nil` — a `nil`
            // snapshot mirrors the FFI's
            // `dashpay_profile_present == false`, which the Rust
            // `IdentityChangeSet::merge` policy treats as "no
            // update" (NOT delete). DashPay doesn't expose a
            // user-driven "delete profile" today; if it ever does,
            // the removal must arrive via a separate signal so we
            // know it's intentional. Match the dpns-name handling
            // shape: a missing snapshot leaves any existing row
            // intact.
            if let profile = entry.dashpayProfile {
                upsertDashpayProfile(identityRow: row, profile: profile)
            }

            // Attach the identity to its owning `PersistentWallet`
            // via the relationship. This is the sole wallet-side
            // association on the row — there is no denormalized
            // scalar — so downstream `@Query` views traverse
            // `identity.wallet?.walletId` when they need the raw
            // id. `deleteRule: .nullify` on the inverse nulls this
            // out cleanly if the wallet row is ever removed.
            //
            // Wallet id resolution: prefer the per-entry
            // `walletId` when Rust sets it (covers corner cases
            // where a changeset carries identities anchored to a
            // different wallet — e.g. a BLAST pass that surfaces
            // foreign identities the local wallet observes). Fall
            // back to the scope `walletId` that parameterised this
            // callback, which is always the wallet whose
            // changeset we're applying. The fallback matters for
            // the "create new identity" flow: Rust emits the
            // identity entry with `wallet_id_is_some == false`
            // (the identity wasn't wallet-linked in its own Rust
            // struct at emit time), and without the fallback we'd
            // orphan the just-registered row.
            let resolvedWalletId = entry.walletId ?? walletId
            row.wallet = fetchWalletForLink(walletId: resolvedWalletId)
        }

        for identityId in removed {
            let descriptor = FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == identityId }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                backgroundContext.delete(existing)
            }
        }

        // No save() — bracketed by changesetBegin/End.
        }  // onQueue
    }

    /// Upsert a `PersistentDPNSName` row for every label the FFI
    /// identity entry carried. Rows are keyed on
    /// `(networkRaw, normalizedParentDomainName, normalizedLabel)`,
    /// matching `PersistentDPNSName`'s
    /// `#Unique<…>([\.networkRaw, \.normalizedParentDomainName,
    /// \.normalizedLabel])` declaration — which itself mirrors the
    /// DPNS contract's `parentNameAndLabel` unique index. If a label
    /// transferred between identities on the same network the
    /// existing row's `identity` is rebound to the current owner.
    ///
    /// The FFI `IdentityEntryFFI.dpns_names` array carries only the
    /// display label today; the parent domain defaults to `"dash"`
    /// (the only top-level DPNS domain on Dash Platform), and the
    /// normalized forms are derived via
    /// `PersistentDPNSName.normalize(_:)` on insert. If/when the FFI
    /// is extended to carry the parent domain, this site's defaults
    /// become the fallback path.
    ///
    /// Append-only at the per-identity level: existing rows whose
    /// label is no longer in the FFI list survive (see the call-site
    /// comment on `IdentityChangeSet::merge`'s policy). The function
    /// only ever inserts or refreshes; it does NOT cascade-prune.
    ///
    /// Assumes it's already running on `serialQueue` — only called
    /// from inside `persistIdentities`'s `onQueue` body.
    private func upsertDPNSNames(
        identityRow: PersistentIdentity,
        names: [(label: String, acquiredAt: UInt64)]
    ) {
        if names.isEmpty {
            return
        }

        let networkRaw = identityRow.networkRaw
        // DPNS today exposes only the "dash" top-level domain. If the
        // FFI ever forwards a different parent, the model carries it
        // through verbatim — for now we stamp the universal default.
        let parentDomainName = "dash"
        let normalizedParentDomainName = PersistentDPNSName.normalize(parentDomainName)

        for entry in names {
            let normalizedLabel = PersistentDPNSName.normalize(entry.label)
            let descriptor = FetchDescriptor<PersistentDPNSName>(
                predicate: #Predicate {
                    $0.networkRaw == networkRaw
                        && $0.normalizedParentDomainName == normalizedParentDomainName
                        && $0.normalizedLabel == normalizedLabel
                }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                // Refresh the timestamp if the FFI now carries a
                // non-zero value. Don't clobber a real timestamp
                // with a `0` placeholder — `acquired_at` is sticky
                // once set.
                if entry.acquiredAt != 0 && existing.acquiredAt != entry.acquiredAt {
                    existing.acquiredAt = entry.acquiredAt
                    existing.lastUpdated = Date()
                }
                // Refresh the display label too — a later flush may
                // carry a corrected casing for the same normalized
                // form (e.g. originally synced as "alice" then
                // re-synced as "Alice"). The normalized index column
                // doesn't change, so the unique constraint holds.
                if existing.label != entry.label {
                    existing.label = entry.label
                    existing.lastUpdated = Date()
                }
                // Rebind to the current owner if the label transferred
                // between identities on this network. DPNS supports
                // transfers, and the unique constraint is per-network,
                // so the row stays but the owner pointer moves.
                if existing.identity !== identityRow {
                    existing.identity = identityRow
                    existing.lastUpdated = Date()
                }
            } else {
                let row = PersistentDPNSName(
                    identity: identityRow,
                    label: entry.label,
                    parentDomainName: parentDomainName,
                    acquiredAt: entry.acquiredAt
                )
                backgroundContext.insert(row)
            }
        }
    }

    /// Upsert the at-most-one `PersistentDashpayProfile` row for an
    /// identity. Idempotent on repeated flushes: an existing row is
    /// refreshed in place rather than replaced, so SwiftUI views
    /// observing it via `@Query` see field-level updates rather than
    /// row-replacement churn.
    ///
    /// The DashPay contract guarantees one `profile` document per
    /// `ownerId`, so we never have to disambiguate multiple rows for
    /// the same identity — `identityRow.dashpayProfile` is either
    /// already present (refresh) or absent (insert).
    ///
    /// Runs on `serialQueue` — only called from inside
    /// `persistIdentities`'s `onQueue` body.
    private func upsertDashpayProfile(
        identityRow: PersistentIdentity,
        profile: DashpayProfileSnapshot
    ) {
        if let existing = identityRow.dashpayProfile {
            // Field-level refresh. Every column is overwritten on
            // every flush — the FFI snapshot is authoritative for
            // the profile document's contents (the underlying
            // `IdentityEntry::dashpay_profile` is a whole-document
            // `Some(_)` payload, not a partial diff). Fields the
            // sender omitted come through as `nil` here too, so
            // setting them to nil mirrors the on-Platform state.
            existing.displayName = profile.displayName
            existing.bio = profile.bio
            existing.publicMessage = profile.publicMessage
            existing.avatarUrl = profile.avatarUrl
            existing.avatarHash = profile.avatarHash
            existing.avatarFingerprint = profile.avatarFingerprint
            existing.lastUpdated = Date()
        } else {
            let row = PersistentDashpayProfile(
                identity: identityRow,
                displayName: profile.displayName,
                publicMessage: profile.publicMessage,
                bio: profile.bio,
                avatarUrl: profile.avatarUrl,
                avatarHash: profile.avatarHash,
                avatarFingerprint: profile.avatarFingerprint
            )
            backgroundContext.insert(row)
            // SwiftData populates the inverse `dashpayProfile`
            // pointer from the `inverse:` declaration on
            // `PersistentIdentity.dashpayProfile`, so we don't need
            // to assign `identityRow.dashpayProfile = row` here.
        }
    }

    // MARK: - Identity keys persistence

    /// Upsert / remove rows from `PersistentPublicKey` in response to
    /// an `IdentityKeysChangeSet` forwarded by the Rust side.
    ///
    /// Mapping:
    /// - Each `upsert` is keyed by `(identity_id, key_id)` — the
    ///   same composite the Rust side uses for `BTreeMap` uniqueness.
    /// - Each `removed` pair deletes the matching row.
    ///
    /// `PrivateKeyKindFFI` encoding:
    /// - `None` (0): clear any stored `privateKeyKeychainIdentifier`.
    /// - `Clear` (1): store raw 32-byte key material to the Keychain
    ///   via `KeychainManager`, record the resulting identifier.
    /// - `AtWalletDerivationPath` (2): no Keychain write — the seed
    ///   is stored at wallet level, and `derivationPath` tells the
    ///   signing path to re-derive. Stored as the identifier so
    ///   `hasPrivateKey` still reflects presence, but with a
    ///   `derived:` prefix so consumers can distinguish stored-bytes
    ///   vs. derived-on-demand.
    func persistIdentityKeys(
        walletId: Data,
        upserts: [IdentityKeyEntrySnapshot],
        removed: [(identityId: Data, keyId: UInt32)]
    ) {
        onQueue {
        for entry in upserts {
            // PersistentPublicKey is keyed on (identity, keyId) via
            // its parent relationship; fetch by keyId + identityId
            // (stored as base58 string on the row).
            let targetKeyId = Int32(bitPattern: entry.keyId)
            let identityHex = entry.identityId.toBase58String()
            let descriptor = FetchDescriptor<PersistentPublicKey>(
                predicate: #Predicate {
                    $0.keyId == targetKeyId && $0.identityId == identityHex
                }
            )
            let row: PersistentPublicKey
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
            } else {
                let purposeEnum = KeyPurpose(rawValue: entry.purpose) ?? .authentication
                let levelEnum = SecurityLevel(rawValue: entry.securityLevel) ?? .high
                let keyTypeEnum = KeyType(rawValue: entry.keyType) ?? .ecdsaSecp256k1
                row = PersistentPublicKey(
                    keyId: targetKeyId,
                    purpose: purposeEnum,
                    securityLevel: levelEnum,
                    keyType: keyTypeEnum,
                    publicKeyData: entry.publicKeyData,
                    readOnly: entry.readOnly,
                    disabledAt: entry.disabledAt.map { Int64(bitPattern: $0) },
                    identityId: identityHex
                )
                backgroundContext.insert(row)
                // Link to the owning identity if we already have the
                // row. (We don't insert a missing parent here —
                // Rust-side ordering guarantees identities apply
                // before keys within the same changeset.)
                let identityIdData = entry.identityId
                let parentDescriptor = FetchDescriptor<PersistentIdentity>(
                    predicate: #Predicate { $0.identityId == identityIdData }
                )
                if let parent = try? backgroundContext.fetch(parentDescriptor).first {
                    row.identity = parent
                    parent.addPublicKey(row)
                }
            }
            // Refresh mutable fields every upsert.
            row.publicKeyData = entry.publicKeyData
            row.readOnly = entry.readOnly
            row.disabledAt = entry.disabledAt.map { Int64(bitPattern: $0) }

            // Private-key handling.
            //
            // No bytes cross the FFI — when the entry carries
            // derivation indices, Swift re-derives the 32-byte
            // ECDSA scalar from the owning wallet's mnemonic and
            // stores it in the keychain under the serialized
            // derivation path. Wallet id resolves the same way as
            // for the identity row itself: prefer per-entry
            // `entry.walletId` (lets Rust route a key to a
            // foreign wallet in some future cross-wallet-scan
            // flow), fall back to the scope `walletId` that
            // parameterised this callback. Keys without
            // derivation indices are watch-only and clear any
            // prior stored identifier.
            if let indices = entry.derivationIndices {
                let resolvedWalletId = entry.walletId ?? walletId
                let keychainId = deriveAndStoreIdentityKey(
                    entry: entry,
                    walletId: resolvedWalletId,
                    indices: indices,
                    publicKeyHex: entry.publicKeyData.toHexString(),
                    publicKeyHashHex: entry.publicKeyHash.toHexString(),
                    identityIdBase58: identityHex
                )
                row.privateKeyKeychainIdentifier = keychainId
            } else {
                row.privateKeyKeychainIdentifier = nil
            }

            row.lastAccessed = Date()
        }

        for (identityIdBytes, keyId) in removed {
            let targetKeyId = Int32(bitPattern: keyId)
            let identityHex = identityIdBytes.toBase58String()
            let descriptor = FetchDescriptor<PersistentPublicKey>(
                predicate: #Predicate {
                    $0.keyId == targetKeyId && $0.identityId == identityHex
                }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                backgroundContext.delete(existing)
            }
        }

        // `walletId` is now consumed as the scope fallback in the
        // derivation branch above, so it's no longer a dead
        // parameter. No save() — bracketed by
        // changesetBegin/End.
        }  // onQueue
    }

    // MARK: - Token balance persistence

    /// Apply a `TokenBalanceChangeSet` upsert/removal pair to
    /// `PersistentTokenBalance` rows.
    ///
    /// Mapping:
    /// - Each upsert is keyed by `(tokenId, identityId)` — the same
    ///   composite the Rust side uses on its `BTreeMap`. The 32-byte
    ///   token id from Rust is rendered as base58 to match
    ///   `PersistentTokenBalance.tokenId` (string column, the same
    ///   shape the rest of the app uses for token id strings).
    /// - Each removal deletes the matching row.
    ///
    /// Token metadata (name, symbol, decimals) is owned by
    /// `PersistentToken` and joined at read time — we don't replicate
    /// it here. The `PersistentTokenBalance.token` relationship is
    /// linked when the matching `PersistentToken` row exists; rows
    /// inserted before the contract has been parsed locally simply
    /// link later when SwiftUI re-queries.
    func persistTokenBalances(
        walletId: Data,
        upserts: [TokenBalanceUpsertSnapshot],
        removals: [TokenBalanceRemovalSnapshot]
    ) {
        onQueue {
        let network = walletNetwork(walletId: walletId) ?? .testnet

        for entry in upserts {
            let tokenIdBase58 = entry.tokenId.toBase58String()
            let identityIdData = entry.identityId
            let descriptor = FetchDescriptor<PersistentTokenBalance>(
                predicate: #Predicate {
                    $0.tokenId == tokenIdBase58 && $0.identityId == identityIdData
                }
            )
            let row: PersistentTokenBalance
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
            } else {
                row = PersistentTokenBalance(
                    tokenId: tokenIdBase58,
                    identityId: entry.identityId,
                    balance: 0,
                    network: network
                )
                backgroundContext.insert(row)
                linkTokenBalanceRelations(
                    row: row,
                    identityId: entry.identityId,
                    tokenIdData: entry.tokenId
                )
            }
            row.updateBalance(Int64(bitPattern: entry.balance))
            row.markAsSynced()
            // Re-link on every upsert too so a balance row that
            // pre-existed before its parent identity / token row
            // landed gets stitched into the relationship graph on the
            // next sync round.
            if row.identity == nil || row.token == nil {
                linkTokenBalanceRelations(
                    row: row,
                    identityId: entry.identityId,
                    tokenIdData: entry.tokenId
                )
            }
        }

        for entry in removals {
            let tokenIdBase58 = entry.tokenId.toBase58String()
            let identityIdData = entry.identityId
            let descriptor = FetchDescriptor<PersistentTokenBalance>(
                predicate: #Predicate {
                    $0.tokenId == tokenIdBase58 && $0.identityId == identityIdData
                }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                backgroundContext.delete(existing)
            }
        }

        // No save() — bracketed by changesetBegin/End from the Rust
        // store() round.
        }  // onQueue
    }

    /// Stitch a freshly-inserted `PersistentTokenBalance` into the
    /// relationship graph: link the owning `PersistentIdentity` (when
    /// present locally) and the matching `PersistentToken` (looked up
    /// by its 32-byte canonical id, which `PersistentToken.id`
    /// stores). Either side may legitimately be missing if the row
    /// is being inserted before the contract has been parsed locally
    /// — the next sync round re-links via the upsert-path nil-check.
    private func linkTokenBalanceRelations(
        row: PersistentTokenBalance,
        identityId: Data,
        tokenIdData: Data
    ) {
        let identityDescriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == identityId }
        )
        if let parent = try? backgroundContext.fetch(identityDescriptor).first {
            row.identity = parent
        }
        let tokenDescriptor = FetchDescriptor<PersistentToken>(
            predicate: #Predicate { $0.id == tokenIdData }
        )
        if let token = try? backgroundContext.fetch(tokenDescriptor).first {
            row.token = token
        }
    }

    /// Owned snapshot of a `TokenBalanceUpsertFFI` row. Same
    /// rationale as `IdentityEntrySnapshot`: callbacks copy out the
    /// raw FFI struct fields before the trampoline returns, so the
    /// handler runs against pure-Swift values regardless of when the
    /// Rust-side allocation gets reclaimed.
    struct TokenBalanceUpsertSnapshot {
        let identityId: Data
        let tokenId: Data
        let balance: UInt64
    }

    /// Owned snapshot of a `TokenBalanceRemovalFFI` row.
    struct TokenBalanceRemovalSnapshot {
        let identityId: Data
        let tokenId: Data
    }

    // MARK: - DashPay contact-request persistence

    /// Apply a DashPay `ContactChangeSet` projection to SwiftData.
    ///
    /// Mapping:
    /// - Each `upsert.ContactRequestFFI` becomes one row keyed by
    ///   `(networkRaw, ownerIdentityId, contactIdentityId, isOutgoing)`
    ///   on `PersistentDashpayContactRequest`. The Rust side projects
    ///   `ContactChangeSet::sent_requests` / `incoming_requests` /
    ///   `established` into this flat array (with `is_outgoing`
    ///   stamped per row), so the upsert path is direction-agnostic.
    /// - Each `removedSent` row drops the matching outgoing row.
    /// - Each `removedIncoming` row drops the matching incoming row.
    ///
    /// The owner identity is required to exist in SwiftData before
    /// the row is inserted — the relationship is non-optional and
    /// `networkRaw` is read off it. If a flush carries a contact
    /// upsert for an owner identity Swift hasn't seen yet (race with
    /// a first-time identity flush), the row is skipped; the next
    /// flush will replay it after the identity row lands. In
    /// practice the changeset is one round, so this only matters
    /// for the very first identity registration where the contact
    /// changeset and identity changeset arrive in the same store()
    /// call — within a round, identities apply before contacts (see
    /// the ordering in `FFIPersister::store`), so the lookup here
    /// will normally succeed.
    func persistContacts(
        walletId: Data,
        upserts: [ContactRequestSnapshot],
        removedSent: [ContactRequestRemovalSnapshot],
        removedIncoming: [ContactRequestRemovalSnapshot]
    ) {
        onQueue {
            for entry in upserts {
                let ownerId = entry.ownerIdentityId
                let ownerDescriptor = FetchDescriptor<PersistentIdentity>(
                    predicate: #Predicate { $0.identityId == ownerId }
                )
                guard let owner = try? backgroundContext.fetch(ownerDescriptor).first else {
                    // Owner identity hasn't landed yet. Within a
                    // single round identities apply before contacts,
                    // so we'd only hit this if the FFI changeset
                    // surfaces a contact for an identity that isn't
                    // managed by any wallet locally — there's no
                    // identity row to hang it off, and the contract's
                    // `ownerId` invariant means the row would be
                    // orphaned anyway. Skip silently; the next sync
                    // round will replay it once the owner row exists.
                    continue
                }

                let networkRaw = owner.networkRaw
                let contactId = entry.contactIdentityId
                let isOutgoing = entry.isOutgoing
                let descriptor = FetchDescriptor<PersistentDashpayContactRequest>(
                    predicate: #Predicate {
                        $0.networkRaw == networkRaw
                            && $0.ownerIdentityId == ownerId
                            && $0.contactIdentityId == contactId
                            && $0.isOutgoing == isOutgoing
                    }
                )
                if let existing = try? backgroundContext.fetch(descriptor).first {
                    // Refresh in place — every column is overwritten
                    // because the FFI snapshot is authoritative for
                    // the underlying `ContactRequest` document. This
                    // is also the path `established` rows take to
                    // promote a previously-pending row in place over
                    // its prior `sent_requests` / `incoming_requests`
                    // entry; the unique key is identical because the
                    // promotion doesn't change `(owner, contact,
                    // direction)`.
                    existing.senderKeyIndex = entry.senderKeyIndex
                    existing.recipientKeyIndex = entry.recipientKeyIndex
                    existing.accountReference = entry.accountReference
                    existing.encryptedPublicKey = entry.encryptedPublicKey
                    existing.encryptedAccountLabel = entry.encryptedAccountLabel
                    existing.autoAcceptProof = entry.autoAcceptProof
                    existing.coreHeightCreatedAt = entry.coreHeightCreatedAt
                    existing.createdAtMillis = entry.createdAtMillis
                    if existing.owner !== owner {
                        existing.owner = owner
                    }
                    existing.lastUpdated = Date()
                } else {
                    let row = PersistentDashpayContactRequest(
                        owner: owner,
                        contactIdentityId: entry.contactIdentityId,
                        isOutgoing: entry.isOutgoing,
                        senderKeyIndex: entry.senderKeyIndex,
                        recipientKeyIndex: entry.recipientKeyIndex,
                        accountReference: entry.accountReference,
                        encryptedPublicKey: entry.encryptedPublicKey,
                        encryptedAccountLabel: entry.encryptedAccountLabel,
                        autoAcceptProof: entry.autoAcceptProof,
                        coreHeightCreatedAt: entry.coreHeightCreatedAt,
                        createdAtMillis: entry.createdAtMillis
                    )
                    backgroundContext.insert(row)
                }
            }

            for tomb in removedSent {
                deleteContactRow(
                    ownerId: tomb.ownerIdentityId,
                    contactId: tomb.contactIdentityId,
                    isOutgoing: true
                )
            }
            for tomb in removedIncoming {
                deleteContactRow(
                    ownerId: tomb.ownerIdentityId,
                    contactId: tomb.contactIdentityId,
                    isOutgoing: false
                )
            }
            // No save() — bracketed by changesetBegin/End from the
            // Rust store() round.
            _ = walletId  // reserved for future wallet-scope batching
        }
    }

    /// Delete the single `PersistentDashpayContactRequest` row matching
    /// `(ownerIdentityId, contactIdentityId, isOutgoing)`. The fourth
    /// uniqueness column (`networkRaw`) is implied by the owner — an
    /// identity belongs to exactly one network — so we don't have to
    /// fan out the predicate across networks. Silent on miss (no
    /// existing row): the FFI changeset replays tombstones, and an
    /// already-removed row is the success state.
    ///
    /// Assumes it's already running on `serialQueue`.
    private func deleteContactRow(ownerId: Data, contactId: Data, isOutgoing: Bool) {
        let direction = isOutgoing
        let descriptor = FetchDescriptor<PersistentDashpayContactRequest>(
            predicate: #Predicate {
                $0.ownerIdentityId == ownerId
                    && $0.contactIdentityId == contactId
                    && $0.isOutgoing == direction
            }
        )
        if let existing = try? backgroundContext.fetch(descriptor).first {
            backgroundContext.delete(existing)
        }
    }

    /// Owned snapshot of a `ContactRequestFFI` row. Decouples the
    /// lifetime of the encrypted-key buffers from the Rust-side
    /// allocation: the callback copies them into Swift `Data` before
    /// returning, so `free_contact_requests_ffi` runs cleanly.
    struct ContactRequestSnapshot {
        let ownerIdentityId: Data
        let contactIdentityId: Data
        let isOutgoing: Bool
        let senderKeyIndex: UInt32
        let recipientKeyIndex: UInt32
        let accountReference: UInt32
        let encryptedPublicKey: Data
        let encryptedAccountLabel: Data?
        let autoAcceptProof: Data?
        let coreHeightCreatedAt: UInt32
        let createdAtMillis: UInt64
    }

    /// Owned snapshot of a `ContactRequestRemovalFFI` row. Carries
    /// just the `(owner, contact)` pair — the direction is implied
    /// by which array (`removed_sent` vs `removed_incoming`) the
    /// removal came from on the FFI side.
    struct ContactRequestRemovalSnapshot {
        let ownerIdentityId: Data
        let contactIdentityId: Data
    }

    // MARK: - Identity private-key derivation

    /// Derive the 32-byte ECDSA scalar for an identity key from the
    /// owning wallet's mnemonic and stash it in the keychain at the
    /// serialized DIP-9 derivation path. Returns the keychain
    /// account string on success (which `PersistentPublicKey.priv-
    /// ateKeyKeychainIdentifier` stores) or `nil` if anything in the
    /// pipeline fails — mnemonic missing, network unresolved, path
    /// build error, FFI derivation error, or keychain write failure.
    ///
    /// Idempotent per `(wallet, identity_index, key_index)` triple:
    /// repeated persister callbacks for the same key overwrite
    /// cleanly via `storeIdentityPrivateKey`'s delete-then-add.
    ///
    /// Runs off the main actor (this whole handler fires from the
    /// Rust persister thread); every touched API is either
    /// `nonisolated` or backed by thread-safe primitives.
    private func deriveAndStoreIdentityKey(
        entry: IdentityKeyEntrySnapshot,
        walletId: Data,
        indices: (identityIndex: UInt32, keyIndex: UInt32),
        publicKeyHex: String,
        publicKeyHashHex: String,
        identityIdBase58: String
    ) -> String? {
        // 1. Resolve the wallet's network from SwiftData. We need it
        //    to feed `KeyDerivation.getIdentityAuthenticationPath`
        //    so the path chooses the right `coin_type` (mainnet vs
        //    testnet).
        let walletDescriptor = FetchDescriptor<PersistentWallet>(
            predicate: PersistentWallet.predicate(walletId: walletId)
        )
        guard
            let persistentWallet = try? backgroundContext.fetch(walletDescriptor).first
        else {
            print("⚠️ deriveAndStoreIdentityKey: wallet row not found for \(walletId.prefix(4).toHexString())…")
            return nil
        }
        let network: Network = persistentWallet.network ?? .testnet

        // 2. Fetch the mnemonic UTF-8 bytes for this wallet from the
        //    keychain. Keep the call site off Swift `String` so the
        //    plaintext phrase does not live in higher-level heap
        //    objects longer than necessary.
        let mnemonicUTF8Bytes: Data
        do {
            mnemonicUTF8Bytes = try WalletStorage().retrieveMnemonicUTF8Bytes(for: walletId)
        } catch {
            print("⚠️ deriveAndStoreIdentityKey: mnemonic missing for wallet \(walletId.prefix(4).toHexString())…: \(error.localizedDescription)")
            return nil
        }

        // 3. Mnemonic UTF-8 bytes → 64-byte BIP39 seed.
        let seed: Data
        do {
            seed = try Mnemonic.toSeed(mnemonicUTF8Bytes: mnemonicUTF8Bytes)
        } catch {
            print("⚠️ deriveAndStoreIdentityKey: mnemonic-to-seed failed: \(error.localizedDescription)")
            return nil
        }

        // 4. Build the DIP-9 authentication path. The string form
        //    doubles as the keychain account suffix so the explorer
        //    can render it.
        let derivationPath: String
        do {
            derivationPath = try KeyDerivation.getIdentityAuthenticationPath(
                network: network,
                identityIndex: indices.identityIndex,
                keyIndex: indices.keyIndex
            )
        } catch {
            print("⚠️ deriveAndStoreIdentityKey: path build failed: \(error.localizedDescription)")
            return nil
        }

        // 5. Derive the 32-byte scalar via the FFI bridge. The
        //    bridge writes into a caller-provided buffer; we zero
        //    the scratch `Data` on the way out for hygiene (the
        //    keychain item is the real home for the bytes).
        var privateKey = Data(count: 32)
        let rc: Int32 = privateKey.withUnsafeMutableBytes { pkBytes -> Int32 in
            guard let pkPtr = pkBytes.bindMemory(to: UInt8.self).baseAddress else { return -1 }
            return seed.withUnsafeBytes { seedBytes -> Int32 in
                guard let seedPtr = seedBytes.bindMemory(to: UInt8.self).baseAddress else {
                    return -1
                }
                return derivationPath.withCString { pathCStr in
                    key_wallet_derive_private_key_from_seed(seedPtr, pathCStr, pkPtr)
                }
            }
        }
        guard rc == 0 else {
            print("⚠️ deriveAndStoreIdentityKey: FFI derive failed (rc=\(rc))")
            // Zero out any partial write before returning.
            privateKey.resetBytes(in: 0..<privateKey.count)
            return nil
        }

        // 6. Stash in the keychain. `KeychainManager.shared` is the
        //    single app-wide instance backed by
        //    `org.dashfoundation.wallet`.
        let metadata = KeychainManager.IdentityPrivateKeyMetadata(
            identityId: identityIdBase58,
            keyId: entry.keyId,
            walletId: walletId.toHexString(),
            identityIndex: indices.identityIndex,
            keyIndex: indices.keyIndex,
            derivationPath: derivationPath,
            publicKey: publicKeyHex,
            publicKeyHash: publicKeyHashHex,
            keyType: entry.keyType,
            purpose: entry.purpose,
            securityLevel: entry.securityLevel
        )
        let account = KeychainManager.shared.storeIdentityPrivateKey(
            privateKey,
            derivationPath: derivationPath,
            metadata: metadata
        )

        // 7. Scrub the local copy regardless of outcome.
        privateKey.resetBytes(in: 0..<privateKey.count)

        if account == nil {
            print("⚠️ deriveAndStoreIdentityKey: keychain write failed for \(derivationPath)")
        }
        return account
    }

    // MARK: - Identity snapshot structs

    /// Swift-side snapshot of the Rust `IdentityEntryFFI` with C
    /// strings + byte tuples already converted. The callback copies
    /// these out of the raw FFI struct before handing control to
    /// `persistIdentities` so the Rust-side free-loop can run
    /// immediately after the callback returns.
    struct IdentityEntrySnapshot {
        let identityId: Data
        let balance: UInt64
        let revision: UInt64
        /// `nil` for out-of-wallet (observed) identities — they have
        /// no derivation context. `Some(_)` mirrors the BIP-9 HD
        /// identity index used during registration.
        let identityIndex: UInt32?
        let label: String?
        let status: UInt8
        let walletId: Data?
        /// Confirmed DPNS labels owned by this identity, paired with
        /// their `acquired_at` Unix-millis timestamp (`0` when the
        /// source `Option<u64>` was `None`). Mirrors the parallel
        /// `dpns_names` / `dpns_names_acquired_at` arrays on
        /// `IdentityEntryFFI`. Empty when the identity has no settled
        /// labels.
        let dpnsNames: [(label: String, acquiredAt: UInt64)]
        /// DashPay profile snapshot — populated iff
        /// `IdentityEntryFFI.dashpay_profile_present == true`. `nil`
        /// means "no update for this flush", which mirrors the
        /// changeset's `dashpay_profile: None` semantics on the Rust
        /// side (NOT a delete signal). Inner fields are individually
        /// optional because every DashPay profile field but the
        /// implicit `$ownerId` is optional in the contract schema.
        let dashpayProfile: DashpayProfileSnapshot?
    }

    /// Owned snapshot of the `dashpay_profile_*` fields on
    /// `IdentityEntryFFI`. Decouples the lifetime of every contained
    /// `String` / `Data` from the FFI heap so the callback can
    /// return immediately and the Rust side can run its free-loop.
    struct DashpayProfileSnapshot {
        let displayName: String?
        let bio: String?
        let publicMessage: String?
        let avatarUrl: String?
        /// 32-byte SHA-256 of the avatar binary (DIP-15 `avatarHash`).
        /// `nil` when the source `avatar_hash_present == false` —
        /// disambiguates "no hash" from "all-zero hash" since the
        /// underlying byte array is zero-initialized either way.
        let avatarHash: Data?
        /// 8-byte DHash perceptual fingerprint (DIP-15
        /// `avatarFingerprint`). `nil` when the source
        /// `avatar_fingerprint_present == false`.
        let avatarFingerprint: Data?
    }

    /// Swift-side snapshot of `IdentityKeyEntryFFI` — public-key
    /// payload copied to owned `Data`, derivation breadcrumb +
    /// precomputed pubkey hash captured as scalars. Same rationale
    /// as `IdentityEntrySnapshot`: decouple lifetime from the
    /// callback window.
    struct IdentityKeyEntrySnapshot {
        let identityId: Data
        let keyId: UInt32
        let purpose: UInt8
        let securityLevel: UInt8
        let keyType: UInt8
        let readOnly: Bool
        let disabledAt: UInt64?
        let publicKeyData: Data
        let publicKeyHash: Data
        /// Owning wallet if this key is derivable from one we control.
        let walletId: Data?
        /// DIP-9 `(identity_index, key_index)` pair. Present iff the
        /// client is expected to re-derive the private key locally.
        let derivationIndices: (identityIndex: UInt32, keyIndex: UInt32)?
    }

    // MARK: - Watch-only Restore: Account Addresses

    /// Upsert `PersistentCoreAddress` rows for one account's address
    /// pool. Fires on wallet create (initial gap-limit fill), pool
    /// extension, and when SPV flips an address's `used` flag.
    ///
    /// Addresses are identified by their base58check string (the
    /// `@Attribute(.unique)` on `PersistentCoreAddress.address`).
    /// Parent linkage uses the same lookup key as
    /// `persistAccount(walletId:spec:)` so the row reliably maps to
    /// the right `PersistentAccount`.
    func persistAccountAddresses(
        walletId: Data,
        accountKey: AccountLookupKey,
        entries: [CoreAddressEntrySnapshot]
    ) {
        onQueue {
        guard let account = fetchAccount(walletId: walletId, key: accountKey) else {
            return
        }

        // DIP-17 PlatformPayment pool addresses land in
        // `PersistentPlatformAddress` so they don't share a model with
        // Core-chain (base58check) addresses.
        let isPlatformPayment = accountKey.typeTag == 14
        if isPlatformPayment {
            persistPlatformPaymentAddresses(
                account: account,
                walletId: walletId,
                entries: entries
            )
            return
        }

        for entry in entries {
            let address = entry.address
            let existingDescriptor = FetchDescriptor<PersistentCoreAddress>(
                predicate: #Predicate { $0.address == address }
            )
            let existing = try? backgroundContext.fetch(existingDescriptor).first
            let row: PersistentCoreAddress
            if let existing = existing {
                row = existing
            } else {
                row = PersistentCoreAddress(
                    address: entry.address,
                    publicKey: entry.publicKey,
                    poolTypeTag: entry.poolTypeTag,
                    addressIndex: entry.addressIndex,
                    derivationPath: entry.derivationPath,
                    isUsed: entry.isUsed,
                    balance: entry.balance
                )
                backgroundContext.insert(row)
            }
            // Mutation path for both insert + update.
            row.publicKey = entry.publicKey
            row.poolTypeTag = entry.poolTypeTag
            row.addressIndex = entry.addressIndex
            row.derivationPath = entry.derivationPath
            row.isUsed = entry.isUsed
            row.balance = entry.balance
            row.account = account
            row.lastUpdated = Date()

            // Backfill the `coreAddress` link on any TXOs that were
            // persisted before this address row existed. The SPV
            // pass can emit UTXOs for an address whose pool row
            // hasn't landed yet; in that case `upsertUtxo` skipped
            // the relationship and `record.coreAddress` stayed nil.
            // Without this sweep the storage-explorer's "Address
            // Row" field renders as "—" forever even though the
            // address row now exists. Avoid the SwiftData
            // optional-relationship-in-predicate gotcha by
            // filtering nil-coreAddress in Swift after the fetch.
            let txoBackfillDescriptor = FetchDescriptor<PersistentTxo>(
                predicate: #Predicate { $0.address == address }
            )
            if let txosAtAddress = try? backgroundContext.fetch(txoBackfillDescriptor) {
                for txo in txosAtAddress where txo.coreAddress == nil {
                    txo.coreAddress = row
                }
            }
        }

        if !self.inChangeset { try? backgroundContext.save() }
        }  // onQueue
    }

    /// Upsert PlatformPayment entries into `PersistentPlatformAddress`.
    /// Called only when the address-emit target account is a DIP-17
    /// PlatformPayment account (type tag 14). The Rust side emits the
    /// DIP-0018 bech32m form in `entry.address`; we derive the
    /// 20-byte hash + address type here so BLAST balance updates
    /// (which arrive with `addressHash` only) can upsert the same row.
    private func persistPlatformPaymentAddresses(
        account: PersistentAccount,
        walletId: Data,
        entries: [CoreAddressEntrySnapshot]
    ) {
        for entry in entries {
            guard let (addressType, addressHash) =
                platformAddressComponents(fromBech32m: entry.address)
            else {
                continue
            }
            let address = entry.address
            let descriptor = FetchDescriptor<PersistentPlatformAddress>(
                predicate: #Predicate { $0.address == address }
            )
            let row: PersistentPlatformAddress
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
                row.addressType = addressType
                row.addressHash = addressHash
            } else {
                row = PersistentPlatformAddress(
                    address: entry.address,
                    addressType: addressType,
                    addressHash: addressHash,
                    publicKey: entry.publicKey,
                    accountIndex: account.accountIndex,
                    addressIndex: entry.addressIndex,
                    derivationPath: entry.derivationPath,
                    isUsed: entry.isUsed,
                    balance: entry.balance,
                    nonce: 0,
                    walletId: walletId
                )
                backgroundContext.insert(row)
            }
            // Address-emit is authoritative for derivation metadata
            // and the used flag on first creation; we preserve any
            // later BLAST-driven balance/nonce updates by only
            // lowering `isUsed` if the emit says so explicitly and
            // we don't already have funds showing.
            row.publicKey = entry.publicKey
            row.accountIndex = account.accountIndex
            row.addressIndex = entry.addressIndex
            row.derivationPath = entry.derivationPath
            if entry.isUsed {
                row.isUsed = true
            } else if row.balance == 0 && row.nonce == 0 {
                row.isUsed = false
            }
            if row.balance == 0 && entry.balance != 0 {
                row.balance = entry.balance
            }
            row.account = account
            row.walletId = walletId
            row.lastUpdated = Date()
        }

        if !self.inChangeset { try? backgroundContext.save() }
    }

    /// Split a DIP-0018 bech32m platform address back into
    /// `(addressType, 20-byte hash)`. Returns nil on any decode
    /// failure or unexpected type byte. Type bytes follow
    /// DIP-0018: `0xb0` → P2PKH (stored as 0), `0x80` → P2SH
    /// (stored as 1).
    private func platformAddressComponents(
        fromBech32m address: String
    ) -> (addressType: UInt8, hash: Data)? {
        guard let decoded = Bech32m.decode(address.lowercased()),
              decoded.hrp == "dash" || decoded.hrp == "tdash",
              decoded.data.count == 21
        else {
            return nil
        }
        let typeByte = decoded.data[0]
        let hash = decoded.data.subdata(in: 1..<21)
        switch typeByte {
        case 0xb0: return (0, hash)
        case 0x80: return (1, hash)
        default: return nil
        }
    }

    /// Lookup key mirroring the identifying subset of
    /// `AccountSpecFFI` so the handler can locate the
    /// `PersistentAccount` row for address linkage. `standardTag` is
    /// included because a wallet can have both BIP44 (tag 0) and
    /// BIP32 (tag 1) Standard accounts at the same index — without
    /// disambiguating on `standardTag`, BIP32 addresses would be
    /// routed to the BIP44 row.
    struct AccountLookupKey {
        let typeTag: UInt32
        let index: UInt32
        let standardTag: UInt8
        let registrationIndex: UInt32
        let keyClass: UInt32
        let userIdentityId: Data
        let friendIdentityId: Data
    }

    /// Snapshot of a `CoreAddressEntryFFI` with the C strings copied
    /// into Swift Strings so the callback can return before we touch
    /// the data.
    struct CoreAddressEntrySnapshot {
        let address: String
        let publicKey: Data
        let poolTypeTag: UInt8
        let addressIndex: UInt32
        let isUsed: Bool
        let balance: UInt64
        let derivationPath: String
    }

    private func fetchAccount(
        walletId: Data,
        key: AccountLookupKey
    ) -> PersistentAccount? {
        let typeTag = key.typeTag
        let index = key.index
        let descriptor = FetchDescriptor<PersistentAccount>(
            predicate: #Predicate {
                $0.wallet.walletId == walletId
                    && $0.accountType == typeTag
                    && $0.accountIndex == index
            }
        )
        let matches = (try? backgroundContext.fetch(descriptor)) ?? []
        return matches.first { acc in
            acc.standardTag == key.standardTag
                && acc.registrationIndex == key.registrationIndex
                && acc.keyClass == key.keyClass
                && acc.userIdentityId == key.userIdentityId
                && acc.friendIdentityId == key.friendIdentityId
        }
    }

    // MARK: - Watch-only Restore: Wallet Metadata

    /// Set network + birth height on the `PersistentWallet` row. Fires
    /// once at wallet registration with values the Rust side can
    /// contribute but Swift can't easily recompute (network is on the
    /// manager's SDK; birth height is SPV's confirmed tip at creation).
    func persistWalletMetadata(walletId: Data, network: Network, birthHeight: UInt32) {
        onQueue {
            let wallet = ensureWalletRecord(walletId: walletId)
            wallet.network = network
            wallet.birthHeight = birthHeight
            wallet.lastUpdated = Date()
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    /// Set the user-facing name on the `PersistentWallet` row.
    /// Called from `PlatformWalletManager.createWallet` after the FFI
    /// returns a wallet id; only Swift knows the name, so it doesn't
    /// travel through a Rust-side callback. Silently skips if the row
    /// is missing (wallet wasn't successfully registered).
    public func setWalletName(walletId: Data, name: String) {
        onQueue {
            guard let wallet = findWalletRecord(walletId: walletId) else { return }
            wallet.name = name
            wallet.lastUpdated = Date()
            try? backgroundContext.save()
        }
    }

    public func identityIdsForWallet(walletId: Data) throws -> [Data] {
        try onQueue {
            let descriptor = FetchDescriptor<PersistentWallet>(
                predicate: PersistentWallet.predicate(walletId: walletId)
            )
            guard let walletRow = try backgroundContext.fetch(descriptor).first else {
                return []
            }
            return walletRow.identities.map { $0.identityId }
        }
    }

    /// Wipe a wallet's SwiftData footprint.
    public func deleteWalletData(walletId: Data) throws {
        try onQueue {
            do {
                let walletDescriptor = FetchDescriptor<PersistentWallet>(
                    predicate: PersistentWallet.predicate(walletId: walletId)
                )
                let walletRow = try backgroundContext.fetch(walletDescriptor).first
                let walletNetwork = walletRow?.network

                if let walletRow = walletRow {
                    // Wallet identity relationships are `.nullify`; this delete path cascades them explicitly.
                    let identitiesToDelete = Array(walletRow.identities)
                    let identityIds = identitiesToDelete.map { $0.identityId }

                    for identityId in identityIds {
                        let balanceDescriptor = FetchDescriptor<PersistentTokenBalance>(
                            predicate: PersistentTokenBalance.predicate(identityId: identityId)
                        )
                        for row in try backgroundContext.fetch(balanceDescriptor) {
                            backgroundContext.delete(row)
                        }
                    }

                    for identity in identitiesToDelete {
                        backgroundContext.delete(identity)
                    }
                }

                let txoDescriptor = FetchDescriptor<PersistentTxo>(
                    predicate: #Predicate<PersistentTxo> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(txoDescriptor) {
                    backgroundContext.delete(row)
                }

                let pendingDescriptor = FetchDescriptor<PersistentPendingInput>(
                    predicate: #Predicate<PersistentPendingInput> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(pendingDescriptor) {
                    backgroundContext.delete(row)
                }

                // `loadCachedAssetLocksOnQueue` rehydrates these rows on
                // the wallet-load path back into the Rust-side
                // `unused_asset_locks` map so an in-flight registration
                // can resume across an app kill. Without this cleanup,
                // delete-then-reimport of the same wallet would
                // resurrect stale Pending / Resumable asset-lock state
                // that the user thought they had wiped.
                let assetLockDescriptor = FetchDescriptor<PersistentAssetLock>(
                    predicate: #Predicate<PersistentAssetLock> { $0.walletId == walletId }
                )
                for row in try backgroundContext.fetch(assetLockDescriptor) {
                    backgroundContext.delete(row)
                }

                if let walletRow = walletRow {
                    backgroundContext.delete(walletRow)
                }

                try backgroundContext.save()

                let txRows = try backgroundContext.fetch(FetchDescriptor<PersistentTransaction>())
                for tx in txRows where tx.outputs.isEmpty &&
                    tx.inputs.isEmpty &&
                    tx.pendingInputs.isEmpty {
                    backgroundContext.delete(tx)
                }

                if let walletNetwork = walletNetwork {
                    let networkRaw = walletNetwork.rawValue
                    let siblingDescriptor = FetchDescriptor<PersistentWallet>(
                        predicate: #Predicate<PersistentWallet> { $0.networkRaw == networkRaw }
                    )
                    let remaining = try backgroundContext.fetch(siblingDescriptor)
                        .filter { $0.walletId != walletId }
                    if remaining.isEmpty {
                        let scopeId = syncStateScopeId(for: walletNetwork)
                        let syncDescriptor = FetchDescriptor<PersistentPlatformAddressesSyncState>(
                            predicate: #Predicate { $0.walletId == scopeId }
                        )
                        if let syncRow = try backgroundContext.fetch(syncDescriptor).first {
                            backgroundContext.delete(syncRow)
                        }
                    }
                }

                try backgroundContext.save()
            } catch {
                backgroundContext.rollback()
                throw error
            }
        }
    }

    // MARK: - Watch-only Restore: Account xpub

    /// Upsert a `PersistentAccount` row with the full `AccountSpecFFI`
    /// payload. Key is `(walletId, type_tag, index, registration_index,
    /// key_class, user_identity_id, friend_identity_id)` — everything
    /// that uniquely identifies an account across variants.
    func persistAccount(walletId: Data, spec: AccountSpecFFI) {
        onQueue {
            guard let wallet = findWalletRecord(walletId: walletId) else { return }
            let typeTag = UInt32(spec.type_tag)
            let index = spec.index
            let registrationIndex = spec.registration_index
            let keyClass = spec.key_class
            var userIdentityId = Data(count: 32)
            withUnsafeBytes(of: spec.user_identity_id) { src in
                userIdentityId.withUnsafeMutableBytes { dst in
                    dst.copyMemory(from: src)
                }
            }
            var friendIdentityId = Data(count: 32)
            withUnsafeBytes(of: spec.friend_identity_id) { src in
                friendIdentityId.withUnsafeMutableBytes { dst in
                    dst.copyMemory(from: src)
                }
            }
            let xpubBytes: Data
            if let xpubPtr = spec.account_xpub_bytes, spec.account_xpub_bytes_len > 0 {
                xpubBytes = Data(bytes: xpubPtr, count: Int(spec.account_xpub_bytes_len))
            } else {
                xpubBytes = Data()
            }

            // Upsert keyed by the full account identity. We can't easily
            // express the identity tuple in a #Predicate with local `Data`
            // captures, so fetch by (walletId, accountType, accountIndex)
            // and verify the richer fields in Swift.
            let descriptor = FetchDescriptor<PersistentAccount>(
                predicate: #Predicate {
                    $0.wallet.walletId == walletId
                        && $0.accountType == typeTag
                        && $0.accountIndex == index
                }
            )
            let existing = (try? backgroundContext.fetch(descriptor)) ?? []
            let match = existing.first { acc in
                // `standardTag` splits Standard accounts into BIP44 (0)
                // and BIP32 (1) variants. Without it, the second emit
                // (whichever the Rust side serializes last) silently
                // aliases onto the first row and the BIP32 account is
                // never persisted as its own record.
                acc.standardTag == spec.standard_tag
                    && acc.registrationIndex == registrationIndex
                    && acc.keyClass == keyClass
                    && acc.userIdentityId == userIdentityId
                    && acc.friendIdentityId == friendIdentityId
            }
            let account: PersistentAccount
            if let match = match {
                account = match
            } else {
                account = PersistentAccount(
                    wallet: wallet,
                    accountType: typeTag,
                    accountIndex: index,
                    accountTypeName: accountTypeName(
                        for: spec.type_tag,
                        standardTag: spec.standard_tag
                    )
                )
                backgroundContext.insert(account)
            }
            account.standardTag = spec.standard_tag
            account.registrationIndex = registrationIndex
            account.keyClass = keyClass
            account.userIdentityId = userIdentityId
            account.friendIdentityId = friendIdentityId
            account.accountExtendedPubKeyBytes = xpubBytes
            account.lastUpdated = Date()
            if !self.inChangeset { try? backgroundContext.save() }
        }
    }

    // MARK: - Watch-only Restore: Load

    /// Enumerate persisted wallets into heap-allocated `WalletRestoreEntryFFI[]`.
    ///
    /// Ownership: Swift owns every allocation returned and retains them
    /// on `self.loadAllocations` keyed by the entries pointer. Rust
    /// calls `loadWalletListFree` exactly once after it's done reading,
    /// at which point we release the allocations.
    ///
    /// A wallet is "restorable" when it has at least one
    /// `PersistentAccount` row with non-empty
    /// `accountExtendedPubKeyBytes`. The Rust side reconstructs the
    /// watch-only `Wallet` via `Wallet::new_watch_only(network,
    /// wallet_id, accounts)`; accounts come directly from the spec
    /// array, wallet id from the top-level struct.
    ///
    /// Returns `(nil, 0)` if nothing is restorable.
    func loadWalletList() -> (entries: UnsafePointer<WalletRestoreEntryFFI>?, count: Int, errored: Bool) {
        onQueue {
        // Scope the fetch to the handler's bound network so a
        // per-network manager only sees its own wallets. If
        // `network` is `nil` (legacy callers that haven't threaded
        // network through yet) we fall back to the cross-network
        // fetch — those callers were already fragile against
        // cross-network data and the new path keeps them on the
        // pre-refactor behavior until they migrate.
        let walletDescriptor: FetchDescriptor<PersistentWallet>
        if let network = self.network {
            let raw = network.rawValue
            walletDescriptor = FetchDescriptor<PersistentWallet>(
                predicate: #Predicate { $0.networkRaw == raw }
            )
        } else {
            walletDescriptor = FetchDescriptor<PersistentWallet>()
        }
        let wallets: [PersistentWallet]
        do {
            wallets = try backgroundContext.fetch(walletDescriptor)
        } catch {
            // Surfacing the SwiftData failure to Rust is critical —
            // returning success-with-empty here would let restore
            // appear to "succeed" with zero wallets, hiding a real
            // database fault from the user. The callback returns
            // non-zero on `errored == true`.
            NSLog(
                "[persistor-load:swift] PersistentWallet fetch failed: %@",
                String(describing: error)
            )
            return (nil, 0, true)
        }
        let restorable = wallets.filter { wallet in
            wallet.accounts.contains { ($0.accountExtendedPubKeyBytes?.isEmpty == false) }
        }
        if restorable.isEmpty {
            return (nil, 0, false)
        }

        // Single bucketed fetch of every unspent `PersistentTxo` so
        // each wallet's per-iteration buffer build is a dictionary
        // lookup instead of a fresh database round-trip. Prefetches
        // `account.wallet` to keep the legacy-walletId routing path
        // (rows whose `walletId` field defaults to `Data()` because
        // they predate the denorm) from triggering one SwiftData
        // fault per row when we resolve the parent wallet.
        //
        // The fetch happens BEFORE we allocate `entriesPtr` /
        // `LoadAllocation` so an early fetch failure doesn't leak
        // the entries buffer (`LoadAllocation.release` is only
        // called on the path through `loadAllocations` after the
        // pointer hand-off to Rust succeeds).
        var unspentBuckets: [Data: [PersistentTxo]] = [:]
        do {
            var unspentDescriptor = FetchDescriptor<PersistentTxo>(
                predicate: #Predicate { $0.isSpent == false }
            )
            unspentDescriptor.relationshipKeyPathsForPrefetching = [\.account]
            // Bail with `errored = true` on a SwiftData failure rather
            // than degrading to an empty bucket map. Without this, Rust
            // would see `entry.utxos_count == 0` for every wallet,
            // skip `wallet_info.update_balance()`, and the restore
            // would silently report zero core-chain funds — exactly
            // the failure mode this code path was added to eliminate.
            let unspent: [PersistentTxo]
            do {
                unspent = try backgroundContext.fetch(unspentDescriptor)
            } catch {
                NSLog(
                    "[persistor-load:swift] PersistentTxo unspent fetch failed: %@",
                    String(describing: error)
                )
                return (nil, 0, true)
            }
            unspentBuckets.reserveCapacity(restorable.count)
            for row in unspent {
                guard row.account != nil else { continue }
                let key: Data
                if !row.walletId.isEmpty {
                    key = row.walletId
                } else if let account = row.account {
                    // `account.wallet` is non-optional on the
                    // model but is a fault-loaded relationship;
                    // a relationship-store inconsistency would
                    // crash here, so guard via Optional cast.
                    let wallet: PersistentWallet? = account.wallet
                    guard let resolved = wallet else { continue }
                    key = resolved.walletId
                } else {
                    continue
                }
                unspentBuckets[key, default: []].append(row)
            }
        }

        // Allocate `entriesPtr` and the `LoadAllocation` here — past
        // the fallible SwiftData fetch above — so an early-error path
        // doesn't leak the entries buffer (LoadAllocation only gets
        // released through the `loadAllocations` map after the
        // successful pointer hand-off at the bottom of this fn).
        let allocation = LoadAllocation()
        let entriesPtr = UnsafeMutablePointer<WalletRestoreEntryFFI>.allocate(
            capacity: restorable.count
        )
        allocation.entries = entriesPtr
        allocation.entriesCount = restorable.count

        for (i, w) in restorable.enumerated() {
            let sortedAccounts = w.accounts
                .filter { ($0.accountExtendedPubKeyBytes?.isEmpty == false) }
                .sorted {
                    ($0.accountType, $0.accountIndex, $0.registrationIndex, $0.keyClass)
                        < ($1.accountType, $1.accountIndex, $1.registrationIndex, $1.keyClass)
                }
            let accountsBuffer: UnsafeMutablePointer<AccountSpecFFI>?
            let accountsWritten: Int
            if sortedAccounts.isEmpty {
                accountsBuffer = nil
                accountsWritten = 0
            } else {
                let buf = UnsafeMutablePointer<AccountSpecFFI>.allocate(capacity: sortedAccounts.count)
                var written = 0
                for acc in sortedAccounts {
                    // Filter above guarantees non-nil + non-empty.
                    let xpub = acc.accountExtendedPubKeyBytes ?? Data()
                    // Reject rows whose `accountType` (UInt32) doesn't
                    // fit in `u8`. `truncatingIfNeeded` would silently
                    // wrap a corrupt 0x100+ value into a potentially-
                    // valid tag in the 0–255 range, defeating Rust's
                    // `AccountTypeTagFFI::try_from_u8` validation.
                    //
                    // A `continue` here would silently drop a
                    // funds-bearing account from the snapshot and
                    // still report a successful restore — so abort
                    // the whole load callback instead. The Rust
                    // loader treats `errored = true` as a hard fail
                    // and won't construct a half-loaded manager.
                    guard let typeTagByte = UInt8(exactly: acc.accountType) else {
                        NSLog(
                            "[persistor-load:swift] aborting load: account row has accountType %u out of UInt8 range — refusing to silently drop it",
                            acc.accountType
                        )
                        buf.deallocate()
                        allocation.release()
                        return (nil, 0, true)
                    }
                    let xpubBuffer = UnsafeMutablePointer<UInt8>.allocate(capacity: xpub.count)
                    xpub.copyBytes(to: xpubBuffer, count: xpub.count)
                    allocation.scalarBuffers.append((xpubBuffer, xpub.count))

                    var spec = AccountSpecFFI()
                    spec.type_tag = typeTagByte
                    spec.standard_tag = acc.standardTag
                    spec.index = acc.accountIndex
                    spec.registration_index = acc.registrationIndex
                    spec.key_class = acc.keyClass
                    copyBytes(acc.userIdentityId, into: &spec.user_identity_id)
                    copyBytes(acc.friendIdentityId, into: &spec.friend_identity_id)
                    spec.account_xpub_bytes = UnsafePointer(xpubBuffer)
                    spec.account_xpub_bytes_len = UInt(xpub.count)
                    buf[written] = spec
                    written += 1
                }
                if written == 0 {
                    buf.deallocate()
                    accountsBuffer = nil
                    accountsWritten = 0
                } else {
                    accountsBuffer = buf
                    accountsWritten = written
                    allocation.accountArrays.append((buf, written))
                }
            }

            let cachedBalances = loadCachedBalancesOnQueue(walletId: w.walletId)
            // Compact-write into the buffer with a `written` counter so
            // a malformed row (`hash.count != 20`) doesn't leave an
            // uninitialized slot in the published slice. Rust reads
            // exactly `entry.platform_address_balances_count` entries
            // from the pointer; any uninit slot would be undefined
            // behaviour. Same pattern the UTXO loader below uses.
            let addressBalancesBuffer: UnsafeMutablePointer<AddressBalanceEntryFFI>?
            let addressBalancesWritten: Int
            if cachedBalances.isEmpty {
                addressBalancesBuffer = nil
                addressBalancesWritten = 0
            } else {
                let buf = UnsafeMutablePointer<AddressBalanceEntryFFI>.allocate(
                    capacity: cachedBalances.count
                )
                var written = 0
                for cached in cachedBalances {
                    let (addressType, hash, balance, nonce, accountIndex, addressIndex) = cached
                    guard hash.count == 20 else { continue }

                    var hashTuple:
                        (
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
                        ) = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
                    withUnsafeMutableBytes(of: &hashTuple) { raw in
                        raw.copyBytes(from: hash)
                    }

                    buf[written] = AddressBalanceEntryFFI(
                        address: PlatformAddressFFI(address_type: addressType, hash: hashTuple),
                        balance: balance,
                        nonce: nonce,
                        account_index: accountIndex,
                        address_index: addressIndex
                    )
                    written += 1
                }
                if written == 0 {
                    buf.deallocate()
                    addressBalancesBuffer = nil
                    addressBalancesWritten = 0
                } else {
                    addressBalancesBuffer = buf
                    addressBalancesWritten = written
                    allocation.addressBalanceArrays.append((buf, written))
                }
            }

            let syncState = w.network.flatMap { loadCachedSyncStateOnQueue(network: $0) }

            // Identity slice. Sorted by `identityIndex` then
            // `identityId` so the rehydrated `IndexMap` order is
            // deterministic across launches; the explorer paginates by
            // index, so the order matters for stable rendering.
            let sortedIdentities = w.identities.sorted {
                if $0.identityIndex != $1.identityIndex {
                    return $0.identityIndex < $1.identityIndex
                }
                return $0.identityId.lexicographicallyPrecedes($1.identityId)
            }
            let identitiesBuffer = buildIdentityRestoreBuffer(
                identities: sortedIdentities,
                allocation: allocation
            )

            var entry = WalletRestoreEntryFFI()
            copyBytes(w.walletId, into: &entry.wallet_id)
            entry.network = (w.network ?? .testnet).ffiValue
            entry.accounts = accountsBuffer.map { UnsafePointer($0) }
            entry.accounts_count = UInt(accountsWritten)
            entry.platform_address_balances = addressBalancesBuffer.map { UnsafePointer($0) }
            entry.platform_address_balances_count = UInt(addressBalancesWritten)
            entry.platform_sync_height = syncState?.syncHeight ?? 0
            entry.platform_sync_timestamp = syncState?.syncTimestamp ?? 0
            entry.platform_last_known_recent_block = syncState?.lastKnownRecentBlock ?? 0
            entry.identities = identitiesBuffer.map { UnsafePointer($0) }
            entry.identities_count = UInt(sortedIdentities.count)
            // Core-chain sync metadata. `PersistentWallet` doesn't
            // carry a separate `lastProcessedHeight` column today;
            // for non-pruning SPV wallets the two heights advance in
            // lockstep at runtime, so re-using `syncedHeight` keeps
            // the restored wallet aligned with the runtime invariant.
            // Sending `0` here would leave `metadata.last_processed_height`
            // at `birth_height - 1` after restore, which mis-buckets
            // matured coinbase outputs as immature in
            // `update_balance` until SPV next advances. The proper
            // fix is a dedicated column on `PersistentWallet` —
            // tracked separately.
            entry.birth_height = w.birthHeight
            entry.synced_height = w.syncedHeight
            entry.last_processed_height = w.syncedHeight
            entry.last_synced = w.lastSynced

            // Persisted `last_applied_chain_lock` bincode bytes from
            // the previous session. Rust's `build_wallet_start_state`
            // decodes these and stamps `wallet_info.metadata.
            // last_applied_chain_lock`, so the asset-lock-resume
            // CL-from-metadata fallback in `proof.rs` can fire on
            // catch-up tasks at app launch without waiting for SPV
            // to re-apply a fresh chainlock. Wallets that have
            // never observed a chainlock (fresh creations,
            // pre-feature rows) carry `nil` here and the FFI fields
            // stay null / zero — Rust load falls back to leaving
            // `metadata.last_applied_chain_lock = None`.
            if let clBytes = w.lastAppliedChainLockBytes, !clBytes.isEmpty {
                let buffer = UnsafeMutablePointer<UInt8>.allocate(
                    capacity: clBytes.count
                )
                clBytes.copyBytes(to: buffer, count: clBytes.count)
                allocation.scalarBuffers.append((buffer, clBytes.count))
                entry.last_applied_chain_lock_bytes = UnsafePointer(buffer)
                entry.last_applied_chain_lock_bytes_len = UInt(clBytes.count)
            } else {
                entry.last_applied_chain_lock_bytes = nil
                entry.last_applied_chain_lock_bytes_len = 0
            }

            // Persisted unspent UTXOs for this wallet. The SPV inbound
            // path writes `PersistentTxo` rows and flips `isSpent`
            // (rather than deleting) on spend, so the unspent set is
            // exactly `isSpent == false`. Rust routes each row into
            // the matching funds-bearing account by tag; rows whose
            // account isn't a funds variant get silently skipped on
            // the receiving side.
            let (utxoBuf, utxoCount, utxoErrored) = buildUtxoRestoreBuffer(
                rows: unspentBuckets[w.walletId] ?? [],
                allocation: allocation
            )
            // `buildUtxoRestoreBuffer` already deallocated its own
            // buffer on the errored path; release everything else
            // we've accumulated and abort the load callback so Rust
            // doesn't see a partial / dropped-row snapshot.
            if utxoErrored {
                allocation.release()
                return (nil, 0, true)
            }
            entry.utxos = utxoBuf.map { UnsafePointer($0) }
            entry.utxos_count = UInt(utxoCount)

            let (poolBuf, poolCount, poolErrored) = buildCoreAddressPoolBuffer(
                accounts: sortedAccounts,
                allocation: allocation
            )
            if poolErrored {
                allocation.release()
                return (nil, 0, true)
            }
            entry.core_address_pools = poolBuf.map { UnsafePointer($0) }
            entry.core_address_pools_count = UInt(poolCount)

            // Tracked asset-lock rows. The Rust side rehydrates these
            // into `unused_asset_locks` so an in-flight registration
            // that was killed mid-flight can resume from the latest
            // status without rebroadcasting. Empty / null when the
            // wallet has no persisted locks.
            let assetLockRows = loadCachedAssetLocksOnQueue(walletId: w.walletId)
            let (assetLockBuf, assetLockCount) = buildAssetLockRestoreBuffer(
                rows: assetLockRows,
                allocation: allocation
            )
            entry.tracked_asset_locks = assetLockBuf.map { UnsafePointer($0) }
            entry.tracked_asset_locks_count = UInt(assetLockCount)

            // Funding tx records for asset locks at `statusRaw < 2`
            // (Built / Broadcast). The Rust load path re-inserts each
            // entry into the matching `standard_bip44_accounts[
            // account_index].transactions_mut()` bucket so the next
            // incoming chain-lock event can cascade-promote them.
            // Without this, the in-memory transactions map starts
            // empty after every restart, `apply_chain_lock` finds
            // nothing to promote at that height, and any asset lock
            // whose funding block has already been chain-locked
            // stays stuck at `Broadcast` indefinitely.
            //
            // Rows are filtered to `statusRaw < 2` so already-IS-
            // locked / already-chain-locked locks (which already
            // carry their proof on the `PersistentAssetLock` row and
            // don't need cascade-promotion) don't take up FFI
            // bandwidth. Empty / null when the wallet has no
            // unresolved locks.
            let (unresolvedBuf, unresolvedCount) =
                buildUnresolvedAssetLockTxRecordBuffer(
                    walletId: w.walletId,
                    allocation: allocation
                )
            entry.unresolved_asset_lock_tx_records = unresolvedBuf.map { UnsafePointer($0) }
            entry.unresolved_asset_lock_tx_records_count = UInt(unresolvedCount)

            // Primary-identity selection + gap-limit scan watermark
            // were dropped from the FFI shape — both moved off the
            // Rust manager (UI owns selection now, scan resume is
            // derived from the highest already-registered slot).
            entriesPtr[i] = entry
            // Bump the initialized-count so a later abort path's
            // `release()` only deinitializes slots that were
            // actually written (see `entriesInitialized`'s
            // doc-comment for why we can't reuse `entriesCount`).
            allocation.entriesInitialized = i + 1
        }

        let typed = UnsafePointer(entriesPtr)
        loadAllocations[UnsafeRawPointer(typed)] = allocation
        return (typed, restorable.count, false)
        }  // onQueue
    }

    /// Allocate a contiguous `[IdentityRestoreEntryFFI]` buffer for
    /// one wallet's identities and stash every nested allocation on
    /// `allocation` so the matching free callback can release them.
    ///
    /// Returns `nil` for an empty input — Rust treats `null` +
    /// `count == 0` as "no identities for this wallet".
    ///
    /// Every entry on a wallet's list is wallet-owned by definition
    /// (the per-identity `is_watched` flag was dropped along with the
    /// underlying `WatchedIdentity` type). The Rust side files each
    /// entry into `wallet_identities[wallet_id][identity_index]`.
    ///
    /// `dpns_names` / `contested_dpns_names` / `alias` aren't
    /// reflected here today — the sync path drops them on the floor.
    /// Both arrays come back as zero length and a `null` outer
    /// pointer. They're wired up so a future SwiftData column for
    /// either list is one-line work. The user-facing `alias` lives
    /// on `PersistentIdentity` and is read directly by the UI; it
    /// no longer roundtrips through Rust now that `ManagedIdentity`
    /// dropped its `label` field.
    /// Build a contiguous `[UtxoRestoreEntryFFI]` buffer for one
    /// wallet's unspent UTXOs. Walks `PersistentTxo` rows scoped to
    /// `walletId` and `isSpent == false`, copies the account-tag
    /// fields off the parent `PersistentAccount`, and emits one row
    /// per UTXO. Returns `(nil, 0)` for empty input — Rust treats
    /// `null` + `count == 0` as "no UTXOs to restore".
    ///
    /// Per-row script_pubkey buffers and the outer array are tracked
    /// on `allocation` so `loadWalletListFree` can release them.
    /// Rows whose `outpoint` payload isn't 32 bytes are skipped — the
    /// model stores it as `Data` (`outpoint: Data`) and bad data
    /// shouldn't crash the FFI handoff.
    /// Build the per-wallet UTXO restore buffer from a list of
    /// `PersistentTxo` rows already bucketed for this wallet by the
    /// caller. The bucketing pass in `loadWalletList` does the
    /// SwiftData fetch once for the whole batch (legacy empty-walletId
    /// rows route via `account.wallet.walletId`), so this function is
    /// pure marshalling.
    private func buildUtxoRestoreBuffer(
        rows: [PersistentTxo],
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<UtxoRestoreEntryFFI>?, Int, Bool) {
        if rows.isEmpty {
            return (nil, 0, false)
        }
        let buf = UnsafeMutablePointer<UtxoRestoreEntryFFI>.allocate(capacity: rows.count)
        var written = 0
        for record in rows {
            guard let account = record.account else { continue }
            // `outpoint` on `PersistentTxo` is 36 bytes (32-byte txid
            // followed by LE u32 vout) — composed via
            // `makeOutpoint(txid:vout:)`. Use the dedicated `txid`
            // accessor, which prefers `transaction.txid` and falls
            // back to `outpoint.prefix(32)` so storage-explorer rows
            // and the FFI handoff agree on the same 32-byte identity.
            //
            // A row whose `txid` doesn't measure 32 bytes is corrupt
            // by construction (the model guarantees the prefix on
            // every write). Treat it the same way as the corrupt
            // `accountType` case below — abort the whole load so the
            // caller can surface the error rather than silently
            // under-restoring the funds set. Symmetric handling
            // keeps the restore contract uniform.
            let txid = record.txid
            guard txid.count == 32 else {
                NSLog(
                    "[persistor-load:swift] aborting load: UTXO has txid of %d bytes (expected 32) — refusing to silently drop it",
                    txid.count
                )
                buf.deallocate()
                return (nil, 0, true)
            }
            // Reject UTXOs whose parent `accountType` (UInt32) doesn't
            // fit in `u8`. Truncating would silently wrap a corrupt
            // 0x100+ value into a potentially-valid tag in 0–255 and
            // bypass Rust's `try_from_u8` validation. Drop-and-continue
            // would also silently under-restore the funds set, so we
            // signal `errored = true` and let `loadWalletList` fail
            // the whole callback — the persisted snapshot is corrupt.
            guard let typeTagByte = UInt8(exactly: account.accountType) else {
                NSLog(
                    "[persistor-load:swift] aborting load: UTXO has parent accountType %u out of UInt8 range — refusing to silently drop it",
                    account.accountType
                )
                buf.deallocate()
                return (nil, 0, true)
            }

            // Allocate + copy the script_pubkey bytes. Empty scripts
            // pass through with a null pointer + zero len.
            let scriptBytes = record.scriptPubKey
            let scriptPtr: UnsafePointer<UInt8>?
            let scriptLen = scriptBytes.count
            if scriptLen > 0 {
                let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: scriptLen)
                scriptBytes.copyBytes(to: buffer, count: scriptLen)
                allocation.scalarBuffers.append((buffer, scriptLen))
                scriptPtr = UnsafePointer(buffer)
            } else {
                scriptPtr = nil
            }

            var utxo = UtxoRestoreEntryFFI()
            // Tag fields are FFI-typed `u8` and validated via
            // `try_from_u8` on the Rust side; pass the exact byte
            // we just guarded above.
            utxo.type_tag = typeTagByte
            utxo.standard_tag = account.standardTag
            utxo.account_index = account.accountIndex
            utxo.registration_index = account.registrationIndex
            utxo.key_class = account.keyClass
            copyBytes(account.userIdentityId, into: &utxo.user_identity_id)
            copyBytes(account.friendIdentityId, into: &utxo.friend_identity_id)
            copyBytes(txid, into: &utxo.prev_txid)
            utxo.vout = record.vout
            utxo.value_duffs = record.amount
            utxo.script_pubkey = scriptPtr
            utxo.script_pubkey_len = UInt(scriptLen)
            utxo.height = record.height
            utxo.is_coinbase = record.isCoinbase
            utxo.is_confirmed = record.isConfirmed
            utxo.is_instantlocked = record.isInstantLocked
            utxo.is_locked = record.isLocked
            buf[written] = utxo
            written += 1
        }
        if written == 0 {
            buf.deallocate()
            return (nil, 0, false)
        }
        allocation.utxoArrays.append((buf, written))
        return (buf, written, false)
    }

    /// Build a contiguous `[AccountAddressPoolFFI]` buffer for one
    /// wallet's persisted core address pools
    private func buildCoreAddressPoolBuffer(
        accounts: [PersistentAccount],
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<AccountAddressPoolFFI>?, Int, Bool) {
        var groups: [(account: PersistentAccount, poolTypeTag: UInt8, rows: [PersistentCoreAddress])] = []
        for account in accounts {
            if account.coreAddresses.isEmpty { continue }
            var byPool: [UInt8: [PersistentCoreAddress]] = [:]
            for addr in account.coreAddresses {
                byPool[addr.poolTypeTag, default: []].append(addr)
            }
            for (tag, rows) in byPool.sorted(by: { $0.key < $1.key }) {
                groups.append((account, tag, rows))
            }
        }
        if groups.isEmpty {
            return (nil, 0, false)
        }

        let buf = UnsafeMutablePointer<AccountAddressPoolFFI>.allocate(capacity: groups.count)
        var written = 0
        for group in groups {
            let account = group.account
            guard let typeTagByte = UInt8(exactly: account.accountType) else {
                NSLog(
                    "[persistor-load:swift] aborting load: address-pool account row has accountType %u out of UInt8 range",
                    account.accountType
                )
                buf.deallocate()
                return (nil, 0, true)
            }

            // Inner CoreAddressEntryFFI array — one row per address.
            let rowBuf = UnsafeMutablePointer<CoreAddressEntryFFI>.allocate(
                capacity: group.rows.count
            )
            for (j, row) in group.rows.enumerated() {
                var e = CoreAddressEntryFFI()
                copyBytes(row.publicKey, into: &e.public_key)
                e.has_public_key = (row.publicKey.count == 33)
                e.pool_type_tag = group.poolTypeTag
                e.address_index = row.addressIndex
                e.is_used = row.isUsed
                e.balance = row.balance
                e.address_base58 = UnsafePointer(
                    duplicateCString(row.address, allocation: allocation)
                )
                e.derivation_path = UnsafePointer(
                    duplicateCString(row.derivationPath, allocation: allocation)
                )
                rowBuf[j] = e
            }
            allocation.coreAddressEntryArrays.append((rowBuf, group.rows.count))

            var spec = AccountSpecFFI()
            spec.type_tag = typeTagByte
            spec.standard_tag = account.standardTag
            spec.index = account.accountIndex
            spec.registration_index = account.registrationIndex
            spec.key_class = account.keyClass
            copyBytes(account.userIdentityId, into: &spec.user_identity_id)
            copyBytes(account.friendIdentityId, into: &spec.friend_identity_id)
            spec.account_xpub_bytes = nil
            spec.account_xpub_bytes_len = 0

            var pool = AccountAddressPoolFFI()
            pool.account = spec
            pool.pool_type_tag = group.poolTypeTag
            pool.addresses_ptr = UnsafePointer(rowBuf)
            pool.addresses_count = UInt(group.rows.count)
            buf[written] = pool
            written += 1
        }
        allocation.coreAddressPoolArrays.append((buf, written))
        return (buf, written, false)
    }

    /// Build a contiguous `[AssetLockEntryFFI]` buffer for one wallet's
    /// tracked asset locks. Walks `PersistentAssetLock` rows scoped to
    /// `walletId`, copies the consensus-encoded transaction + optional
    /// bincode-encoded proof into Swift-owned heap buffers, and emits
    /// one row per lock. Returns `(nil, 0)` for empty input — Rust
    /// treats `null` + `count == 0` as "no tracked locks to restore".
    ///
    /// Per-row transaction/proof buffers and the outer array are
    /// tracked on `allocation` so `loadWalletListFree` releases them.
    /// Rows whose `outPointHex` doesn't parse back to 36 bytes are
    /// skipped — the model writes them in a known shape, so a
    /// mismatch indicates corruption that would crash Rust's decoder
    /// anyway.
    private func buildAssetLockRestoreBuffer(
        rows: [AssetLockEntrySnapshot],
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<AssetLockEntryFFI>?, Int) {
        if rows.isEmpty {
            return (nil, 0)
        }
        let buf = UnsafeMutablePointer<AssetLockEntryFFI>.allocate(capacity: rows.count)
        var written = 0
        for record in rows {
            // Parse `<txid_hex>:<vout>` back into the 36-byte raw form
            // the Rust side expects. Any parse failure drops the row
            // — we can't manufacture a valid outpoint and a malformed
            // row indicates an old / corrupt snapshot.
            guard let outPoint = decodeOutPointHex(record.outPointHex) else {
                NSLog(
                    "[persistor-load:swift] dropping asset-lock row with malformed outPointHex: %@",
                    record.outPointHex
                )
                continue
            }

            // Allocate + copy the transaction bytes (Rust-owned for
            // the callback window via the allocation).
            let txBytes = record.transactionBytes
            let txPtr: UnsafePointer<UInt8>?
            let txLen = txBytes.count
            if txLen > 0 {
                let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: txLen)
                txBytes.copyBytes(to: buffer, count: txLen)
                allocation.scalarBuffers.append((buffer, txLen))
                txPtr = UnsafePointer(buffer)
            } else {
                // A row with no transaction bytes is broken — Rust's
                // load path will reject it; drop here.
                NSLog(
                    "[persistor-load:swift] dropping asset-lock row with empty transactionBytes: %@",
                    record.outPointHex
                )
                continue
            }

            // Optional proof bytes.
            let proofPtr: UnsafePointer<UInt8>?
            let proofLen: Int
            if let bytes = record.proofBytes, !bytes.isEmpty {
                let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: bytes.count)
                bytes.copyBytes(to: buffer, count: bytes.count)
                allocation.scalarBuffers.append((buffer, bytes.count))
                proofPtr = UnsafePointer(buffer)
                proofLen = bytes.count
            } else {
                proofPtr = nil
                proofLen = 0
            }

            var entry = AssetLockEntryFFI()
            copyBytes(outPoint, into: &entry.out_point)
            entry.transaction_bytes = txPtr
            entry.transaction_bytes_len = UInt(txLen)
            // BIP44 account the funding tx was built from, captured
            // on every upsert. The Rust load path uses this value to
            // route the unresolved record back into the matching
            // `standard_bip44_accounts[account_index]` bucket — a
            // wrong value silently drops the record, which broke
            // restore for any wallet that funded an asset lock from
            // a non-zero account index. Pre-feature rows default to
            // 0 (matches the previous behavior; the only realistic
            // common case).
            entry.account_index = UInt32(bitPattern: record.accountIndexRaw)
            // Exact (not clamping) conversion: a corrupt persisted row
            // with `fundingTypeRaw` or `statusRaw` outside `0...255`
            // would be silently coerced to a valid-looking enum value
            // by `UInt8(clamping:)` (negative → 0 = Built / IdentityRegistration,
            // >255 → 255 = sentinel). Either drops or rewrites the
            // asset-lock's effective state. Skip the row instead,
            // logged loudly so an operator can see and fix the bad row.
            guard let fundingType = UInt8(exactly: record.fundingTypeRaw) else {
                NSLog(
                    "[persistor-load] dropping asset-lock row %@ — fundingTypeRaw out of u8 range: %d",
                    record.outPointHex,
                    record.fundingTypeRaw
                )
                continue
            }
            guard let status = UInt8(exactly: record.statusRaw) else {
                NSLog(
                    "[persistor-load] dropping asset-lock row %@ — statusRaw out of u8 range: %d",
                    record.outPointHex,
                    record.statusRaw
                )
                continue
            }
            entry.funding_type = fundingType
            entry.identity_index = UInt32(bitPattern: record.identityIndexRaw)
            entry.amount_duffs = UInt64(bitPattern: record.amountDuffs)
            entry.status = status
            entry.proof_bytes = proofPtr
            entry.proof_bytes_len = UInt(proofLen)
            buf[written] = entry
            written += 1
        }
        if written == 0 {
            buf.deallocate()
            return (nil, 0)
        }
        allocation.assetLockArrays.append((buf, written))
        return (buf, written)
    }

    /// Build the per-wallet `UnresolvedAssetLockTxRecordFFI` array
    /// for the load callback. One entry per `PersistentAssetLock` row
    /// at `statusRaw < 2` (Built / Broadcast) whose funding tx has a
    /// matching `PersistentTransaction` row. Returns `(nil, 0)` when
    /// there are no eligible rows.
    ///
    /// The Rust side reads each row and re-inserts the decoded
    /// transaction into the matching BIP44 account's in-memory
    /// `transactions()` map so the next chain-lock event can promote
    /// it via `apply_chain_lock`. See
    /// `restore_unresolved_asset_lock_tx_records` for the Rust-side
    /// contract.
    ///
    /// Rows with no matching `PersistentTransaction` (e.g. an
    /// orphaned asset-lock row whose tx never made it into the
    /// transaction table) are skipped — the Rust side has no way to
    /// reconstruct the funding tx without its consensus bytes, so
    /// projecting an empty row would just bloat the FFI surface.
    private func buildUnresolvedAssetLockTxRecordBuffer(
        walletId: Data,
        allocation: LoadAllocation
    ) -> (UnsafeMutablePointer<UnresolvedAssetLockTxRecordFFI>?, Int) {
        // Filter to `statusRaw < 2` so already-IS-locked /
        // already-chain-locked rows don't end up in the array —
        // those locks have their proof bytes persisted on the
        // `PersistentAssetLock` row and the Rust side doesn't need
        // the funding tx in the in-memory map to use them.
        let descriptor = FetchDescriptor<PersistentAssetLock>(
            predicate: #Predicate { entry in
                entry.walletId == walletId && entry.statusRaw < 2
            }
        )
        guard let locks = try? backgroundContext.fetch(descriptor), !locks.isEmpty else {
            return (nil, 0)
        }

        // Pre-query the matching `PersistentTransaction` rows.
        // `PersistentAssetLock.outPointHex` carries the txid in
        // display order; `PersistentTransaction.txid` is wire order
        // — the same flip `decodeOutPointHex` already performs.
        let buf = UnsafeMutablePointer<UnresolvedAssetLockTxRecordFFI>.allocate(
            capacity: locks.count
        )
        var written = 0
        for lock in locks {
            guard let outpoint = decodeOutPointHex(lock.outPointHex) else {
                continue
            }
            let txid = outpoint.prefix(32)
            let txidData = Data(txid)
            let txDescriptor = FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == txidData }
            )
            guard let txRow = try? backgroundContext.fetch(txDescriptor).first else {
                // No matching tx — Rust can't reconstruct the
                // funding body without its consensus bytes. Skip.
                continue
            }
            let txBytes = txRow.transactionData
            guard !txBytes.isEmpty else {
                // A stub row whose real upsert never arrived;
                // skip rather than emit an undecodable buffer.
                continue
            }

            // Allocate the consensus-bytes buffer. Lifetime is
            // owned by `allocation.scalarBuffers`, freed by
            // `LoadAllocation.release()` after Rust returns.
            let txBuf = UnsafeMutablePointer<UInt8>.allocate(capacity: txBytes.count)
            txBytes.copyBytes(to: txBuf, count: txBytes.count)
            allocation.scalarBuffers.append((txBuf, txBytes.count))

            var entry = UnresolvedAssetLockTxRecordFFI()
            // Use the row's persisted `accountIndexRaw` — the Rust
            // side looks up `standard_bip44_accounts.get(&account_index)`
            // and silently drops the restore if the account doesn't
            // exist, so passing the actual funding account is
            // load-bearing for any wallet that funded an asset lock
            // from a non-zero BIP44 account index.
            entry.account_index = UInt32(bitPattern: lock.accountIndexRaw)
            entry.tx_bytes = txBuf
            entry.tx_bytes_len = UInt(txBytes.count)
            entry.context_raw = txRow.context
            entry.block_height = txRow.blockHeight
            if let hash = txRow.blockHash, hash.count == 32 {
                withUnsafeMutableBytes(of: &entry.block_hash) { raw in
                    raw.copyBytes(from: hash)
                }
            }
            entry.block_timestamp = UInt64(txRow.blockTimestamp)
            entry.first_seen = txRow.firstSeen
            buf[written] = entry
            written += 1
        }
        if written == 0 {
            buf.deallocate()
            return (nil, 0)
        }
        allocation.unresolvedAssetLockTxRecordArrays.append((buf, written))
        return (buf, written)
    }

    /// Parse `<txid_hex (display order)>:<vout>` back into the 36-byte
    /// raw outpoint Rust expects (32-byte raw txid + 4-byte
    /// little-endian vout). Mirror of
    /// `PersistentAssetLock.encodeOutPoint`. Returns `nil` for any
    /// parse failure.
    private func decodeOutPointHex(_ hex: String) -> Data? {
        let parts = hex.split(separator: ":", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2 else { return nil }
        let txidHex = String(parts[0])
        guard let vout = UInt32(parts[1]) else { return nil }
        guard txidHex.count == 64 else { return nil }
        var txid = Data(capacity: 32)
        var idx = txidHex.startIndex
        for _ in 0..<32 {
            let end = txidHex.index(idx, offsetBy: 2)
            guard let byte = UInt8(txidHex[idx..<end], radix: 16) else { return nil }
            txid.append(byte)
            idx = end
        }
        // Reverse from display-order back to raw byte order.
        let raw = Data(txid.reversed())
        var out = Data(raw)
        out.append(contentsOf: withUnsafeBytes(of: vout.littleEndian) { Data($0) })
        return out
    }

    private func buildIdentityRestoreBuffer(
        identities: [PersistentIdentity],
        allocation: LoadAllocation
    ) -> UnsafeMutablePointer<IdentityRestoreEntryFFI>? {
        if identities.isEmpty {
            return nil
        }
        let buf = UnsafeMutablePointer<IdentityRestoreEntryFFI>.allocate(
            capacity: identities.count
        )
        for (j, identity) in identities.enumerated() {
            var entry = IdentityRestoreEntryFFI()
            copyBytes(identity.identityId, into: &entry.identity_id)
            // `PersistentIdentity` stores balance / revision as Int64
            // bit-pattern (matches how `persistIdentities` writes them).
            // Round-trip them as the same UInt64 bit-pattern.
            entry.balance = UInt64(bitPattern: identity.balance)
            entry.revision = UInt64(bitPattern: identity.revision)
            entry.identity_index = identity.identityIndex
            // Status isn't persisted today (no `status` column on
            // `PersistentIdentity`); fall back to `Unknown` (0). The
            // next identity sync round will re-stamp it via the
            // identity changeset path.
            entry.status = 0

            // DPNS names — currently empty. Wiring is here so a
            // future query against `PersistentDpnsName` rows (or a
            // dedicated array column on the identity) drops in
            // without touching the FFI plumbing.
            entry.dpns_names = nil
            entry.dpns_names_count = 0
            entry.contested_dpns_names = nil
            entry.contested_dpns_names_count = 0

            // Public keys — read the per-identity `PersistentPublicKey`
            // rows (relationship navigated directly; the rows are
            // fetched lazily by SwiftData but live in the same
            // background context as the identity row so the access is
            // synchronous). Sort by `keyId` so the BTreeMap that gets
            // built on the Rust side keeps a deterministic order.
            let sortedKeys = identity.publicKeys.sorted { $0.keyId < $1.keyId }
            if sortedKeys.isEmpty {
                entry.keys = nil
                entry.keys_count = 0
            } else {
                let keyBuf = UnsafeMutablePointer<IdentityKeyRestoreFFI>.allocate(
                    capacity: sortedKeys.count
                )
                for (k, pk) in sortedKeys.enumerated() {
                    var row = IdentityKeyRestoreFFI()
                    row.key_id = UInt32(bitPattern: pk.keyId)
                    // PersistentPublicKey stores the discriminants as
                    // `String(rawValue)` of the original `UInt8` —
                    // same shape as the `purposeEnum` /
                    // `securityLevelEnum` / `keyTypeEnum` accessors on
                    // the model. Decode back to `UInt8`; fall back to
                    // `UInt8.max` (an out-of-range sentinel) on parse
                    // failure so Rust's
                    // `KeyType::try_from(u8)` /
                    // `Purpose::try_from(u8)` /
                    // `SecurityLevel::try_from(u8)` rejects the row
                    // and `build_identity_public_keys` drops it. The
                    // prior fallback (`?? 0`) silently coerced
                    // corrupt rows into ECDSA_SECP256K1 / AUTHENTICATION
                    // / MASTER — a far worse outcome than a clean
                    // skip-and-continue.
                    row.key_type = UInt8(pk.keyType) ?? UInt8.max
                    row.purpose = UInt8(pk.purpose) ?? UInt8.max
                    row.security_level = UInt8(pk.securityLevel) ?? UInt8.max
                    row.read_only = pk.readOnly

                    // Allocate a dedicated byte buffer for the public
                    // key data. Same lifetime convention as xpub
                    // bytes — released by `LoadAllocation.release`
                    // via the `scalarBuffers` list.
                    let len = pk.publicKeyData.count
                    if len > 0 {
                        let dataBuf = UnsafeMutablePointer<UInt8>.allocate(capacity: len)
                        pk.publicKeyData.copyBytes(to: dataBuf, count: len)
                        row.data = UnsafePointer(dataBuf)
                        row.data_len = UInt(len)
                        allocation.scalarBuffers.append((dataBuf, len))
                    } else {
                        row.data = nil
                        row.data_len = 0
                    }
                    keyBuf[k] = row
                }
                entry.keys = UnsafePointer(keyBuf)
                entry.keys_count = UInt(sortedKeys.count)
                allocation.identityKeyArrays.append((keyBuf, sortedKeys.count))
            }

            buf[j] = entry
        }
        allocation.identityArrays.append((buf, identities.count))
        return buf
    }

    /// Allocate a NUL-terminated UTF-8 c-string copy of `s` and stash
    /// it on `allocation` for release after the load callback returns.
    /// Empty strings are still allocated (round-trip with a
    /// `\0`-only buffer) — callers gate emission on `!isEmpty` before
    /// asking for one.
    private func duplicateCString(
        _ s: String,
        allocation: LoadAllocation
    ) -> UnsafeMutablePointer<CChar> {
        // `Array(s.utf8) + [0]` builds the byte sequence with the
        // trailing NUL Rust's `CStr::from_ptr` requires. We allocate
        // the pointer ourselves (rather than `strdup`) so the free
        // path can use `deallocate()` without coupling to libc.
        let utf8 = Array(s.utf8) + [0]
        let buf = UnsafeMutablePointer<CChar>.allocate(capacity: utf8.count)
        for (i, byte) in utf8.enumerated() {
            buf[i] = CChar(bitPattern: byte)
        }
        allocation.cStringBuffers.append((buf, utf8.count))
        return buf
    }

    /// Return the list of wallet ids that could be restored from
    /// SwiftData (i.e. have ≥1 account with a non-empty xpub). Used by
    /// `PlatformWalletManager.loadFromPersistor` after the FFI call
    /// succeeds so it can fetch a Swift-side handle for each wallet
    /// Rust just reconstructed.
    public func restorableWalletIds() -> [Data] {
        onQueue {
            let descriptor = FetchDescriptor<PersistentWallet>()
            guard let wallets = try? backgroundContext.fetch(descriptor) else {
                return []
            }
            return wallets
                .filter { w in
                    w.accounts.contains { ($0.accountExtendedPubKeyBytes?.isEmpty == false) }
                }
                .map { $0.walletId }
        }
    }

    /// Release all allocations for a given load-callback result.
    func loadWalletListFree(entries: UnsafeRawPointer?) {
        onQueue {
            guard let entries = entries,
                  let allocation = loadAllocations.removeValue(forKey: entries) else {
                return
            }
            allocation.release()
        }
    }

    /// Outstanding load-call allocations keyed by the entries pointer
    /// we handed to Rust. Drained by `loadWalletListFree`.
    private var loadAllocations: [UnsafeRawPointer: LoadAllocation] = [:]

    /// Human-readable name for a persisted account, mirroring the
    /// top-level `AccountTypeTagFFI` discriminant plus — for tag 0
    /// (Standard) — the `StandardAccountTypeTagFFI` sub-discriminant.
    /// BIP44 vs BIP32 gets folded into the name so the UI can
    /// distinguish them without reading `standardTag` separately.
    private func accountTypeName(for tag: UInt8, standardTag: UInt8) -> String {
        switch tag {
        case 0:
            switch standardTag {
            case 0: return "BIP44 Account"
            case 1: return "BIP32 Account"
            default: return "Standard Account(\(standardTag))"
            }
        case 1: return "CoinJoin"
        case 2: return "Identity Registration"
        case 3: return "Identity Top-Up"
        case 4: return "Identity Top-Up (Unbound)"
        case 5: return "Identity Invitation"
        case 6: return "Asset Lock Address Top-Up"
        case 7: return "Asset Lock Shielded Address Top-Up"
        case 8: return "Provider Voting Keys"
        case 9: return "Provider Owner Keys"
        case 10: return "Provider Operator Keys"
        case 11: return "Provider Platform Keys"
        case 12: return "DashPay Receiving Funds"
        case 13: return "DashPay External Account"
        case 14: return "Platform Payment"
        case 15: return "Identity Auth (ECDSA)"
        case 16: return "Identity Auth (BLS)"
        default: return "Unknown(\(tag))"
        }
    }

    /// Build the 32-byte synthetic walletId used as the uniqueness
    /// key for the per-network `PersistentPlatformAddressesSyncState` row. The content
    /// is "platform-sync:<networkName>" zero-padded to 32 bytes.
    private func syncStateScopeId(for network: Network) -> Data {
        let scopeString = "platform-sync:\(network.networkName)"
        var data = Data(scopeString.utf8.prefix(32))
        if data.count < 32 {
            data.append(Data(repeating: 0, count: 32 - data.count))
        }
        return data
    }

    /// Look up a transaction record for the asset-lock proof flow's
    /// persister fallback (Rust trait method
    /// `PlatformWalletPersistence::get_core_tx_record`).
    ///
    /// The Rust-side asset-lock proof flow needs the chain-lock
    /// height + block hash + timestamp to construct a
    /// `ChainAssetLockProof`. With upstream's
    /// `keep-finalized-transactions` Cargo feature OFF (the default),
    /// chain-locked records are evicted from the in-memory
    /// `transactions()` map, so the chain-lock metadata is no longer
    /// reachable through the wallet-info API. The persister received
    /// the record on the chain-lock-transition `store` call before
    /// eviction; this lookup walks the corresponding
    /// `PersistentTransaction` row.
    ///
    /// Returns the row's actual `context` discriminant alongside the
    /// block info (when applicable). The Rust side faithfully
    /// reconstructs the matching `TransactionContext` variant — no
    /// chain-lock filter here, so a row in any state may be
    /// returned. `blockHash` / `blockHeight` / `blockTimestamp` are
    /// only meaningful for `context` 2 (InBlock) and 3
    /// (InChainLockedBlock); the Rust side ignores those fields for
    /// 0 (Mempool) and 1 (InstantSend).
    ///
    /// Returns `nil` when no `PersistentTransaction` row exists for
    /// `txid`, when an in-block / chain-locked row is missing its
    /// `blockHash` (treated as miss rather than fabricating a zero
    /// hash that would round-trip back to Rust as a real block id),
    /// or when the row has no `transactionData` (the FFI write path
    /// always populates it, so a missing one signals a corrupt row
    /// the Rust side can't decode anyway).
    ///
    /// The wallet-id is currently unused (`txid` is globally
    /// unique), but is accepted to match the Rust trait signature
    /// and to leave room for a wallet-scoped variant.
    func coreTxRecord(
        walletId: Data,
        txid: Data
    ) -> (context: UInt32, blockHeight: UInt32, blockHash: Data, blockTimestamp: UInt32, transactionData: Data)? {
        _ = walletId
        return onQueue {
            let descriptor = FetchDescriptor<PersistentTransaction>(
                predicate: #Predicate { $0.txid == txid }
            )
            guard let row = try? backgroundContext.fetch(descriptor).first else {
                return nil
            }
            // The Rust side decodes `transactionData` into a
            // `dashcore::Transaction`; an empty buffer (left over
            // from an orphaned stub row in the UTXO upsert path
            // whose real upsert never arrived) won't decode, so
            // treat it as miss.
            guard !row.transactionData.isEmpty else {
                return nil
            }
            let transactionData = row.transactionData
            switch row.context {
            case 0, 1:
                // Mempool / InstantSend — block fields not meaningful;
                // the Rust side ignores them. Hand back zeroed
                // placeholders so the caller's tuple shape stays
                // uniform.
                return (
                    context: row.context,
                    blockHeight: 0,
                    blockHash: Data(count: 32),
                    blockTimestamp: 0,
                    transactionData: transactionData
                )
            default:
                // InBlock / InChainLockedBlock — `blockHash` MUST be
                // present and 32 bytes for the row to round-trip
                // correctly to Rust as a `BlockHash`.
                guard let blockHash = row.blockHash, blockHash.count == 32 else {
                    return nil
                }
                return (
                    context: row.context,
                    blockHeight: row.blockHeight,
                    blockHash: blockHash,
                    blockTimestamp: row.blockTimestamp,
                    transactionData: transactionData
                )
            }
        }
    }

    /// Look up the network for a wallet id by reading the owning
    /// `PersistentWallet` row. Returns `nil` if the wallet row
    /// doesn't exist or its network hasn't been resolved yet.
    private func walletNetwork(walletId: Data) -> Network? {
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        guard let wallet = try? backgroundContext.fetch(descriptor).first else {
            return nil
        }
        return wallet.network
    }
}

/// Retains all heap allocations produced by a single
/// `loadWalletList` call. Released wholesale by `loadWalletListFree`.
private final class LoadAllocation {
    var entries: UnsafeMutablePointer<WalletRestoreEntryFFI>?
    /// Allocated capacity — equal to `restorable.count`. Used for
    /// `deallocate()` (which only requires "the original allocation
    /// size") and as the upper bound on `entriesInitialized`.
    var entriesCount: Int = 0
    /// How many of the `entriesCount` slots have actually been
    /// written via `entriesPtr[i] = entry`. Tracked separately from
    /// `entriesCount` because early-abort paths (account-tag
    /// overflow, UTXO marshalling failure) call `release()` after
    /// only `0..<i` slots have been initialized; calling
    /// `deinitialize(count: entriesCount)` over the full capacity
    /// would deinitialize uninitialized memory, which is UB by
    /// `UnsafeMutablePointer`'s contract. The fact that
    /// `WalletRestoreEntryFFI` and its siblings happen to import as
    /// trivial C structs means the no-op deinit doesn't crash today,
    /// but any future field that imports as a non-trivial Swift
    /// type would turn this into real UB.
    var entriesInitialized: Int = 0
    /// `AccountSpecFFI` arrays per wallet.
    var accountArrays: [(UnsafeMutablePointer<AccountSpecFFI>, Int)] = []
    /// `AddressBalanceEntryFFI` arrays per wallet.
    var addressBalanceArrays: [(UnsafeMutablePointer<AddressBalanceEntryFFI>, Int)] = []
    /// `IdentityRestoreEntryFFI` arrays per wallet.
    var identityArrays: [(UnsafeMutablePointer<IdentityRestoreEntryFFI>, Int)] = []
    /// Per-identity `IdentityKeyRestoreFFI` arrays. One entry per
    /// identity that has at least one persisted public key. The byte
    /// buffers each row's `data` pointer references live in
    /// `scalarBuffers` (same `UnsafeMutablePointer<UInt8>.allocate`
    /// shape as xpub bytes).
    var identityKeyArrays: [(UnsafeMutablePointer<IdentityKeyRestoreFFI>, Int)] = []
    /// Byte buffers backing `root_xpub_bytes` and `account_xpub_bytes`.
    var scalarBuffers: [(UnsafeMutablePointer<UInt8>, Int)] = []
    /// NUL-terminated c-string buffers carried by identity entries
    /// (`label`, dpns name labels, etc.). Allocated via plain
    /// `UnsafeMutablePointer<CChar>.allocate`, freed by `deallocate()`.
    var cStringBuffers: [(UnsafeMutablePointer<CChar>, Int)] = []
    /// `*const c_char` arrays referenced by `dpns_names` /
    /// `contested_dpns_names`. Each inner pointer points into
    /// `cStringBuffers`; releasing this array doesn't touch the
    /// underlying strings.
    var cStringPointerArrays: [(UnsafeMutablePointer<UnsafePointer<CChar>?>, Int)] = []
    /// Per-wallet `UtxoRestoreEntryFFI` arrays. The script bytes each
    /// row references live in `scalarBuffers`.
    var utxoArrays: [(UnsafeMutablePointer<UtxoRestoreEntryFFI>, Int)] = []
    /// Per-wallet `AssetLockEntryFFI` arrays. The transaction-bytes
    /// and proof-bytes buffers each row references live in
    /// `scalarBuffers`.
    var assetLockArrays: [(UnsafeMutablePointer<AssetLockEntryFFI>, Int)] = []
    /// Per-wallet `UnresolvedAssetLockTxRecordFFI` arrays — the funding
    /// tx records for asset locks at `statusRaw < 2` that the Rust
    /// load path re-inserts into the in-memory `transactions()` map
    /// so the next chain-lock event can cascade-promote them. The
    /// `tx_bytes` buffer each row references lives in `scalarBuffers`.
    var unresolvedAssetLockTxRecordArrays: [(UnsafeMutablePointer<UnresolvedAssetLockTxRecordFFI>, Int)] = []
    /// Per-wallet `AccountAddressPoolFFI` arrays, the persisted core
    /// address pools
    var coreAddressPoolArrays: [(UnsafeMutablePointer<AccountAddressPoolFFI>, Int)] = []
    /// Inner `CoreAddressEntryFFI` arrays, one per pool entry above.
    var coreAddressEntryArrays: [(UnsafeMutablePointer<CoreAddressEntryFFI>, Int)] = []

    func release() {
        if let entries = entries {
            // Deinitialize ONLY the slots that were actually written
            // (`entriesInitialized`), then deallocate the full
            // capacity (`entriesCount`). Per Swift's pointer
            // contract, `deinitialize(count:)` requires the region
            // to be initialized; `deallocate()` only requires the
            // pointer to match the original allocation.
            if entriesInitialized > 0 {
                entries.deinitialize(count: entriesInitialized)
            }
            entries.deallocate()
        }
        for (ptr, count) in accountArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in addressBalanceArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in identityArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in identityKeyArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, _) in scalarBuffers {
            ptr.deallocate()
        }
        for (ptr, _) in cStringBuffers {
            ptr.deallocate()
        }
        for (ptr, _) in cStringPointerArrays {
            ptr.deallocate()
        }
        for (ptr, count) in utxoArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in assetLockArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in unresolvedAssetLockTxRecordArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in coreAddressEntryArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
        for (ptr, count) in coreAddressPoolArrays {
            ptr.deinitialize(count: count)
            ptr.deallocate()
        }
    }
}

/// Copy bytes from `src` into a fixed-size C-tuple field. Swift
/// imports `u8[N]` as an N-tuple — identical memory layout, so
/// `withUnsafeMutableBytes` gives us a contiguous write window of
/// exactly N bytes.
@inline(__always)
private func copyBytes<T>(_ src: Data, into dst: inout T) {
    withUnsafeMutableBytes(of: &dst) { raw in
        let bytes = raw.bindMemory(to: UInt8.self)
        let len = min(src.count, raw.count)
        src.copyBytes(to: bytes, count: len)
    }
}

// MARK: - C Callbacks

private func persistAddressBalancesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    entriesPtr: UnsafePointer<AddressBalanceEntryFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let entriesPtr = entriesPtr,
          count > 0 else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)

    var entries: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32)] = []
    entries.reserveCapacity(Int(count))

    for i in 0..<Int(count) {
        let entry = entriesPtr[i]
        let hashData = withUnsafeBytes(of: entry.address.hash) { Data($0) }
        entries.append((
            entry.address.address_type,
            hashData,
            entry.balance,
            entry.nonce,
            entry.account_index,
            entry.address_index
        ))
    }

    handler.persistAddressBalances(walletId: walletId, entries: entries)
    return 0
}

private func persistWalletChangesetCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    changesetPtr: UnsafePointer<WalletChangeSetFFI>?
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let changesetPtr = changesetPtr else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)
    handler.persistWalletChangeset(walletId: walletId, changeset: changesetPtr)
    return 0
}

/// C shim for `on_changeset_begin_fn`. Forwards to
/// `PlatformWalletPersistenceHandler.beginChangeset` so the handler
/// can prep any wallet-scope batching it needs for the round.
private func changesetBeginCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    handler.beginChangeset(walletId: walletId)
    return 0
}

/// C shim for `on_changeset_end_fn`. Forwards to
/// `PlatformWalletPersistenceHandler.endChangeset(walletId:success:)`,
/// which does the single `save()` (or `rollback()`) that commits all
/// per-kind writes accumulated during the round.
private func changesetEndCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    success: Bool
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    handler.endChangeset(walletId: walletId, success: success)
    return 0
}

private func persistSyncStateCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    syncHeight: UInt64,
    syncTimestamp: UInt64,
    lastKnownRecentBlock: UInt64
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)
    handler.persistSyncState(
        walletId: walletId,
        syncHeight: syncHeight,
        syncTimestamp: syncTimestamp,
        lastKnownRecentBlock: lastKnownRecentBlock
    )
    return 0
}

/// C shim for `on_persist_account_registrations_fn`. Walks the
/// Rust-owned `[AccountSpecFFI]` slice and writes one
/// `PersistentAccount` row per entry. Replaces the legacy
/// per-entry `on_persist_account_fn` — same shape per row, but
/// the round arrives as a single batched callback so the whole
/// registration round flushes through one `store(...)` cycle on
/// the Rust side.
private func persistAccountRegistrationsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    specsPtr: UnsafePointer<AccountSpecFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    if count > 0, let specsPtr = specsPtr {
        for i in 0..<Int(count) {
            handler.persistAccount(walletId: walletId, spec: specsPtr[i])
        }
    }
    return 0
}

private func loadWalletListCallback(
    context: UnsafeMutableRawPointer?,
    outEntries: UnsafeMutablePointer<UnsafePointer<WalletRestoreEntryFFI>?>?,
    outCount: UnsafeMutablePointer<UInt>?
) -> Int32 {
    guard let context = context,
          let outEntries = outEntries,
          let outCount = outCount else {
        return 1
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let (entries, count, errored) = handler.loadWalletList()
    outEntries.pointee = entries
    outCount.pointee = UInt(count)
    // Surface SwiftData fetch failures as a non-zero callback return so
    // the Rust loader aborts instead of silently degrading to an empty
    // restore (which previously masked database faults as
    // "successful 0-balance restore").
    return errored ? 1 : 0
}

private func loadWalletListFreeCallback(
    context: UnsafeMutableRawPointer?,
    entries: UnsafePointer<WalletRestoreEntryFFI>?,
    _ count: UInt
) {
    guard let context = context else { return }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    handler.loadWalletListFree(entries: entries.map(UnsafeRawPointer.init))
}

/// C shim for `on_persist_account_address_pools_fn`. Walks the
/// Rust-owned `[AccountAddressPoolFFI]` slice and dispatches one
/// `persistAccountAddresses` call per pool. Replaces the legacy
/// per-pool `on_persist_account_addresses_fn` — same row shape
/// but batched into a single round so the whole registration
/// flushes through one Rust-side `store(...)` cycle.
private func persistAccountAddressPoolsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    poolsPtr: UnsafePointer<AccountAddressPoolFFI>?,
    count: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    guard count > 0, let poolsPtr = poolsPtr else {
        return 0
    }

    for i in 0..<Int(count) {
        let pool = poolsPtr[i]
        let spec = pool.account
        var userIdentityId = Data(count: 32)
        withUnsafeBytes(of: spec.user_identity_id) { src in
            userIdentityId.withUnsafeMutableBytes { dst in dst.copyMemory(from: src) }
        }
        var friendIdentityId = Data(count: 32)
        withUnsafeBytes(of: spec.friend_identity_id) { src in
            friendIdentityId.withUnsafeMutableBytes { dst in dst.copyMemory(from: src) }
        }
        let key = PlatformWalletPersistenceHandler.AccountLookupKey(
            typeTag: UInt32(spec.type_tag),
            index: spec.index,
            standardTag: spec.standard_tag,
            registrationIndex: spec.registration_index,
            keyClass: spec.key_class,
            userIdentityId: userIdentityId,
            friendIdentityId: friendIdentityId
        )

        // Copy every C-string into a Swift String before leaving the
        // callback — Rust owns the underlying storage only for this window.
        var snapshots: [PlatformWalletPersistenceHandler.CoreAddressEntrySnapshot] = []
        snapshots.reserveCapacity(Int(pool.addresses_count))
        if pool.addresses_count > 0, let addressesPtr = pool.addresses_ptr {
            for j in 0..<Int(pool.addresses_count) {
                let entry = addressesPtr[j]
                let address = entry.address_base58.map { String(cString: $0) } ?? ""
                let derivationPath = entry.derivation_path.map { String(cString: $0) } ?? ""
                let publicKey: Data
                if entry.has_public_key {
                    var pk = Data(count: 33)
                    withUnsafeBytes(of: entry.public_key) { src in
                        pk.withUnsafeMutableBytes { dst in dst.copyMemory(from: src) }
                    }
                    publicKey = pk
                } else {
                    publicKey = Data()
                }
                if address.isEmpty { continue }
                snapshots.append(.init(
                    address: address,
                    publicKey: publicKey,
                    poolTypeTag: entry.pool_type_tag,
                    addressIndex: entry.address_index,
                    isUsed: entry.is_used,
                    balance: entry.balance,
                    derivationPath: derivationPath
                ))
            }
        }

        handler.persistAccountAddresses(walletId: walletId, accountKey: key, entries: snapshots)
    }

    return 0
}

/// C shim for `on_persist_identities_fn`. Copies every
/// `IdentityEntryFFI` into an owned `IdentityEntrySnapshot` before
/// invoking the handler so the Rust-side free-loop can release
/// heap allocations the moment this closure returns.
///
/// Typed pointers arrive as `UnsafeRawPointer?` because
/// `@convention(c)` can't carry non-`@objc`-bridgeable typed Swift
/// pointers — we cast to the real layout via `assumingMemoryBound`
/// here on the Swift side. The Rust `#[repr(C)]` definitions match
/// the Swift struct layout byte-for-byte so the cast is sound.
private func persistIdentitiesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<IdentityEntryFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<FFIByteTuple32>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.IdentityEntrySnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            let identityId = dataFromTuple32(e.identity_id)
            let walletIdField: Data? = e.wallet_id_is_some ? dataFromTuple32(e.wallet_id) : nil
            let identityIndexField: UInt32? = e.identity_index_is_some ? e.identity_index : nil

            // Walk the parallel DPNS arrays into Swift-owned
            // `(label, acquired_at)` tuples. Inner null pointers
            // (interior NUL labels — unreachable in practice) are
            // skipped without dropping the timestamp slot count, so
            // we keep iteration index-aligned. The Rust-side
            // `free_identity_entry_ffi` releases the C strings + the
            // outer arrays after this callback returns.
            var dpnsNames: [(label: String, acquiredAt: UInt64)] = []
            let dpnsCount = Int(e.dpns_names_count)
            if dpnsCount > 0,
               let labelsPtr = e.dpns_names,
               let acquiredPtr = e.dpns_names_acquired_at {
                dpnsNames.reserveCapacity(dpnsCount)
                for j in 0..<dpnsCount {
                    let labelPtr = labelsPtr[j]
                    let acquiredAt = acquiredPtr[j]
                    if let labelPtr = labelPtr {
                        let label = String(cString: labelPtr)
                        dpnsNames.append((label: label, acquiredAt: acquiredAt))
                    }
                }
            }

            // Walk the optional DashPay profile block. The
            // `dashpay_profile_present` bit is the single source of
            // truth: `false` means "no update for this flush"
            // (changeset-`None` semantics, NOT a delete signal), so
            // we carry `dashpayProfile == nil` through to the
            // handler. When the bit is set, every `*_present`
            // sub-flag is checked individually because zero-valued
            // payloads (empty strings, all-zero hashes /
            // fingerprints) are valid contract values and the FFI
            // would otherwise alias them to "absent".
            let dashpayProfile: PlatformWalletPersistenceHandler.DashpayProfileSnapshot?
            if e.dashpay_profile_present {
                let avatarHash: Data? = e.dashpay_profile_avatar_hash_present
                    ? hashData(e.dashpay_profile_avatar_hash)
                    : nil
                let avatarFingerprint: Data? = e.dashpay_profile_avatar_fingerprint_present
                    ? Swift.withUnsafeBytes(of: e.dashpay_profile_avatar_fingerprint) {
                        Data($0)
                    }
                    : nil
                dashpayProfile = PlatformWalletPersistenceHandler.DashpayProfileSnapshot(
                    displayName: e.dashpay_profile_display_name.map { String(cString: $0) },
                    bio: e.dashpay_profile_bio.map { String(cString: $0) },
                    publicMessage: e.dashpay_profile_public_message.map { String(cString: $0) },
                    avatarUrl: e.dashpay_profile_avatar_url.map { String(cString: $0) },
                    avatarHash: avatarHash,
                    avatarFingerprint: avatarFingerprint
                )
            } else {
                dashpayProfile = nil
            }

            upserts.append(.init(
                identityId: identityId,
                balance: e.balance,
                revision: e.revision,
                identityIndex: identityIndexField,
                // Label is no longer carried over the FFI — Swift
                // owns `PersistentIdentity.alias` directly.
                label: nil,
                status: e.status,
                walletId: walletIdField,
                dpnsNames: dpnsNames,
                dashpayProfile: dashpayProfile
            ))
        }
    }

    var removed: [Data] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removed.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            removed.append(dataFromTuple32(removedPtr[i]))
        }
    }

    handler.persistIdentities(
        walletId: walletId,
        upserts: upserts,
        removed: removed
    )
    return 0
}

/// C shim for `on_persist_identity_keys_fn`. Same snapshot + cast
/// pattern as the identities callback.
private func persistIdentityKeysCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<IdentityKeyEntryFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<IdentityKeyRemovalFFI>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            let identityId = dataFromTuple32(e.identity_id)
            let pubKey: Data
            if let ptr = e.public_key_data_ptr, e.public_key_data_len > 0 {
                pubKey = Data(bytes: ptr, count: Int(e.public_key_data_len))
            } else {
                pubKey = Data()
            }
            let walletId: Data? = e.wallet_id_is_some ? dataFromTuple32(e.wallet_id) : nil
            let indices: (identityIndex: UInt32, keyIndex: UInt32)? =
                e.derivation_indices_is_some
                    ? (e.identity_index, e.key_index)
                    : nil
            upserts.append(.init(
                identityId: identityId,
                keyId: e.key_id,
                purpose: e.purpose,
                securityLevel: e.security_level,
                keyType: e.key_type,
                readOnly: e.read_only,
                disabledAt: e.disabled_at_is_some ? e.disabled_at : nil,
                publicKeyData: pubKey,
                publicKeyHash: dataFromTuple20(e.public_key_hash),
                walletId: walletId,
                derivationIndices: indices
            ))
        }
    }

    var removed: [(identityId: Data, keyId: UInt32)] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removed.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            let r = removedPtr[i]
            removed.append((identityId: dataFromTuple32(r.identity_id), keyId: r.key_id))
        }
    }

    handler.persistIdentityKeys(walletId: walletId, upserts: upserts, removed: removed)
    return 0
}

/// C shim for `on_persist_token_balances_fn`. Same snapshot + cast
/// pattern as the identities callback — copies every
/// `TokenBalanceUpsertFFI` / `TokenBalanceRemovalFFI` into an owned
/// Swift snapshot before invoking the handler so the callback can
/// return immediately even if the receiver dispatches asynchronously.
private func persistTokenBalancesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<TokenBalanceUpsertFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<TokenBalanceRemovalFFI>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.TokenBalanceUpsertSnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            upserts.append(.init(
                identityId: dataFromTuple32(e.identity_id),
                tokenId: dataFromTuple32(e.token_id),
                balance: e.balance
            ))
        }
    }

    var removals: [PlatformWalletPersistenceHandler.TokenBalanceRemovalSnapshot] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removals.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            let r = removedPtr[i]
            removals.append(.init(
                identityId: dataFromTuple32(r.identity_id),
                tokenId: dataFromTuple32(r.token_id)
            ))
        }
    }

    handler.persistTokenBalances(walletId: walletId, upserts: upserts, removals: removals)
    return 0
}

/// C shim for `on_persist_asset_locks_fn`. Copies every
/// `AssetLockEntryFFI` row + every removed-outpoint tuple into
/// Swift-owned `Data` snapshots before invoking the handler so the
/// Rust-side `_storage` Vec can release the byte buffers as soon as
/// this trampoline returns.
private func persistAssetLocksCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<AssetLockEntryFFI>?,
    upsertsCount: UInt,
    removedPtr: UnsafePointer<FFIByteTuple36>?,
    removedCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.AssetLockEntrySnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            // Outpoint tuple → 36-byte raw → display-order hex string.
            let outPointRaw = Swift.withUnsafeBytes(of: e.out_point) { Data($0) }
            let outPointHex = PersistentAssetLock.encodeOutPoint(rawBytes: outPointRaw)
            // Consensus-encoded transaction bytes.
            let txBytes: Data
            if let ptr = e.transaction_bytes, e.transaction_bytes_len > 0 {
                txBytes = Data(bytes: ptr, count: Int(e.transaction_bytes_len))
            } else {
                txBytes = Data()
            }
            // Optional bincode-encoded proof.
            let proofBytes: Data?
            if let ptr = e.proof_bytes, e.proof_bytes_len > 0 {
                proofBytes = Data(bytes: ptr, count: Int(e.proof_bytes_len))
            } else {
                proofBytes = nil
            }
            upserts.append(.init(
                outPointHex: outPointHex,
                transactionBytes: txBytes,
                fundingTypeRaw: Int(e.funding_type),
                identityIndexRaw: Int32(bitPattern: e.identity_index),
                accountIndexRaw: Int32(bitPattern: e.account_index),
                amountDuffs: Int64(bitPattern: e.amount_duffs),
                statusRaw: Int(e.status),
                proofBytes: proofBytes
            ))
        }
    }

    var removed: [Data] = []
    if removedCount > 0, let removedPtr = removedPtr {
        removed.reserveCapacity(Int(removedCount))
        for i in 0..<Int(removedCount) {
            var tuple = removedPtr[i]
            let bytes = Swift.withUnsafeBytes(of: &tuple) { Data($0) }
            removed.append(bytes)
        }
    }

    handler.persistAssetLocks(walletId: walletId, upserts: upserts, removed: removed)
    return 0
}

/// C shim for `on_persist_contacts_fn`. Same snapshot + cast pattern
/// as the identities callback — copies every `ContactRequestFFI` /
/// `ContactRequestRemovalFFI` row into Swift-owned tuples before
/// invoking the handler so the matching `free_contact_requests_ffi`
/// pass on the Rust side runs cleanly the moment we return.
///
/// The `removed_sent` and `removed_incoming` arrays come in as two
/// parallel `*const ContactRequestRemovalFFI` slots; we keep them
/// separate through the snapshot too because the handler uses the
/// arrival bucket to decide which `is_outgoing` row to delete.
private func persistContactsCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsPtr: UnsafePointer<ContactRequestFFI>?,
    upsertsCount: UInt,
    removedSentPtr: UnsafePointer<ContactRequestRemovalFFI>?,
    removedSentCount: UInt,
    removedIncomingPtr: UnsafePointer<ContactRequestRemovalFFI>?,
    removedIncomingCount: UInt
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    var upserts: [PlatformWalletPersistenceHandler.ContactRequestSnapshot] = []
    if upsertsCount > 0, let upsertsPtr = upsertsPtr {
        upserts.reserveCapacity(Int(upsertsCount))
        for i in 0..<Int(upsertsCount) {
            let e = upsertsPtr[i]
            // `encrypted_public_key` is always populated on a
            // well-formed `ContactRequest` document — Rust's
            // `allocate_byte_buffer` only returns `(null, 0)` for an
            // empty slice, which is invalid for this field. Treat a
            // null pointer here defensively as an empty `Data` rather
            // than panicking; the unique constraint still upserts the
            // row, the row just won't decrypt.
            let encryptedPublicKey: Data
            if let pkPtr = e.encrypted_public_key, e.encrypted_public_key_len > 0 {
                encryptedPublicKey = Data(bytes: pkPtr, count: Int(e.encrypted_public_key_len))
            } else {
                encryptedPublicKey = Data()
            }
            let encryptedAccountLabel: Data?
            if let labelPtr = e.encrypted_account_label, e.encrypted_account_label_len > 0 {
                encryptedAccountLabel = Data(
                    bytes: labelPtr,
                    count: Int(e.encrypted_account_label_len)
                )
            } else {
                encryptedAccountLabel = nil
            }
            let autoAcceptProof: Data?
            if let proofPtr = e.auto_accept_proof, e.auto_accept_proof_len > 0 {
                autoAcceptProof = Data(bytes: proofPtr, count: Int(e.auto_accept_proof_len))
            } else {
                autoAcceptProof = nil
            }

            upserts.append(.init(
                ownerIdentityId: dataFromTuple32(e.owner_id),
                contactIdentityId: dataFromTuple32(e.contact_id),
                isOutgoing: e.is_outgoing,
                senderKeyIndex: e.sender_key_index,
                recipientKeyIndex: e.recipient_key_index,
                accountReference: e.account_reference,
                encryptedPublicKey: encryptedPublicKey,
                encryptedAccountLabel: encryptedAccountLabel,
                autoAcceptProof: autoAcceptProof,
                coreHeightCreatedAt: e.core_height_created_at,
                createdAtMillis: e.created_at
            ))
        }
    }

    var removedSent: [PlatformWalletPersistenceHandler.ContactRequestRemovalSnapshot] = []
    if removedSentCount > 0, let removedSentPtr = removedSentPtr {
        removedSent.reserveCapacity(Int(removedSentCount))
        for i in 0..<Int(removedSentCount) {
            let r = removedSentPtr[i]
            removedSent.append(.init(
                ownerIdentityId: dataFromTuple32(r.owner_id),
                contactIdentityId: dataFromTuple32(r.contact_id)
            ))
        }
    }

    var removedIncoming: [PlatformWalletPersistenceHandler.ContactRequestRemovalSnapshot] = []
    if removedIncomingCount > 0, let removedIncomingPtr = removedIncomingPtr {
        removedIncoming.reserveCapacity(Int(removedIncomingCount))
        for i in 0..<Int(removedIncomingCount) {
            let r = removedIncomingPtr[i]
            removedIncoming.append(.init(
                ownerIdentityId: dataFromTuple32(r.owner_id),
                contactIdentityId: dataFromTuple32(r.contact_id)
            ))
        }
    }

    handler.persistContacts(
        walletId: walletId,
        upserts: upserts,
        removedSent: removedSent,
        removedIncoming: removedIncoming
    )
    return 0
}

/// Copy a fixed 32-byte C tuple into an owned `Data`. Used by the
/// identity-persistence callbacks where Rust hands over `[u8; 32]`
/// fields as `(UInt8, UInt8, ...)` tuples.
@inline(__always)
private func dataFromTuple32(_ tuple: FFIByteTuple32) -> Data {
    var value = tuple
    return Swift.withUnsafeBytes(of: &value) { Data($0) }
}

/// Copy a fixed 20-byte C tuple into an owned `Data`. Identical
/// idiom to `dataFromTuple32`, just for RIPEMD160(SHA256) pubkey
/// hashes on identity-key entries.
@inline(__always)
private func dataFromTuple20(_ tuple: FFIByteTuple20) -> Data {
    var value = tuple
    return Swift.withUnsafeBytes(of: &value) { Data($0) }
}

private func persistWalletMetadataCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    network: FFINetwork,
    birthHeight: UInt32
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    handler.persistWalletMetadata(
        walletId: walletId,
        network: Network(ffiNetwork: network),
        birthHeight: birthHeight
    )
    return 0
}

/// C shim for `on_get_core_tx_record_fn`. Calls
/// `PlatformWalletPersistenceHandler.coreTxRecord(...)` and writes
/// the row's actual context kind, block info (when applicable), and
/// raw transaction bytes to the Rust-owned output pointers.
///
/// The transaction bytes are allocated here via
/// `UnsafeMutablePointer<UInt8>.allocate(capacity:)` and the
/// allocation is owned by the Rust side until it invokes
/// `getCoreTxRecordFreeCallback` below — Rust calls free exactly
/// once per hit.
///
/// Output contract:
/// - Sets `*outFound = true` and populates `outContextKind` (and
///   the three block fields when context is 2 or 3, plus the tx
///   bytes pointer + length) on a hit; returns `0`.
/// - Sets `*outFound = false` on a miss; returns `0`.
/// - Returns `0` even on Swift-side errors (treated as miss); the
///   Rust side's `record_or_persister` helper logs and falls
///   through to the caller's existing not-found / poll path.
private func getCoreTxRecordCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    txidPtr: UnsafePointer<UInt8>?,
    outContextKind: UnsafeMutablePointer<UInt8>?,
    outBlockHeight: UnsafeMutablePointer<UInt32>?,
    outBlockHash: UnsafeMutablePointer<UInt8>?,
    outBlockTimestamp: UnsafeMutablePointer<UInt32>?,
    outTxBytes: UnsafeMutablePointer<UnsafePointer<UInt8>?>?,
    outTxBytesLen: UnsafeMutablePointer<UInt>?,
    outFound: UnsafeMutablePointer<Bool>?
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let txidPtr = txidPtr,
          let outFound = outFound else {
        return 0
    }
    outFound.pointee = false
    outTxBytes?.pointee = nil
    outTxBytesLen?.pointee = 0

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    let txid = Data(bytes: txidPtr, count: 32)

    guard let row = handler.coreTxRecord(walletId: walletId, txid: txid) else {
        // Miss — outFound already set to false above.
        return 0
    }

    outContextKind?.pointee = UInt8(row.context)
    outBlockHeight?.pointee = row.blockHeight
    outBlockTimestamp?.pointee = row.blockTimestamp
    if let outBlockHash = outBlockHash {
        // `coreTxRecord` returns a 32-byte `blockHash` (real for
        // in-block / chain-locked rows, zeroed placeholder for
        // mempool / IS rows that the Rust side will ignore), so
        // this copy is bounded.
        row.blockHash.copyBytes(
            to: UnsafeMutableBufferPointer(start: outBlockHash, count: 32),
            count: 32
        )
    }

    // Hand the tx bytes to Rust. The buffer outlives this callback
    // — Rust calls `getCoreTxRecordFreeCallback` to release it.
    let len = row.transactionData.count
    let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: len)
    row.transactionData.copyBytes(
        to: UnsafeMutableBufferPointer(start: buffer, count: len),
        count: len
    )
    outTxBytes?.pointee = UnsafePointer(buffer)
    outTxBytesLen?.pointee = UInt(len)

    outFound.pointee = true
    return 0
}

/// Paired free callback for `on_get_core_tx_record_free_fn`.
/// Releases the buffer `getCoreTxRecordCallback` allocated above.
/// `UInt8` is trivial so no `deinitialize(count:)` is required —
/// `deallocate()` alone matches the `allocate(capacity:)`.
private func getCoreTxRecordFreeCallback(
    context: UnsafeMutableRawPointer?,
    txBytes: UnsafePointer<UInt8>?,
    _ txBytesLen: UInt
) {
    guard let txBytes = txBytes else { return }
    UnsafeMutablePointer(mutating: txBytes).deallocate()
    _ = context
    _ = txBytesLen
}
