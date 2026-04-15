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

    /// Upsert address balances into SwiftData.
    ///
    /// Called from the Rust persistence callback with only the addresses
    /// whose balance changed in this sync round.
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
    ///
    /// Returns tuples of (addressType, addressHash bytes, balance) for
    /// populating the UI before the first network sync completes.
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

    /// Build `PersistenceCallbacks` that point to this handler.
    ///
    /// The returned struct must not outlive `self`.
    func makeCallbacks() -> PersistenceCallbacks {
        let contextPtr = Unmanaged.passUnretained(self).toOpaque()
        var cb = PersistenceCallbacks()
        cb.context = contextPtr
        cb.on_persist_address_balances_fn = persistAddressBalancesCallback
        return cb
    }
}

// MARK: - C Callback

/// Static C callback that the Rust persister invokes with incremental
/// address balance updates. Recovers the handler from the context pointer
/// and upserts into SwiftData.
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
