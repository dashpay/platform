import Foundation
import DashSDKFFI

// Read-only diagnostic surface mirroring the `*_blocking` snapshot
// accessors on the Rust-side `PlatformWalletManager`. Every type and
// method here is a flat 1:1 bridge — marshal in, call FFI, marshal
// out, free, return. The decision logic lives upstream in
// `platform_wallet::manager::accessors`.

extension PlatformWalletManager {

    // MARK: - Phase 2 — Manager-level snapshots

    /// Atomic snapshot of every wallet id currently registered on the
    /// Rust manager. Avoids the Swift-side `wallets` cache so callers
    /// debugging cache drift can compare the two.
    public func listWalletIdsAtomic() -> [Data] {
        guard isConfigured, handle != NULL_HANDLE else { return [] }

        var outBytes: UnsafePointer<UInt8>? = nil
        var outCount: UInt = 0
        let res = platform_wallet_manager_list_wallet_ids(handle, &outBytes, &outCount)
        guard PlatformWalletResult(res).isSuccess, let ptr = outBytes, outCount > 0 else {
            return []
        }
        defer { platform_wallet_manager_free_wallet_ids(UnsafeMutablePointer(mutating: ptr), outCount) }
        return walletIdsFromFlatBuffer(ptr: ptr, count: Int(outCount))
    }

    public struct PlatformAddressSyncConfigSnapshot {
        public let intervalSeconds: UInt64
        public let watchListSize: Int
        public let lastEventUnixSeconds: UInt64
    }

    public func platformAddressSyncConfigSnapshot() -> PlatformAddressSyncConfigSnapshot? {
        guard isConfigured, handle != NULL_HANDLE else { return nil }
        var out = PlatformAddressSyncConfigFFI(
            interval_seconds: 0,
            watch_list_size: 0,
            last_event_unix_seconds: 0
        )
        let res = platform_wallet_manager_platform_address_sync_config(handle, &out)
        guard PlatformWalletResult(res).isSuccess else { return nil }
        return PlatformAddressSyncConfigSnapshot(
            intervalSeconds: out.interval_seconds,
            watchListSize: Int(out.watch_list_size),
            lastEventUnixSeconds: out.last_event_unix_seconds
        )
    }

    public struct IdentitySyncConfigSnapshot {
        public let intervalSeconds: UInt64
        public let queueDepth: Int
    }

    public func identitySyncConfigSnapshot() -> IdentitySyncConfigSnapshot? {
        guard isConfigured, handle != NULL_HANDLE else { return nil }
        var out = IdentitySyncConfigFFI(interval_seconds: 0, queue_depth: 0)
        let res = platform_wallet_manager_identity_sync_config(handle, &out)
        guard PlatformWalletResult(res).isSuccess else { return nil }
        return IdentitySyncConfigSnapshot(
            intervalSeconds: out.interval_seconds,
            queueDepth: Int(out.queue_depth)
        )
    }

    // MARK: - Phase 3 — Per-wallet state

    public struct CoreWalletStateSnapshot {
        public let syncedHeight: UInt32
        public let lastProcessedHeight: UInt32
        public let monitorRevision: UInt64
    }

