import Foundation
import SwiftData

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
    }

    // MARK: - Address Balances

    /// Upsert address balances into SwiftData.
    func persistAddressBalances(walletId: Data, entries: [(UInt8, Data, UInt64)]) {
        for (addressType, addressHash, balance) in entries {
            let descriptor = FetchDescriptor<PersistentAddressBalance>(
                predicate: #Predicate { $0.addressHash == addressHash }
            )

            if let existing = try? backgroundContext.fetch(descriptor).first {
                existing.updateBalance(balance)
            } else {
                let record = PersistentAddressBalance(
                    addressType: addressType,
                    addressHash: addressHash,
                    balance: balance,
                    walletId: walletId
                )
                backgroundContext.insert(record)
            }
        }

        try? backgroundContext.save()
    }

    /// Load all cached address balances for a wallet.
    public func loadCachedBalances(walletId: Data) -> [(UInt8, [UInt8], UInt64)] {
        let descriptor = FetchDescriptor<PersistentAddressBalance>(
            predicate: PersistentAddressBalance.predicate(walletId: walletId)
        )

        guard let records = try? backgroundContext.fetch(descriptor) else {
            return []
        }

        return records.map { record in
            (record.addressType, Array(record.addressHash), record.balance)
        }
    }

    // MARK: - Sync State

    /// Upsert sync state into SwiftData.
    func persistSyncState(
        walletId: Data,
        syncHeight: UInt64,
        syncTimestamp: UInt64,
        lastKnownRecentBlock: UInt64
    ) {
        let descriptor = FetchDescriptor<PersistentSyncState>(
            predicate: #Predicate { $0.walletId == walletId }
        )

        if let existing = try? backgroundContext.fetch(descriptor).first {
            existing.syncHeight = syncHeight
            existing.syncTimestamp = syncTimestamp
            existing.lastKnownRecentBlock = lastKnownRecentBlock
            existing.lastUpdated = Date()
        } else {
            let record = PersistentSyncState(
                walletId: walletId,
                syncHeight: syncHeight,
                syncTimestamp: syncTimestamp,
                lastKnownRecentBlock: lastKnownRecentBlock
            )
            backgroundContext.insert(record)
        }

        try? backgroundContext.save()
    }

    /// Load cached sync state for a wallet.
    public func loadCachedSyncState(walletId: Data) -> (syncHeight: UInt64, syncTimestamp: UInt64, lastKnownRecentBlock: UInt64)? {
        let descriptor = FetchDescriptor<PersistentSyncState>(
            predicate: #Predicate { $0.walletId == walletId }
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

        try? backgroundContext.save()
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
        let record: PersistentTransaction
        if let existing = try? backgroundContext.fetch(descriptor).first {
            record = existing
        } else {
            record = PersistentTransaction(
                txid: txidHex,
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
        cb.on_persist_address_balances_fn = persistAddressBalancesCallback
        cb.on_persist_wallet_changeset_fn = persistWalletChangesetCallback
        cb.on_persist_sync_state_fn = persistSyncStateCallback
        return cb
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

    var entries: [(UInt8, Data, UInt64)] = []
    entries.reserveCapacity(count)

    for i in 0..<count {
        let entry = entriesPtr[i]
        let hashData = withUnsafeBytes(of: entry.address.hash) { Data($0) }
        entries.append((entry.address.address_type, hashData, entry.balance))
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
