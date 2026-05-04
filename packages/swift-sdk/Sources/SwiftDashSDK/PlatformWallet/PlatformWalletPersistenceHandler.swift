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

    public init(modelContainer: ModelContainer) {
        self.modelContainer = modelContainer
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
    private func onQueue<T>(_ body: () -> T) -> T {
        serialQueue.sync(execute: body)
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
            let cs = changeset.pointee

            // Ensure PersistentWallet exists (lightweight upsert).
            let wallet = ensureWalletRecord(walletId: walletId)

            // Chain update.
            if cs.has_chain {
                if cs.chain.has_synced_height {
                    wallet.syncedHeight = cs.chain.synced_height
                }
                wallet.lastUpdated = Date()
            }

            // Balance delta — apply signed changes to cached totals.
            if cs.has_balance {
                let b = cs.balance
                wallet.balanceConfirmed = addDelta(wallet.balanceConfirmed, b.confirmed_delta)
                wallet.balanceUnconfirmed = addDelta(wallet.balanceUnconfirmed, b.unconfirmed_delta)
                wallet.balanceImmature = addDelta(wallet.balanceImmature, b.immature_delta)
                wallet.balanceLocked = addDelta(wallet.balanceLocked, b.locked_delta)
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

    /// Find or create the `PersistentWallet` record for this wallet id.
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
        // `account` is intentionally consumed only by the TXO upsert
        // pass that follows this method's call site. The transaction
        // row itself is account-agnostic — a single tx can land in
        // multiple accounts (or wallets), and per-wallet membership
        // is recovered through the TXO graph (`outputs` / `inputs`)
        // rather than a denormalized column on the transaction.
        _ = account
        let txidData = hashData(tx.txid)
        let descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == txidData }
        )

        let record: PersistentTransaction
        if let existing = try? backgroundContext.fetch(descriptor).first {
            record = existing
        } else {
            record = PersistentTransaction(
                txid: txidData,
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
        record.netAmount = tx.net_amount
        record.fee = tx.has_fee ? tx.fee : nil
        if let labelPtr = tx.label {
            record.label = String(cString: labelPtr)
        }
        record.firstSeen = tx.first_seen
        if let dataPtr = tx.tx_data, tx.tx_data_len > 0 {
            record.transactionData = Data(bytes: dataPtr, count: Int(tx.tx_data_len))
        }
        record.lastUpdated = Date()
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
                parentTx = PersistentTransaction(txid: txidData)
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
    /// travel through a Rust-side callback.
    public func setWalletName(walletId: Data, name: String) {
        onQueue {
            let wallet = ensureWalletRecord(walletId: walletId)
            wallet.name = name
            wallet.lastUpdated = Date()
            try? backgroundContext.save()
        }
    }

    // MARK: - Watch-only Restore: Account xpub

    /// Upsert a `PersistentAccount` row with the full `AccountSpecFFI`
    /// payload. Key is `(walletId, type_tag, index, registration_index,
    /// key_class, user_identity_id, friend_identity_id)` — everything
    /// that uniquely identifies an account across variants.
    func persistAccount(walletId: Data, spec: AccountSpecFFI) {
        onQueue {
        let wallet = ensureWalletRecord(walletId: walletId)
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
        }  // onQueue
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
    func loadWalletList() -> (entries: UnsafePointer<WalletRestoreEntryFFI>?, count: Int) {
        onQueue {
        let walletDescriptor = FetchDescriptor<PersistentWallet>()
        guard let wallets = try? backgroundContext.fetch(walletDescriptor) else {
            return (nil, 0)
        }
        let restorable = wallets.filter { wallet in
            wallet.accounts.contains { ($0.accountExtendedPubKeyBytes?.isEmpty == false) }
        }
        if restorable.isEmpty {
            return (nil, 0)
        }

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
            if sortedAccounts.isEmpty {
                accountsBuffer = nil
            } else {
                let buf = UnsafeMutablePointer<AccountSpecFFI>.allocate(capacity: sortedAccounts.count)
                for (j, acc) in sortedAccounts.enumerated() {
                    // Filter above guarantees non-nil + non-empty.
                    let xpub = acc.accountExtendedPubKeyBytes ?? Data()
                    let xpubBuffer = UnsafeMutablePointer<UInt8>.allocate(capacity: xpub.count)
                    xpub.copyBytes(to: xpubBuffer, count: xpub.count)
                    allocation.scalarBuffers.append((xpubBuffer, xpub.count))

                    var spec = AccountSpecFFI()
                    spec.type_tag = UInt8(truncatingIfNeeded: acc.accountType)
                    spec.standard_tag = acc.standardTag
                    spec.index = acc.accountIndex
                    spec.registration_index = acc.registrationIndex
                    spec.key_class = acc.keyClass
                    copyBytes(acc.userIdentityId, into: &spec.user_identity_id)
                    copyBytes(acc.friendIdentityId, into: &spec.friend_identity_id)
                    spec.account_xpub_bytes = UnsafePointer(xpubBuffer)
                    spec.account_xpub_bytes_len = UInt(xpub.count)
                    buf[j] = spec
                }
                accountsBuffer = buf
                allocation.accountArrays.append((buf, sortedAccounts.count))
            }

            let cachedBalances = loadCachedBalancesOnQueue(walletId: w.walletId)
            let addressBalancesBuffer: UnsafeMutablePointer<AddressBalanceEntryFFI>?
            if cachedBalances.isEmpty {
                addressBalancesBuffer = nil
            } else {
                let buf = UnsafeMutablePointer<AddressBalanceEntryFFI>.allocate(
                    capacity: cachedBalances.count
                )
                for (j, cached) in cachedBalances.enumerated() {
                    let (addressType, hash, balance, nonce, accountIndex, addressIndex) = cached
                    guard hash.count == 20 else {
                        continue
                    }

                    var hashTuple:
                        (
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
                        ) = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
                    if hash.count == 20 {
                        withUnsafeMutableBytes(of: &hashTuple) { raw in
                            raw.copyBytes(from: hash)
                        }
                    }

                    buf[j] = AddressBalanceEntryFFI(
                        address: PlatformAddressFFI(address_type: addressType, hash: hashTuple),
                        balance: balance,
                        nonce: nonce,
                        account_index: accountIndex,
                        address_index: addressIndex
                    )
                }
                addressBalancesBuffer = buf
                allocation.addressBalanceArrays.append((buf, cachedBalances.count))
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
            entry.accounts_count = UInt(sortedAccounts.count)
            entry.platform_address_balances = addressBalancesBuffer.map { UnsafePointer($0) }
            entry.platform_address_balances_count = UInt(cachedBalances.count)
            entry.platform_sync_height = syncState?.syncHeight ?? 0
            entry.platform_sync_timestamp = syncState?.syncTimestamp ?? 0
            entry.platform_last_known_recent_block = syncState?.lastKnownRecentBlock ?? 0
            entry.identities = identitiesBuffer.map { UnsafePointer($0) }
            entry.identities_count = UInt(sortedIdentities.count)
            // Primary-identity selection + gap-limit scan watermark
            // were dropped from the FFI shape — both moved off the
            // Rust manager (UI owns selection now, scan resume is
            // derived from the highest already-registered slot).
            entriesPtr[i] = entry
        }

        let typed = UnsafePointer(entriesPtr)
        loadAllocations[UnsafeRawPointer(typed)] = allocation
        return (typed, restorable.count)
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
                    // `String(rawValue)` of the original `UInt8` — same
                    // shape as the `purposeEnum` / `securityLevelEnum` /
                    // `keyTypeEnum` accessors on the model. Decode
                    // back to `UInt8`; fall back to 0 (the safest DPP
                    // default for each enum) on parse failure so we
                    // don't drop the row entirely.
                    row.key_type = UInt8(pk.keyType) ?? 0
                    row.purpose = UInt8(pk.purpose) ?? 0
                    row.security_level = UInt8(pk.securityLevel) ?? 0
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
    var entriesCount: Int = 0
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

    func release() {
        if let entries = entries {
            entries.deinitialize(count: entriesCount)
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
    let (entries, count) = handler.loadWalletList()
    outEntries.pointee = entries
    outCount.pointee = UInt(count)
    return 0
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
