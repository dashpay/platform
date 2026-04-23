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
    private let backgroundContext: ModelContext

    public init(modelContainer: ModelContainer) {
        self.modelContainer = modelContainer
        self.backgroundContext = ModelContext(modelContainer)
        self.backgroundContext.autosaveEnabled = true
        migrateLegacySyncStateRecordsToNetworkScope()
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

    /// Load all cached platform-address balances for a wallet. Tuple
    /// shape matches the Rust-side `AddressBalanceEntryFFI` layout so
    /// the load-wallet-list path can re-seed the provider on startup
    /// without a full rescan.
    public func loadCachedBalances(walletId: Data) -> [(UInt8, [UInt8], UInt64, UInt32, UInt32, UInt32)] {
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
        guard let network = syncStateNetwork(forWalletId: walletId) else {
            return
        }
        let scopeId = syncStateScopeId(for: network)
        let descriptor = FetchDescriptor<PersistentSyncState>(
            predicate: #Predicate { $0.walletId == scopeId }
        )

        if let existing = try? backgroundContext.fetch(descriptor).first {
            existing.network = network
            existing.syncHeight = syncHeight
            existing.syncTimestamp = syncTimestamp
            existing.lastKnownRecentBlock = lastKnownRecentBlock
            existing.lastUpdated = Date()
        } else {
            let record = PersistentSyncState(
                walletId: scopeId,
                network: network,
                syncHeight: syncHeight,
                syncTimestamp: syncTimestamp,
                lastKnownRecentBlock: lastKnownRecentBlock
            )
            backgroundContext.insert(record)
        }

        migrateLegacySyncStateRecordsToNetworkScope()
        // No save() — bracketed by changesetBegin/End from the
        // Rust store() round.
    }

    /// Load cached sync state for a wallet's network.
    public func loadCachedSyncState(walletId: Data) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        guard let network = syncStateNetwork(forWalletId: walletId) else {
            return nil
        }
        return loadCachedSyncState(network: network)
    }

    /// Load cached sync state for a specific network.
    public func loadCachedSyncState(network: String) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        let normalizedNetwork = normalizedNetworkName(network)
        let scopeId = syncStateScopeId(for: normalizedNetwork)
        let descriptor = FetchDescriptor<PersistentSyncState>(
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
            for i in 0..<cs.accounts_count {
                let acc = accountsPtr[i]
                applyAccountChangeset(walletRecord: wallet, acc: acc)
            }
        }

        // No save() — bracketed by changesetBegin/End.
    }

    /// Find or create the `PersistentWallet` record for this wallet id.
    private func ensureWalletRecord(walletId: Data) -> PersistentWallet {
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        if let existing = try? backgroundContext.fetch(descriptor).first {
            return existing
        }
        let record = PersistentWallet(walletId: walletId, network: "unknown")
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
        let typeName = acc.account_type_name.map { String(cString: $0) } ?? "Unknown"
        let accountIndex = acc.account_index

        // Upsert account (keyed by wallet + typeName + accountIndex).
        let walletId = walletRecord.walletId
        let accountDescriptor = FetchDescriptor<PersistentAccount>(
            predicate: #Predicate {
                $0.wallet?.walletId == walletId
                    && $0.accountTypeName == typeName
                    && $0.accountIndex == accountIndex
            }
        )
        let account: PersistentAccount
        if let existing = try? backgroundContext.fetch(accountDescriptor).first {
            account = existing
            account.lastUpdated = Date()
        } else {
            account = PersistentAccount(
                accountType: 0,
                accountIndex: accountIndex,
                accountTypeName: typeName
            )
            account.wallet = walletRecord
            backgroundContext.insert(account)
        }

        // Highest-used address pool indices.
        if acc.has_external_highest_used {
            account.externalHighestUsed = acc.external_highest_used
        }
        if acc.has_internal_highest_used {
            account.internalHighestUsed = acc.internal_highest_used
        }

        // Transactions.
        if acc.transactions_count > 0, let txsPtr = acc.transactions {
            for i in 0..<acc.transactions_count {
                upsertTransaction(account: account, tx: txsPtr[i])
            }
        }

        // UTXOs added.
        if acc.utxos_added_count > 0, let utxosPtr = acc.utxos_added {
            for i in 0..<acc.utxos_added_count {
                upsertUtxo(account: account, utxo: utxosPtr[i])
            }
        }

        // UTXOs spent — mark them spent (keep for history).
        if acc.utxos_spent_count > 0, let spentPtr = acc.utxos_spent {
            for i in 0..<acc.utxos_spent_count {
                markUtxoSpent(spentPtr[i])
            }
        }

        // UTXOs became InstantSend-locked — update flag.
        if acc.utxos_instant_locked_count > 0, let ilPtr = acc.utxos_instant_locked {
            for i in 0..<acc.utxos_instant_locked_count {
                markUtxoInstantLocked(ilPtr[i])
            }
        }
    }

    private func upsertTransaction(account: PersistentAccount, tx: TransactionRecordFFI) {
        let txidHex = hashHex(tx.txid)
        let descriptor = FetchDescriptor<PersistentTransaction>(
            predicate: #Predicate { $0.txid == txidHex }
        )
        // Pull the denormalized walletId from the account's parent
        // wallet relationship. Both hops are optional on the model,
        // but in regular (non-predicate) Swift code the chain is
        // cheap — and defaulting to `Data()` for orphaned rows is
        // harmless because such rows won't be matched by any
        // per-wallet query anyway.
        let resolvedWalletId: Data = account.wallet?.walletId ?? Data()

        let record: PersistentTransaction
        if let existing = try? backgroundContext.fetch(descriptor).first {
            record = existing
            // Backfill for rows created before the `walletId`
            // column existed (lightweight migration defaulted them
            // to empty Data).
            if record.walletId.isEmpty, !resolvedWalletId.isEmpty {
                record.walletId = resolvedWalletId
            }
        } else {
            record = PersistentTransaction(
                txid: txidHex,
                walletId: resolvedWalletId,
                context: tx.context,
                blockHeight: tx.block_height,
                direction: tx.direction,
                transactionType: tx.transaction_type.map { String(cString: $0) } ?? "Standard",
                netAmount: tx.net_amount,
                firstSeen: tx.first_seen
            )
            record.account = account
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
            record.transactionData = Data(bytes: dataPtr, count: tx.tx_data_len)
        }
        record.lastUpdated = Date()
    }

    private func upsertUtxo(account: PersistentAccount, utxo: UtxoEntryFFI) {
        let txidHex = hashHex(utxo.outpoint.txid)
        let outpoint = "\(txidHex):\(utxo.outpoint.vout)"
        let descriptor = FetchDescriptor<PersistentUtxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        let record: PersistentUtxo
        if let existing = try? backgroundContext.fetch(descriptor).first {
            record = existing
        } else {
            let script: Data = {
                guard let p = utxo.script_pubkey, utxo.script_pubkey_len > 0 else { return Data() }
                return Data(bytes: p, count: utxo.script_pubkey_len)
            }()
            let addressStr = utxo.address.map { String(cString: $0) } ?? ""
            record = PersistentUtxo(
                txid: txidHex,
                vout: utxo.outpoint.vout,
                amount: utxo.amount,
                address: addressStr,
                scriptPubKey: script,
                height: utxo.height
            )
            record.account = account
            backgroundContext.insert(record)
        }

        record.amount = utxo.amount
        record.height = utxo.height
        record.isCoinbase = utxo.is_coinbase
        record.isConfirmed = utxo.is_confirmed
        record.isInstantLocked = utxo.is_instantlocked
        record.isLocked = utxo.is_locked
        record.lastUpdated = Date()
    }

    private func markUtxoSpent(_ op: OutPointFFI) {
        let outpoint = "\(hashHex(op.txid)):\(op.vout)"
        let descriptor = FetchDescriptor<PersistentUtxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        if let utxo = try? backgroundContext.fetch(descriptor).first {
            utxo.isSpent = true
            utxo.lastUpdated = Date()
        }
    }

    private func markUtxoInstantLocked(_ op: OutPointFFI) {
        let outpoint = "\(hashHex(op.txid)):\(op.vout)"
        let descriptor = FetchDescriptor<PersistentUtxo>(
            predicate: #Predicate { $0.outpoint == outpoint }
        )
        if let utxo = try? backgroundContext.fetch(descriptor).first {
            utxo.isInstantLocked = true
            utxo.lastUpdated = Date()
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
        cb.on_persist_account_fn = persistAccountCallback
        cb.on_load_wallet_list_fn = loadWalletListCallback
        cb.on_load_wallet_list_free_fn = loadWalletListFreeCallback
        cb.on_persist_wallet_metadata_fn = persistWalletMetadataCallback
        cb.on_persist_account_addresses_fn = persistAccountAddressesCallback
        cb.on_persist_identities_fn = persistIdentitiesCallback
        cb.on_persist_identity_keys_fn = persistIdentityKeysCallback
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
        _ = walletId  // reserved for future wallet-scope batching
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
        _ = walletId
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

    // MARK: - Identity scalar persistence

    /// Upsert / remove rows from `PersistentIdentity` in response to
    /// an `IdentityChangeSet` forwarded by the Rust side.
    ///
    /// Mapping:
    /// - Each `upsert.identity_id` gets an upsert on
    ///   `PersistentIdentity` keyed by that unique column.
    /// - Each `removed` id drops the matching row.
    /// - `primaryIdentity` is a wallet-level hint that currently
    ///   maps onto `PersistentIdentity.alias == nil` semantics —
    ///   we don't store a dedicated "primary" column yet, so the
    ///   value is ignored on the Swift side for now. Wiring is here
    ///   so the signal doesn't get dropped when we add a column.
    /// - `lastScannedIndex` is wallet-level and not yet stored on
    ///   `PersistentWallet`; also passed through as a future-wire.
    ///
    /// Public keys are written by `persistIdentityKeys` on a paired
    /// callback; this path only touches the identity row itself.
    /// Both callbacks run under the same Rust-side wallet lock so
    /// the two-step apply is atomic from Swift's perspective.
    func persistIdentities(
        walletId: Data,
        upserts: [IdentityEntrySnapshot],
        removed: [Data],
        primaryIdentity: Data?,
        lastScannedIndex: UInt32?
    ) {
        for entry in upserts {
            let identityId = entry.identityId
            let descriptor = FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == identityId }
            )
            let row: PersistentIdentity
            if let existing = try? backgroundContext.fetch(descriptor).first {
                row = existing
            } else {
                row = PersistentIdentity(
                    identityId: entry.identityId,
                    balance: Int64(bitPattern: entry.balance),
                    revision: Int64(bitPattern: entry.revision),
                    isLocal: false
                )
                backgroundContext.insert(row)
            }
            // Scalars that ride every upsert — Rust guarantees
            // monotonic revision + paired balance/revision updates
            // by the merge gate in `IdentityChangeSet::merge`, so
            // overwriting unconditionally here is safe.
            row.balance = Int64(bitPattern: entry.balance)
            row.revision = Int64(bitPattern: entry.revision)
            row.identityIndex = entry.identityIndex
            if let label = entry.label {
                row.alias = label
            }
            row.lastUpdated = Date()

            // Attach the identity to its owning `PersistentWallet`
            // via the relationship. This is the sole wallet-side
            // association on the row — there is no denormalized
            // scalar — so downstream `@Query` views traverse
            // `identity.wallet?.walletId` when they need the raw
            // id. `deleteRule: .nullify` on the inverse nulls this
            // out cleanly if the wallet row is ever removed.
            // `fetchWalletForLink` returns nil when
            // `entry.walletId` is nil (detached identity) or when
            // no matching `PersistentWallet` exists yet — the
            // latter shouldn't happen in practice because
            // `persistWalletChangeset` upserts the wallet row in
            // the same atomic round before calling this path.
            row.wallet = fetchWalletForLink(walletId: entry.walletId)
        }

        for identityId in removed {
            let descriptor = FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == identityId }
            )
            if let existing = try? backgroundContext.fetch(descriptor).first {
                backgroundContext.delete(existing)
            }
        }

        // `primaryIdentity` + `lastScannedIndex` aren't yet mapped
        // onto PersistentIdentity / PersistentWallet columns. Keep
        // the call sites so the future column addition is one-line
        // row.primary = … work.
        _ = primaryIdentity
        _ = lastScannedIndex

        // No save() — bracketed by changesetBegin/End.
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
            // No bytes cross the FFI — when the entry carries both a
            // wallet_id and derivation indices, Swift re-derives the
            // 32-byte ECDSA scalar from the named wallet's mnemonic
            // and stores it in the keychain under the serialized
            // derivation path. Watch-only entries (missing either
            // breadcrumb) clear any prior stored identifier.
            if let walletIdForKey = entry.walletId,
                let indices = entry.derivationIndices {
                let keychainId = deriveAndStoreIdentityKey(
                    entry: entry,
                    walletId: walletIdForKey,
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

        _ = walletId  // reserved for future wallet-scope filters
        // No save() — bracketed by changesetBegin/End.
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
        let network: KeyWalletNetwork = keyWalletNetwork(fromName: persistentWallet.network)

        // 2. Fetch the mnemonic for this wallet from the keychain.
        //    WalletStorage stores it under `wallet.mnemonic.<hex>`
        //    in the unified `org.dashfoundation.wallet` service.
        let mnemonic: String
        do {
            mnemonic = try WalletStorage().retrieveMnemonic(for: walletId)
        } catch {
            print("⚠️ deriveAndStoreIdentityKey: mnemonic missing for wallet \(walletId.prefix(4).toHexString())…: \(error.localizedDescription)")
            return nil
        }

        // 3. Mnemonic → 64-byte BIP39 seed.
        let seed: Data
        do {
            seed = try Mnemonic.toSeed(mnemonic: mnemonic)
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
            publicKeyHash: publicKeyHashHex
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

    /// Map `PersistentWallet.network` string back onto
    /// `KeyWalletNetwork` for `KeyDerivation` calls. Unknown
    /// values fall back to testnet — every DIP-9 helper already
    /// handles Testnet / Devnet / Regtest via the same coin type.
    private func keyWalletNetwork(fromName name: String) -> KeyWalletNetwork {
        switch name.lowercased() {
        case "mainnet": return .mainnet
        case "devnet": return .devnet
        case "regtest": return .testnet
        default: return .testnet
        }
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
        let identityIndex: UInt32
        let label: String?
        let status: UInt8
        let walletId: Data?
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
        }

        try? backgroundContext.save()
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

        try? backgroundContext.save()
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
                $0.wallet?.walletId == walletId
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
    func persistWalletMetadata(walletId: Data, networkTag: UInt8, birthHeight: UInt32) {
        let wallet = ensureWalletRecord(walletId: walletId)
        wallet.network = networkName(for: networkTag)
        wallet.birthHeight = birthHeight
        wallet.lastUpdated = Date()
        try? backgroundContext.save()
        migrateLegacySyncStateRecordsToNetworkScope()
    }

    /// Set the user-facing name on the `PersistentWallet` row.
    /// Called from `PlatformWalletManager.createWallet` after the FFI
    /// returns a wallet id; only Swift knows the name, so it doesn't
    /// travel through a Rust-side callback.
    public func setWalletName(walletId: Data, name: String) {
        let wallet = ensureWalletRecord(walletId: walletId)
        wallet.name = name
        wallet.lastUpdated = Date()
        try? backgroundContext.save()
    }

    /// Reverse of the tag convention used by
    /// `platform_wallet_manager_create_wallet_from_seed`.
    private func networkName(for tag: UInt8) -> String {
        switch tag {
        case 0: return "mainnet"
        case 1: return "testnet"
        case 2: return "devnet"
        case 3: return "regtest"
        default: return "unknown"
        }
    }

    // MARK: - Watch-only Restore: Account xpub

    /// Upsert a `PersistentAccount` row with the full `AccountSpecFFI`
    /// payload. Key is `(walletId, type_tag, index, registration_index,
    /// key_class, user_identity_id, friend_identity_id)` — everything
    /// that uniquely identifies an account across variants.
    func persistAccount(walletId: Data, spec: AccountSpecFFI) {
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
            xpubBytes = Data(bytes: xpubPtr, count: spec.account_xpub_bytes_len)
        } else {
            xpubBytes = Data()
        }

        // Upsert keyed by the full account identity. We can't easily
        // express the identity tuple in a #Predicate with local `Data`
        // captures, so fetch by (walletId, accountType, accountIndex)
        // and verify the richer fields in Swift.
        let descriptor = FetchDescriptor<PersistentAccount>(
            predicate: #Predicate {
                $0.wallet?.walletId == walletId
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
                accountType: typeTag,
                accountIndex: index,
                accountTypeName: accountTypeName(
                    for: spec.type_tag,
                    standardTag: spec.standard_tag
                )
            )
            account.wallet = wallet
            backgroundContext.insert(account)
        }
        account.standardTag = spec.standard_tag
        account.registrationIndex = registrationIndex
        account.keyClass = keyClass
        account.userIdentityId = userIdentityId
        account.friendIdentityId = friendIdentityId
        account.accountExtendedPubKeyBytes = xpubBytes
        account.lastUpdated = Date()
        try? backgroundContext.save()
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
    func loadWalletList() -> (entries: UnsafeRawPointer?, count: Int) {
        let walletDescriptor = FetchDescriptor<PersistentWallet>()
        guard let wallets = try? backgroundContext.fetch(walletDescriptor) else {
            return (nil, 0)
        }
        let restorable = wallets.filter { wallet in
            wallet.accounts.contains { !$0.accountExtendedPubKeyBytes.isEmpty }
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
                .filter { !$0.accountExtendedPubKeyBytes.isEmpty }
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
                    let xpub = acc.accountExtendedPubKeyBytes
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
                    spec.account_xpub_bytes_len = xpub.count
                    buf[j] = spec
                }
                accountsBuffer = buf
                allocation.accountArrays.append((buf, sortedAccounts.count))
            }

            let cachedBalances = loadCachedBalances(walletId: w.walletId)
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

            let syncState = loadCachedSyncState(network: w.network)

            var entry = WalletRestoreEntryFFI()
            copyBytes(w.walletId, into: &entry.wallet_id)
            entry.network = networkTag(for: w.network)
            entry.accounts = accountsBuffer.map { UnsafePointer($0) }
            entry.accounts_count = sortedAccounts.count
            entry.platform_address_balances = addressBalancesBuffer.map { UnsafePointer($0) }
            entry.platform_address_balances_count = cachedBalances.count
            entry.platform_sync_height = syncState?.syncHeight ?? 0
            entry.platform_sync_timestamp = syncState?.syncTimestamp ?? 0
            entry.platform_last_known_recent_block = syncState?.lastKnownRecentBlock ?? 0
            entriesPtr[i] = entry
        }

        let opaque = UnsafeRawPointer(entriesPtr)
        loadAllocations[opaque] = allocation
        return (opaque, restorable.count)
    }

    /// Return the list of wallet ids that could be restored from
    /// SwiftData (i.e. have ≥1 account with a non-empty xpub). Used by
    /// `PlatformWalletManager.loadFromPersistor` after the FFI call
    /// succeeds so it can fetch a Swift-side handle for each wallet
    /// Rust just reconstructed.
    public func restorableWalletIds() -> [Data] {
        let descriptor = FetchDescriptor<PersistentWallet>()
        guard let wallets = try? backgroundContext.fetch(descriptor) else {
            return []
        }
        return wallets
            .filter { w in w.accounts.contains { !$0.accountExtendedPubKeyBytes.isEmpty } }
            .map { $0.walletId }
    }

    /// Release all allocations for a given load-callback result.
    func loadWalletListFree(entries: UnsafeRawPointer?) {
        guard let entries = entries, let allocation = loadAllocations.removeValue(forKey: entries) else {
            return
        }
        allocation.release()
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

    private func networkTag(for name: String) -> UInt8 {
        switch name.lowercased() {
        case "mainnet": return 0
        case "testnet": return 1
        case "devnet": return 2
        case "regtest": return 3
        default: return 1 // default to testnet for unknown during dev
        }
    }

    private func normalizedNetworkName(_ network: String) -> String {
        let trimmed = network.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !trimmed.isEmpty else {
            return "unknown"
        }
        return trimmed
    }

    private func syncStateScopeId(for network: String) -> Data {
        let scopeString = "platform-sync:\(normalizedNetworkName(network))"
        var data = Data(scopeString.utf8.prefix(32))
        if data.count < 32 {
            data.append(Data(repeating: 0, count: 32 - data.count))
        }
        return data
    }

    private func walletNetwork(walletId: Data) -> String? {
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        guard let wallet = try? backgroundContext.fetch(descriptor).first else {
            return nil
        }
        let normalized = normalizedNetworkName(wallet.network)
        return normalized == "unknown" && wallet.network.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            ? nil
            : normalized
    }

    private func syncStateNetwork(forWalletId walletId: Data) -> String? {
        if let network = walletNetwork(walletId: walletId) {
            return network
        }

        let descriptor = FetchDescriptor<PersistentSyncState>(
            predicate: #Predicate { $0.walletId == walletId }
        )
        guard let record = try? backgroundContext.fetch(descriptor).first else {
            return nil
        }
        return syncStateNetwork(for: record)
    }

    private func syncStateNetwork(for record: PersistentSyncState) -> String? {
        let normalized = normalizedNetworkName(record.network)
        if normalized != "unknown" || !record.network.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return normalized
        }

        return walletNetwork(walletId: record.walletId)
    }

    private func fetchSyncStateRecord(scopeId: Data) -> PersistentSyncState? {
        let descriptor = FetchDescriptor<PersistentSyncState>(
            predicate: #Predicate { $0.walletId == scopeId }
        )
        return try? backgroundContext.fetch(descriptor).first
    }

    private func mergeSyncState(_ source: PersistentSyncState, into target: PersistentSyncState) {
        target.syncHeight = max(target.syncHeight, source.syncHeight)
        target.syncTimestamp = max(target.syncTimestamp, source.syncTimestamp)
        target.lastKnownRecentBlock = max(
            target.lastKnownRecentBlock,
            source.lastKnownRecentBlock
        )
        if source.lastUpdated > target.lastUpdated {
            target.lastUpdated = source.lastUpdated
        }
        if let network = syncStateNetwork(for: source) {
            target.network = network
        }
    }

    private func migrateLegacySyncStateRecordsToNetworkScope() {
        let descriptor = FetchDescriptor<PersistentSyncState>()
        guard let records = try? backgroundContext.fetch(descriptor), !records.isEmpty else {
            return
        }

        var canonicalByNetwork: [String: PersistentSyncState] = [:]
        var mutated = false

        for record in records {
            guard let network = syncStateNetwork(for: record) else {
                continue
            }
            let scopeId = syncStateScopeId(for: network)

            if record.walletId == scopeId {
                if record.network != network {
                    record.network = network
                    mutated = true
                }

                if let canonical = canonicalByNetwork[network], canonical !== record {
                    mergeSyncState(record, into: canonical)
                    backgroundContext.delete(record)
                    mutated = true
                } else {
                    canonicalByNetwork[network] = record
                }
                continue
            }

            let canonical: PersistentSyncState
            if let existing = canonicalByNetwork[network] {
                canonical = existing
            } else if let existing = fetchSyncStateRecord(scopeId: scopeId) {
                canonical = existing
                if existing.network != network {
                    existing.network = network
                    mutated = true
                }
                canonicalByNetwork[network] = existing
            } else {
                canonical = PersistentSyncState(
                    walletId: scopeId,
                    network: network,
                    syncHeight: 0,
                    syncTimestamp: 0,
                    lastKnownRecentBlock: 0
                )
                backgroundContext.insert(canonical)
                canonicalByNetwork[network] = canonical
                mutated = true
            }

            mergeSyncState(record, into: canonical)
            backgroundContext.delete(record)
            mutated = true
        }

        if mutated {
            try? backgroundContext.save()
        }
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
    /// Byte buffers backing `root_xpub_bytes` and `account_xpub_bytes`.
    var scalarBuffers: [(UnsafeMutablePointer<UInt8>, Int)] = []

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
        for (ptr, _) in scalarBuffers {
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
    entriesRaw: UnsafeRawPointer?,
    count: Int
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let entriesRaw = entriesRaw,
          count > 0 else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)
    let entriesPtr = entriesRaw.assumingMemoryBound(to: AddressBalanceEntryFFI.self)

    var entries: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32)] = []
    entries.reserveCapacity(count)

    for i in 0..<count {
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
    changesetRaw: UnsafeRawPointer?
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let changesetRaw = changesetRaw else {
        return 0
    }

    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)
    let changesetPtr = changesetRaw.assumingMemoryBound(to: WalletChangeSetFFI.self)
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

