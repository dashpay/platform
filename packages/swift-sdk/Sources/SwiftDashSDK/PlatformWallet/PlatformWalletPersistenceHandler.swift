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
        migrateLegacySyncStateRecordsToNetworkScope()
    }

    // MARK: - Address Balances

    /// Upsert address balances into SwiftData.
    ///
    /// For every incoming BLAST entry we also mirror the balance onto
    /// the matching `PersistentCoreAddress` row — otherwise the Storage
    /// Explorer / address detail views keep showing `Balance: 0` even
    /// after funds land, because that detail view reads the
    /// HD-derived address record (keyed by `account + addressIndex`)
    /// rather than the BLAST balance cache (keyed by `addressHash`).
    /// DIP-17 PlatformPayment addresses are the only pool fed by this
    /// path today (`accountType == 14`).
    func persistAddressBalances(
        walletId: Data,
        entries: [(UInt8, Data, UInt64, UInt32, UInt32, UInt32)]
    ) {
        for (addressType, addressHash, balance, nonce, accountIndex, addressIndex) in entries {
            let descriptor = FetchDescriptor<PersistentAddressBalance>(
                predicate: #Predicate { $0.addressHash == addressHash }
            )

            if let existing = try? backgroundContext.fetch(descriptor).first {
                existing.accountIndex = accountIndex
                existing.addressIndex = addressIndex
                existing.update(balance: balance, nonce: nonce)
            } else {
                let record = PersistentAddressBalance(
                    addressType: addressType,
                    addressHash: addressHash,
                    balance: balance,
                    nonce: nonce,
                    accountIndex: accountIndex,
                    addressIndex: addressIndex,
                    walletId: walletId
                )
                backgroundContext.insert(record)
            }

            mirrorBalanceToCoreAddress(
                walletId: walletId,
                accountIndex: accountIndex,
                addressIndex: addressIndex,
                balance: balance,
                nonce: nonce
            )
        }

        try? backgroundContext.save()
    }

    /// Find the matching `PersistentCoreAddress` for a DIP-17
    /// PlatformPayment entry (wallet + account 14/N + derivation
    /// index) and mirror the balance / used flag onto it.
    /// No-op if the address record hasn't been emitted yet.
    private func mirrorBalanceToCoreAddress(
        walletId: Data,
        accountIndex: UInt32,
        addressIndex: UInt32,
        balance: UInt64,
        nonce: UInt32
    ) {
        let platformPaymentType: UInt32 = 14
        let descriptor = FetchDescriptor<PersistentCoreAddress>(
            predicate: #Predicate { addr in
                addr.addressIndex == addressIndex
                    && addr.account?.accountType == platformPaymentType
                    && addr.account?.accountIndex == accountIndex
                    && addr.account?.wallet?.walletId == walletId
            }
        )
        guard let row = try? backgroundContext.fetch(descriptor).first else {
            return
        }
        let wasUsed = row.isUsed
        row.balance = balance
        if balance > 0 || nonce > 0 {
            row.isUsed = true
        }
        if row.balance != balance || row.isUsed != wasUsed {
            row.lastUpdated = Date()
        }
    }

    /// Load all cached address balances for a wallet.
    public func loadCachedBalances(walletId: Data) -> [(UInt8, [UInt8], UInt64, UInt32, UInt32, UInt32)] {
        let descriptor = FetchDescriptor<PersistentAddressBalance>(
            predicate: PersistentAddressBalance.predicate(walletId: walletId)
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
        try? backgroundContext.save()
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
        // `on_persist_wallet_root_xpub_fn` intentionally unassigned.
        // Root xpub is redundant with `wallet_id` for identity /
        // verification; Rust-side will stop requiring it once the
        // upstream rust-dashcore PR lands.
        cb.on_persist_account_fn = persistAccountCallback
        cb.on_load_wallet_list_fn = loadWalletListCallback
        cb.on_load_wallet_list_free_fn = loadWalletListFreeCallback
        cb.on_persist_wallet_metadata_fn = persistWalletMetadataCallback
        cb.on_persist_account_addresses_fn = persistAccountAddressesCallback
        return cb
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
