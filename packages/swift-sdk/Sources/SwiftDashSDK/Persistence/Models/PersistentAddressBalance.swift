import Foundation
import SwiftData

/// SwiftData model for persisting platform address balances.
///
/// Each record represents a single platform payment address and its
/// full funds snapshot as reported by the last BLAST sync. Records
/// are upserted incrementally — only the addresses whose balance or
/// nonce changed are written on each sync round.
@Model
public final class PersistentAddressBalance {
    /// Address type (0 = P2PKH, 1 = P2SH).
    public var addressType: UInt8
    /// 20-byte address hash, stored as Data for SwiftData indexing.
    @Attribute(.unique) public var addressHash: Data
    /// Credit balance in credits.
    public var balance: UInt64
    /// Address nonce used for anti-replay.
    public var nonce: UInt32 = 0
    /// DIP-17 account index.
    public var accountIndex: UInt32 = 0
    /// DIP-17 derivation index within the account.
    public var addressIndex: UInt32 = 0
    /// 32-byte wallet ID that owns this address.
    public var walletId: Data
    /// Last time this record was updated.
    public var lastUpdated: Date

    public init(
        addressType: UInt8,
        addressHash: Data,
        balance: UInt64,
        nonce: UInt32,
        accountIndex: UInt32,
        addressIndex: UInt32,
        walletId: Data
    ) {
        self.addressType = addressType
        self.addressHash = addressHash
        self.balance = balance
        self.nonce = nonce
        self.accountIndex = accountIndex
        self.addressIndex = addressIndex
        self.walletId = walletId
        self.lastUpdated = Date()
    }

    public func update(balance newBalance: UInt64, nonce newNonce: UInt32) {
        self.balance = newBalance
        self.nonce = newNonce
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