    public func coreWalletState(for walletId: Data) -> CoreWalletStateSnapshot? {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return nil }
        var out = CoreWalletStateFFI(
            synced_height: 0,
            last_processed_height: 0,
            monitor_revision: 0
        )
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_core_wallet_state(handle, raw.baseAddress?.assumingMemoryBound(to: UInt8.self), &out)
        }
        guard PlatformWalletResult(res).isSuccess else { return nil }
        return CoreWalletStateSnapshot(
            syncedHeight: out.synced_height,
            lastProcessedHeight: out.last_processed_height,
            monitorRevision: out.monitor_revision
        )
    }

    public struct IdentityWalletStateSnapshot {
        public let lastScannedIndex: UInt32
        public let scanPending: Bool
    }

    public func identityWalletState(for walletId: Data) -> IdentityWalletStateSnapshot? {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return nil }
        var out = IdentityWalletStateFFI(last_scanned_index: 0, scan_pending: false)
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_identity_wallet_state(handle, raw.baseAddress?.assumingMemoryBound(to: UInt8.self), &out)
        }
        guard PlatformWalletResult(res).isSuccess else { return nil }
        return IdentityWalletStateSnapshot(
            lastScannedIndex: out.last_scanned_index,
            scanPending: out.scan_pending
        )
    }

    public struct PlatformAddressProviderStateSnapshot {
        public let initialized: Bool
        public let accountsWatched: Int
        public let foundCount: Int
        public let knownBalancesCount: Int
        public let watermarkHeight: UInt32
    }

    public func platformAddressProviderState(for walletId: Data) -> PlatformAddressProviderStateSnapshot? {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return nil }
        var out = PlatformAddressProviderStateFFI(
            initialized: false,
            accounts_watched: 0,
            found_count: 0,
            known_balances_count: 0,
            watermark_height: 0
        )
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_platform_address_provider_state(handle, raw.baseAddress?.assumingMemoryBound(to: UInt8.self), &out)
        }
        guard PlatformWalletResult(res).isSuccess else { return nil }
        return PlatformAddressProviderStateSnapshot(
            initialized: out.initialized,
            accountsWatched: Int(out.accounts_watched),
            foundCount: Int(out.found_count),
            knownBalancesCount: Int(out.known_balances_count),
            watermarkHeight: out.watermark_height
        )
    }

    // MARK: - Phase 4 — Floating state
    //
    // The `WalletInfoMetadataSnapshot` accessor (name / description /
    // birth+synced+last-processed heights / total transactions / first
    // loaded at) was removed: every meaningful field either duplicates
    // `CoreWalletStateSnapshot` or has nothing populating it on this
    // path. The C ABI (`platform_wallet_info_metadata*`) and the FFI
    // struct were dropped in lockstep — re-add the surface only if a
    // future caller needs name/description specifically.

    public struct TrackedAssetLockSnapshot {
        public let outpointTxid: Data
        public let outpointVout: UInt32
        public let lockType: UInt8
        public let status: UInt8
        public let registrationIndex: UInt32
        public let instantLockPresent: Bool
        public let chainLockHeight: UInt32
    }

    public func trackedAssetLocks(for walletId: Data) -> [TrackedAssetLockSnapshot] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return [] }
        var outEntries: UnsafePointer<TrackedAssetLockEntryFFI>? = nil
        var outCount: UInt = 0
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_tracked_asset_locks_list(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &outEntries,
                &outCount
            )
        }
        guard PlatformWalletResult(res).isSuccess, let ptr = outEntries, outCount > 0 else { return [] }
        defer { platform_wallet_tracked_asset_locks_free(UnsafeMutablePointer(mutating: ptr), outCount) }
        return (0..<Int(outCount)).map { i in
            var entry = ptr[i]
            let txid = withUnsafeBytes(of: &entry.outpoint_txid) { Data($0) }
            return TrackedAssetLockSnapshot(
                outpointTxid: txid,
                outpointVout: entry.outpoint_vout,
                lockType: entry.lock_type,
                status: entry.status,
                registrationIndex: entry.registration_index,
                instantLockPresent: entry.instant_lock_present,
                chainLockHeight: entry.chain_lock_height
            )
        }
    }

    public func instantSendLockTxids(for walletId: Data) -> [Data] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return [] }
        var outBytes: UnsafePointer<UInt8>? = nil
        var outCount: UInt = 0
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_instant_send_locks(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &outBytes,
                &outCount
            )
        }
        guard PlatformWalletResult(res).isSuccess, let ptr = outBytes, outCount > 0 else { return [] }
        defer { platform_wallet_instant_send_locks_free(UnsafeMutablePointer(mutating: ptr), outCount) }
        return walletIdsFromFlatBuffer(ptr: ptr, count: Int(outCount))
    }

    // MARK: - Phase 5 — Per-account drill-down

    /// Per-account metadata snapshot.
    ///
    /// `isWatchOnly` and `customName` were dropped after upstream
    /// removed both fields from `ManagedCoreFundsAccount` /
    /// `ManagedCoreKeysAccount`. Watch-only is now a wallet-level
    /// property; account-level custom names no longer exist.
    public struct AccountMetadataSnapshot {
        public let totalTransactions: UInt64
        public let totalUtxos: UInt64
        public let monitorRevision: UInt64
    }

    public func accountMetadata(
        for walletId: Data,
        balance: AccountBalance
    ) -> AccountMetadataSnapshot? {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return nil }
        var spec = makeAccountSpec(from: balance)
        var out = AccountMetadataFFI(
            total_transactions: 0,
            total_utxos: 0,
            monitor_revision: 0
        )
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_account_metadata(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &spec,
                &out
            )
        }
        guard PlatformWalletResult(res).isSuccess else { return nil }
        // Free fn is a no-op now (no heap fields), but call it so the
        // surface stays consistent if upstream re-introduces owned data.
        defer { platform_wallet_account_metadata_free(&out) }
        return AccountMetadataSnapshot(
            totalTransactions: out.total_transactions,
            totalUtxos: out.total_utxos,
            monitorRevision: out.monitor_revision
        )
    }

    public struct AccountAddressInfo {
        public let pubkeyHash: Data
        public let addressIndex: UInt32
        public let isUsed: Bool
        public let lastUsedHeight: UInt32
        /// Encoded address string (Base58check P2PKH for every account
        /// variant the explorer surfaces today).
        public let address: String
        /// Raw bytes of the derived public key. Empty when the pool
        /// entry didn't retain the derivation source — the FFI returns
        /// `null` + `len == 0` in that case and we surface it as an
        /// empty `Data`.
        public let publicKeyBytes: Data
    }

    public struct AccountAddressPool {
        public let poolType: UInt8
        public let gapLimit: UInt32
        public let lastUsedIndex: Int64
        public let addresses: [AccountAddressInfo]
    }

    public func accountAddressPools(
        for walletId: Data,
        balance: AccountBalance
    ) -> [AccountAddressPool] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return [] }
        var spec = makeAccountSpec(from: balance)
        var outPools: UnsafePointer<AccountAddressPoolEntryFFI>? = nil
        var outCount: UInt = 0
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_account_address_pools(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &spec,
                &outPools,
                &outCount
            )
        }
        guard PlatformWalletResult(res).isSuccess, let ptr = outPools, outCount > 0 else { return [] }
        defer { platform_wallet_account_address_pools_free(UnsafeMutablePointer(mutating: ptr), outCount) }
        return (0..<Int(outCount)).map { i in
            let entry = ptr[i]
            var addresses: [AccountAddressInfo] = []
            if let addrPtr = entry.addresses, entry.addresses_count > 0 {
                addresses.reserveCapacity(Int(entry.addresses_count))
                for j in 0..<Int(entry.addresses_count) {
                    var a = addrPtr[j]
                    let hash = withUnsafeBytes(of: &a.pubkey_hash) { Data($0) }
                    let address: String = {
                        guard let cstr = a.address else { return "" }
                        return String(cString: cstr)
                    }()
                    let publicKeyBytes: Data = {
                        guard let pkPtr = a.public_key_bytes,
                              a.public_key_bytes_len > 0
                        else { return Data() }
                        return Data(
                            bytes: pkPtr,
                            count: Int(a.public_key_bytes_len)
                        )
                    }()
                    addresses.append(AccountAddressInfo(
                        pubkeyHash: hash,
                        addressIndex: a.address_index,
                        isUsed: a.is_used,
                        lastUsedHeight: a.last_used_height,
                        address: address,
                        publicKeyBytes: publicKeyBytes
                    ))
                }
            }
            return AccountAddressPool(
                poolType: entry.pool_type,
                gapLimit: entry.gap_limit,
                lastUsedIndex: entry.last_used_index,
                addresses: addresses
            )
        }
    }

    public struct AccountUtxo {
        public let outpointTxid: Data
        public let outpointVout: UInt32
        public let valueDuffs: UInt64
        public let scriptPubkey: Data
        public let height: UInt32
        public let isLocked: Bool
    }

    public func accountUtxos(
        for walletId: Data,
        balance: AccountBalance
    ) -> [AccountUtxo] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return [] }
        var spec = makeAccountSpec(from: balance)
        var outUtxos: UnsafePointer<AccountUtxoEntryFFI>? = nil
        var outCount: UInt = 0
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_account_utxos(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &spec,
                &outUtxos,
                &outCount
            )
        }
        guard PlatformWalletResult(res).isSuccess, let ptr = outUtxos, outCount > 0 else { return [] }
        defer { platform_wallet_account_utxos_free(UnsafeMutablePointer(mutating: ptr), outCount) }
        return (0..<Int(outCount)).map { i in
            var entry = ptr[i]
            let txid = withUnsafeBytes(of: &entry.outpoint_txid) { Data($0) }
            let scriptData: Data
            if let sptr = entry.script_pubkey, entry.script_pubkey_len > 0 {
                scriptData = Data(bytes: sptr, count: Int(entry.script_pubkey_len))
            } else {
                scriptData = Data()
            }
            return AccountUtxo(
                outpointTxid: txid,
                outpointVout: entry.outpoint_vout,
                valueDuffs: entry.value_duffs,
                scriptPubkey: scriptData,
                height: entry.height,
                isLocked: entry.is_locked
            )
        }
    }

    // MARK: - Phase 6 — Per-account transactions

    public struct AccountTransaction {
        public let txid: Data
        public let height: UInt32
        public let timestamp: UInt64
        public let valueDeltaDuffs: Int64
        public let feeDuffs: UInt64
        public let isCoinbase: Bool
    }

    public func accountTransactions(
        for walletId: Data,
        balance: AccountBalance,
        pageOffset: Int = 0,
        pageLimit: Int = 0
    ) -> [AccountTransaction] {
        // `Int → UInt` traps on negative input; guard up front so a
        // misuse (e.g. negative offset) returns an empty result rather
        // than crashing. `pageLimit == 0` is reserved for "no limit"
        // by the Rust accessor, so 0 is a valid lower bound for both.
        guard isConfigured,
              handle != NULL_HANDLE,
              walletId.count == 32,
              pageOffset >= 0,
              pageLimit >= 0
        else { return [] }
        var spec = makeAccountSpec(from: balance)
        var outTxs: UnsafePointer<AccountTransactionEntryFFI>? = nil
        var outCount: UInt = 0
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_account_transactions(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &spec,
                UInt(pageOffset),
                UInt(pageLimit),
                &outTxs,
                &outCount
            )
        }
        guard PlatformWalletResult(res).isSuccess, let ptr = outTxs, outCount > 0 else { return [] }
        defer { platform_wallet_account_transactions_free(UnsafeMutablePointer(mutating: ptr), outCount) }
        return (0..<Int(outCount)).map { i in
            var entry = ptr[i]
            let txid = withUnsafeBytes(of: &entry.txid) { Data($0) }
            return AccountTransaction(
                txid: txid,
                height: entry.height,
                timestamp: entry.timestamp,
                valueDeltaDuffs: entry.value_delta_duffs,
                feeDuffs: entry.fee_duffs,
                isCoinbase: entry.is_coinbase
            )
        }
    }

    // MARK: - Phase 7 — Identity manager structure

    public func identityManagerOutOfWalletIds(for walletId: Data) -> [Data] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return [] }
        var outBytes: UnsafePointer<UInt8>? = nil
        var outCount: UInt = 0
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_identity_manager_out_of_wallet_ids(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &outBytes,
                &outCount
            )
        }
        guard PlatformWalletResult(res).isSuccess, let ptr = outBytes, outCount > 0 else { return [] }
        defer { platform_wallet_identity_manager_out_of_wallet_ids_free(UnsafeMutablePointer(mutating: ptr), outCount) }
        return walletIdsFromFlatBuffer(ptr: ptr, count: Int(outCount))
    }

    public struct WalletIdentityRow {
        public let registrationIndex: UInt32
        public let identityId: Data
    }

    public func identityManagerWalletIdentities(for walletId: Data) -> [WalletIdentityRow] {
        guard isConfigured, handle != NULL_HANDLE, walletId.count == 32 else { return [] }
        var outRows: UnsafePointer<WalletIdentityRowFFI>? = nil
        var outCount: UInt = 0
        let res = walletId.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            platform_wallet_identity_manager_wallet_identities(
                handle,
                raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                &outRows,
                &outCount
            )
        }
        guard PlatformWalletResult(res).isSuccess, let ptr = outRows, outCount > 0 else { return [] }
        defer { platform_wallet_identity_manager_wallet_identities_free(UnsafeMutablePointer(mutating: ptr), outCount) }
        return (0..<Int(outCount)).map { i in
            var entry = ptr[i]
            let id = withUnsafeBytes(of: &entry.identity_id) { Data($0) }
            return WalletIdentityRow(
                registrationIndex: entry.registration_index,
                identityId: id
            )
        }
    }

    // MARK: - Phase 8 — DAPI address ban list

    /// One row of the DAPI address ban-list snapshot.
    public struct AddressBanInfo {
        /// The DAPI node URI.
        public let address: String
        /// Whether the address is currently effectively banned.
        public let banned: Bool
        /// Total number of times the address has been banned.
        public let banCount: Int
        /// The instant until which the address remains banned, or `nil`
        /// when there is no active ban window.
        public let bannedUntil: Date?
        /// Human-readable reason for the most recent ban, or `nil` when
        /// none was recorded.
        public let reason: String?
    }

    /// Snapshot of every DAPI address' ban state, including the reason
    /// each address was banned (when recorded).
    public func addressBanInfo() -> [AddressBanInfo] {
        guard isConfigured, handle != NULL_HANDLE else { return [] }
        var outEntries: UnsafePointer<AddressBanInfoFFI>? = nil
        var outCount: UInt = 0
        let res = platform_wallet_manager_address_ban_info(handle, &outEntries, &outCount)
        guard PlatformWalletResult(res).isSuccess, let ptr = outEntries, outCount > 0 else { return [] }
        defer { platform_wallet_manager_address_ban_info_free(UnsafeMutablePointer(mutating: ptr), outCount) }
        return (0..<Int(outCount)).compactMap { i -> AddressBanInfo? in
            let entry = ptr[i]
            // The address is the key identifier; skip any (defensive) NULL
            // entry rather than surfacing a non-actionable blank row.
            guard let addressPtr = entry.address else { return nil }
            let address = String(cString: addressPtr)
            let reason: String? = entry.reason.map { String(cString: $0) }
            let bannedUntil: Date? = entry.banned_until_ms == 0
                ? nil
                : Date(timeIntervalSince1970: Double(entry.banned_until_ms) / 1000.0)
            return AddressBanInfo(
                address: address,
                banned: entry.banned,
                banCount: Int(entry.ban_count),
                bannedUntil: bannedUntil,
                reason: reason
            )
        }
    }
}

