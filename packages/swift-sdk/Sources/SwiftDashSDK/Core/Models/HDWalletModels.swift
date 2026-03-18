import Foundation
import SwiftData
import DashSDKFFI

// Note: The main wallet models are defined in:
// - HDWallet.swift (HDWallet, HDAccount, HDAddress)
// - HDTransaction.swift (HDTransaction, TransactionInput, TransactionOutput)
// - UTXO.swift (HDUTXO)

// MARK: - Account Category

public enum AccountCategory: Equatable, Hashable, Sendable {
    case bip44
    case bip32
    case coinjoin
    case identityRegistration
    case identityInvitation
    case identityTopupNotBound
    case identityTopup
    case providerVotingKeys
    case providerOwnerKeys
    case providerOperatorKeys
    case providerPlatformKeys
}

// MARK: - Account Info

public struct AccountInfo: Identifiable, Hashable, Sendable {
    public let id: String
    public let category: AccountCategory
    public let index: UInt32? // present only for indexed account types
    public let label: String
    public let balance: Balance
    public let addressCount: (external: Int, internal: Int)

    public init(category: AccountCategory,
                index: UInt32? = nil,
                label: String,
                balance: Balance,
                addressCount: (external: Int, internal: Int)) {
        self.category = category
        self.index = index
        self.label = label
        self.balance = balance
        self.addressCount = addressCount

        // Build a stable id
        if let idx = index {
            self.id = "\(category)-\(idx)"
        } else {
            self.id = "\(category)"
        }
    }

    public static func == (lhs: AccountInfo, rhs: AccountInfo) -> Bool {
        return lhs.id == rhs.id
    }

    public func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
}

// MARK: - Address Detail

public struct AddressDetail: Sendable {
    public let address: String
    public let index: UInt32
    public let path: String
    public let isUsed: Bool
    public let publicKey: String

    public init(address: String, index: UInt32, path: String, isUsed: Bool, publicKey: String) {
        self.address = address
        self.index = index
        self.path = path
        self.isUsed = isUsed
        self.publicKey = publicKey
    }
}

// MARK: - Account Detail Info

public struct AccountDetailInfo: Sendable {
    public let account: AccountInfo
    public let accountType: FFIAccountType
    public let xpub: String?
    public let derivationPath: String
    public let gapLimit: UInt32
    public let usedAddresses: Int
    public let unusedAddresses: Int
    public let externalAddresses: [AddressDetail]
    public let internalAddresses: [AddressDetail]

    public init(account: AccountInfo, accountType: FFIAccountType, xpub: String?, derivationPath: String, gapLimit: UInt32, usedAddresses: Int, unusedAddresses: Int, externalAddresses: [AddressDetail], internalAddresses: [AddressDetail]) {
        self.account = account
        self.accountType = accountType
        self.xpub = xpub
        self.derivationPath = derivationPath
        self.gapLimit = gapLimit
        self.usedAddresses = usedAddresses
        self.unusedAddresses = unusedAddresses
        self.externalAddresses = externalAddresses
        self.internalAddresses = internalAddresses
    }
}