private func persistAccountCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    specRaw: UnsafeRawPointer?
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let specRaw = specRaw else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)
    let spec = specRaw.assumingMemoryBound(to: AccountSpecFFI.self).pointee
    handler.persistAccount(walletId: walletId, spec: spec)
    return 0
}

private func loadWalletListCallback(
    context: UnsafeMutableRawPointer?,
    outEntries: UnsafeMutablePointer<UnsafeRawPointer?>?,
    outCount: UnsafeMutablePointer<Int>?
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
    outCount.pointee = count
    return 0
}

private func loadWalletListFreeCallback(
    context: UnsafeMutableRawPointer?,
    entries: UnsafeRawPointer?,
    _ count: Int
) {
    guard let context = context else { return }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    handler.loadWalletListFree(entries: entries)
}

private func persistAccountAddressesCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    specRaw: UnsafeRawPointer?,
    addressesRaw: UnsafeRawPointer?,
    count: Int
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr,
          let specRaw = specRaw,
          count >= 0 else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    let spec = specRaw.assumingMemoryBound(to: AccountSpecFFI.self).pointee
    let addressesPtr: UnsafePointer<CoreAddressEntryFFI>? =
        addressesRaw?.assumingMemoryBound(to: CoreAddressEntryFFI.self)
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
    snapshots.reserveCapacity(count)
    if count > 0, let addressesPtr = addressesPtr {
        for i in 0..<count {
            let entry = addressesPtr[i]
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
    upsertsRaw: UnsafeRawPointer?,
    upsertsCount: Int,
    removedRaw: UnsafeRawPointer?,
    removedCount: Int,
    primaryIdentityRaw: UnsafeRawPointer?,
    hasLastScannedIndex: Bool,
    lastScannedIndex: UInt32
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
    if upsertsCount > 0, let upsertsRaw = upsertsRaw {
        let upsertsPtr = upsertsRaw.assumingMemoryBound(to: IdentityEntryFFI.self)
        upserts.reserveCapacity(upsertsCount)
        for i in 0..<upsertsCount {
            let e = upsertsPtr[i]
            let identityId = dataFromTuple32(e.identity_id)
            let walletIdField: Data? = e.wallet_id_is_some ? dataFromTuple32(e.wallet_id) : nil
            let label: String? = e.label.map { String(cString: $0) }
            upserts.append(.init(
                identityId: identityId,
                balance: e.balance,
                revision: e.revision,
                identityIndex: e.identity_index,
                label: label,
                status: e.status,
                walletId: walletIdField
            ))
        }
    }

    var removed: [Data] = []
    if removedCount > 0, let removedRaw = removedRaw {
        let removedPtr = removedRaw.assumingMemoryBound(to: FFIByteTuple32.self)
        removed.reserveCapacity(removedCount)
        for i in 0..<removedCount {
            removed.append(dataFromTuple32(removedPtr[i]))
        }
    }

    let primary: Data? = primaryIdentityRaw.map { raw in
        let ptr = raw.assumingMemoryBound(to: FFIByteTuple32.self)
        return dataFromTuple32(ptr.pointee)
    }
    let scanIndex: UInt32? = hasLastScannedIndex ? lastScannedIndex : nil

    handler.persistIdentities(
        walletId: walletId,
        upserts: upserts,
        removed: removed,
        primaryIdentity: primary,
        lastScannedIndex: scanIndex
    )
    return 0
}

/// C shim for `on_persist_identity_keys_fn`. Same snapshot + cast
/// pattern as the identities callback.
private func persistIdentityKeysCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    upsertsRaw: UnsafeRawPointer?,
    upsertsCount: Int,
    removedRaw: UnsafeRawPointer?,
    removedCount: Int
) -> Int32 {
    guard let context = context,
          let walletIdPtr = walletIdPtr else {
        return 0
    }
    let handler = Unmanaged<PlatformWalletPersistenceHandler>
        .fromOpaque(context)
        .takeUnretainedValue()
    let walletId = Data(bytes: walletIdPtr, count: 32)

    // Fail fast with a clear message if the Rust / Swift struct
    // layouts have drifted — a subtle field reorder on either side
    // would otherwise crash in `memmove` deep inside
    // `Data(bytes:count:)` with garbage pointer bytes, and take
    // ages to diagnose.
    assertIdentityKeyEntryLayout()

    var upserts: [PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot] = []
    if upsertsCount > 0, let upsertsRaw = upsertsRaw {
        let upsertsPtr = upsertsRaw.assumingMemoryBound(to: IdentityKeyEntryFFI.self)
        upserts.reserveCapacity(upsertsCount)
        for i in 0..<upsertsCount {
            let e = upsertsPtr[i]
            let identityId = dataFromTuple32(e.identity_id)
            let pubKey: Data
            if let ptr = e.public_key_data_ptr, e.public_key_data_len > 0 {
                pubKey = Data(bytes: ptr, count: e.public_key_data_len)
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
    if removedCount > 0, let removedRaw = removedRaw {
        let removedPtr = removedRaw.assumingMemoryBound(to: IdentityKeyRemovalFFI.self)
        removed.reserveCapacity(removedCount)
        for i in 0..<removedCount {
            let r = removedPtr[i]
            removed.append((identityId: dataFromTuple32(r.identity_id), keyId: r.key_id))
        }
    }

    handler.persistIdentityKeys(walletId: walletId, upserts: upserts, removed: removed)
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
    networkTag: UInt8,
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
        networkTag: networkTag,
        birthHeight: birthHeight
    )
    return 0
}
