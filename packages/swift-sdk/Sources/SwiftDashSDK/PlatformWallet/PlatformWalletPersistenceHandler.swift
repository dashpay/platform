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

    // MARK: - Callbacks

    /// Build `PersistenceCallbacks` that point to this handler.
    ///
    /// The returned struct must not outlive `self`.
    func makeCallbacks() -> PersistenceCallbacks {
        let contextPtr = Unmanaged.passUnretained(self).toOpaque()
        var cb = PersistenceCallbacks()
        cb.context = contextPtr
        cb.on_persist_address_balances_fn = persistAddressBalancesCallback
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