// MARK: - Helpers

private func walletIdsFromFlatBuffer(ptr: UnsafePointer<UInt8>, count: Int) -> [Data] {
    var result: [Data] = []
    result.reserveCapacity(count)
    for i in 0..<count {
        let slice = UnsafeBufferPointer(start: ptr.advanced(by: i * 32), count: 32)
        result.append(Data(buffer: slice))
    }
    return result
}

private func makeAccountSpec(from balance: PlatformWalletManager.AccountBalance) -> AccountSpecFFI {
    var spec = AccountSpecFFI()
    spec.type_tag = balance.typeTag
    spec.standard_tag = balance.standardTag
    spec.index = balance.index
    spec.registration_index = balance.registrationIndex
    spec.key_class = balance.keyClass
    withUnsafeMutableBytes(of: &spec.user_identity_id) { raw in
        let count = min(32, balance.userIdentityId.count)
        balance.userIdentityId.copyBytes(to: raw.bindMemory(to: UInt8.self), count: count)
    }
    withUnsafeMutableBytes(of: &spec.friend_identity_id) { raw in
        let count = min(32, balance.friendIdentityId.count)
        balance.friendIdentityId.copyBytes(to: raw.bindMemory(to: UInt8.self), count: count)
    }
    spec.account_xpub_bytes = nil
    spec.account_xpub_bytes_len = 0
    return spec
}
