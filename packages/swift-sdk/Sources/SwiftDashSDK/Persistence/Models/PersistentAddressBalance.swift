import Foundation
import SwiftData

/// SwiftData model for persisting platform address balances.
///
/// Each record represents a single platform payment address and its
/// credit balance as reported by the last BLAST sync. Records are
/// upserted incrementally — only the addresses whose balance changed
/// are written on each sync round.
@Model
public final class PersistentAddressBalance {
    /// Address type (0 = P2PKH, 1 = P2SH).
    public var addressType: UInt8
    /// 20-byte address hash, stored as Data for SwiftData indexing.
    @Attribute(.unique) public var addressHash: Data
    /// Credit balance in duffs.
    public var balance: UInt64
    /// 32-byte wallet ID that owns this address.
    public var walletId: Data
    /// Last time this record was updated.
    public var lastUpdated: Date

    public init(
        addressType: UInt8,
        addressHash: Data,
        balance: UInt64,
        walletId: Data
    ) {
        self.addressType = addressType
        self.addressHash = addressHash
        self.balance = balance
        self.walletId = walletId
        self.lastUpdated = Date()
    }

    public func updateBalance(_ newBalance: UInt64) {
        self.balance = newBalance
        self.lastUpdated = Date()
    }
}

// MARK: - Queries

extension PersistentAddressBalance {
    public static func predicate(walletId: Data) -> Predicate<PersistentAddressBalance> {
        #Predicate<PersistentAddressBalance> { entry in
            entry.walletId == walletId
        }
    }

    public static var nonZeroBalancesPredicate: Predicate<PersistentAddressBalance> {
        #Predicate<PersistentAddressBalance> { entry in
            entry.balance > 0
        }
    }
}
