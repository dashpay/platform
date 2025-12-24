import Foundation
import DashSDKFFI

// MARK: - Network Types

/// Helper to create FFINetworks bitmap from multiple networks
public struct NetworkSet {
    public let networks: Set<KeyWalletNetwork>
    
    public init(_ networks: KeyWalletNetwork...) {
        self.networks = Set(networks)
    }
    
    public init(_ networks: [KeyWalletNetwork]) {
        self.networks = Set(networks)
    }
    
    public var ffiNetworks: FFINetworks {
        var bitmap: UInt32 = 0
        for network in networks {
            switch network {
            case .mainnet: bitmap |= (1 << 0)  // DASH_FLAG
            case .testnet: bitmap |= (1 << 1)  // TESTNET_FLAG
            case .regtest: bitmap |= (1 << 2)  // REGTEST_FLAG
            case .devnet: bitmap |= (1 << 3)   // DEVNET_FLAG
            }
        }
        return FFINetworks(rawValue: bitmap)
    }
}

/// Network type for Dash networks
public enum KeyWalletNetwork: UInt32 {
    case mainnet = 0  // DASH
    case testnet = 1  // TESTNET
    case regtest = 2  // REGTEST
    case devnet = 3   // DEVNET
    
    var ffiValue: FFINetwork {
        switch self {
        case .mainnet: return FFINetwork(rawValue: 0)  // Dash = 0
        case .testnet: return FFINetwork(rawValue: 1)  // Testnet = 1
        case .regtest: return FFINetwork(rawValue: 2)  // Regtest = 2
        case .devnet: return FFINetwork(rawValue: 3)   // Devnet = 3
        }
    }
    
    init(ffiNetwork: FFINetwork) {
        switch ffiNetwork.rawValue {
        case 0: self = .mainnet  // Dash = 0
        case 1: self = .testnet  // Testnet = 1
        case 2: self = .regtest  // Regtest = 2
        case 3: self = .devnet   // Devnet = 3
        default: self = .mainnet
        }
    }
}

// MARK: - Account Types

/// Account type for wallet accounts
public enum AccountType: UInt32 {
    case standardBIP44 = 0
    case standardBIP32 = 1
    case coinJoin = 2
    case identityRegistration = 3
    case identityTopUp = 4
    case identityTopUpNotBound = 5
    case identityInvitation = 6
    case providerVotingKeys = 7
    case providerOwnerKeys = 8
    case providerOperatorKeys = 9
    case providerPlatformKeys = 10
    
    var ffiValue: FFIAccountType {
        FFIAccountType(rawValue: self.rawValue)
    }
    
    init(ffiType: FFIAccountType) {
        self = AccountType(rawValue: ffiType.rawValue) ?? .standardBIP44
    }
}

// MARK: - Address Pool Types

/// Address pool type
public enum AddressPoolType: UInt32 {
    case external = 0  // Receive addresses
    case `internal` = 1  // Change addresses
    case single = 2    // Single pool for non-standard accounts
    
    var ffiValue: FFIAddressPoolType {
        FFIAddressPoolType(rawValue: self.rawValue)
    }
    
    init(ffiType: FFIAddressPoolType) {
        self = AddressPoolType(rawValue: ffiType.rawValue) ?? .external
    }
}

// MARK: - Transaction Context

/// Transaction context for checking
public enum TransactionContext: UInt32 {
    case mempool = 0
    case inBlock = 1
    case inChainLockedBlock = 2
    
    var ffiValue: FFITransactionContext {
        FFITransactionContext(rawValue: self.rawValue)
    }
    
    init(ffiContext: FFITransactionContext) {
        self = TransactionContext(rawValue: ffiContext.rawValue) ?? .mempool
    }
}

// MARK: - Mnemonic Language

/// Language for mnemonic generation
public enum MnemonicLanguage: UInt32 {
    case english = 0
    case chineseSimplified = 1
    case chineseTraditional = 2
    case czech = 3
    case french = 4
    case italian = 5
    case japanese = 6
    case korean = 7
    case portuguese = 8
    case spanish = 9
    
    var ffiValue: FFILanguage {
        FFILanguage(rawValue: self.rawValue)
    }
    
    init(ffiLanguage: FFILanguage) {
        self = MnemonicLanguage(rawValue: ffiLanguage.rawValue) ?? .english
    }
}

// MARK: - Account Creation Options

/// Options for account creation when creating a wallet
public enum AccountCreationOption {
    /// Create default accounts (BIP44 account 0, CoinJoin account 0, and special accounts)
    case `default`
    /// Create all specified accounts plus all special purpose accounts
    case allAccounts
    /// Create only BIP44 accounts (no CoinJoin or special accounts)
    case bip44AccountsOnly
    /// Create specific accounts with full control
    case specificAccounts(bip44: [UInt32], bip32: [UInt32], coinJoin: [UInt32], 
                          topUp: [UInt32], specialTypes: [AccountType])
    /// Create no accounts at all
    case noAccounts
    
    func toFFIOptions() -> FFIWalletAccountCreationOptions {
        var options = FFIWalletAccountCreationOptions()
        
        switch self {
        case .default:
            options.option_type = FFIAccountCreationOptionType(rawValue: 0) // DEFAULT
        case .allAccounts:
            options.option_type = FFIAccountCreationOptionType(rawValue: 1) // ALL_ACCOUNTS
        case .bip44AccountsOnly:
            options.option_type = FFIAccountCreationOptionType(rawValue: 2) // BIP44_ACCOUNTS_ONLY
        case .specificAccounts(let bip44, let bip32, let coinJoin, let topUp, let specialTypes):
            options.option_type = FFIAccountCreationOptionType(rawValue: 3) // SPECIFIC_ACCOUNTS
            
            // Note: These would need to be stored and passed properly
            // This is simplified - actual implementation would need to manage memory
            options.bip44_count = bip44.count
            options.bip32_count = bip32.count
            options.coinjoin_count = coinJoin.count
            options.topup_count = topUp.count
            options.special_account_types_count = specialTypes.count
        case .noAccounts:
            options.option_type = FFIAccountCreationOptionType(rawValue: 4) // NO_ACCOUNTS
        }
        
        return options
    }
}

// Note: DerivationPathType removed (FFIDerivationPathType not present in current headers).

// MARK: - Result Types

/// Balance information for a wallet or account
public struct Balance {
    public let confirmed: UInt64
    public let unconfirmed: UInt64
    public let immature: UInt64
    public let total: UInt64
    
    init(ffiBalance: FFIBalance) {
        self.confirmed = ffiBalance.confirmed
        self.unconfirmed = ffiBalance.unconfirmed
        self.immature = ffiBalance.immature
        self.total = ffiBalance.total
    }
}

/// Address pool information
public struct AddressPoolInfo {
    public let poolType: AddressPoolType
    public let generatedCount: UInt32
    public let usedCount: UInt32
    public let currentGap: UInt32
    public let gapLimit: UInt32
    public let highestUsedIndex: Int32
    
    init(ffiInfo: FFIAddressPoolInfo) {
        self.poolType = AddressPoolType(ffiType: ffiInfo.pool_type)
        self.generatedCount = ffiInfo.generated_count
        self.usedCount = ffiInfo.used_count
        self.currentGap = ffiInfo.current_gap
        self.gapLimit = ffiInfo.gap_limit
        self.highestUsedIndex = ffiInfo.highest_used_index
    }
}

/// Transaction check result
public struct TransactionCheckResult {
    public let isRelevant: Bool
    public let totalReceived: UInt64
    public let totalSent: UInt64
    public let affectedAccountsCount: UInt32
    
    init(ffiResult: FFITransactionCheckResult) {
        self.isRelevant = ffiResult.is_relevant
        self.totalReceived = ffiResult.total_received
        self.totalSent = ffiResult.total_sent
        self.affectedAccountsCount = ffiResult.affected_accounts_count
    }
}

/// Transaction context details
public struct TransactionContextDetails {
    public let context: TransactionContext
    public let height: UInt32
    public let blockHash: Data?
    public let timestamp: UInt32
    
    func toFFI() -> FFITransactionContextDetails {
        var details = FFITransactionContextDetails()
        details.context_type = context.ffiValue
        details.height = height
        details.timestamp = timestamp
        
        if let hash = blockHash {
            hash.withUnsafeBytes { bytes in
                details.block_hash = bytes.bindMemory(to: UInt8.self).baseAddress
            }
        }
        
        return details
    }
}

/// UTXO information
public struct UTXO {
    public let txid: Data
    public let vout: UInt32
    public let amount: UInt64
    public let address: String
    public let scriptPubKey: Data
    public let height: UInt32
    public let confirmations: UInt32
    
    init(ffiUTXO: FFIUTXO) {
        // Copy txid (32 bytes)
        self.txid = withUnsafeBytes(of: ffiUTXO.txid) { Data($0) }
        self.vout = ffiUTXO.vout
        self.amount = ffiUTXO.amount
        
        // Copy address string
        if let addressPtr = ffiUTXO.address {
            self.address = String(cString: addressPtr)
        } else {
            self.address = ""
        }
        
        // Copy script pubkey
        if let scriptPtr = ffiUTXO.script_pubkey, ffiUTXO.script_len > 0 {
            self.scriptPubKey = Data(bytes: scriptPtr, count: ffiUTXO.script_len)
        } else {
            self.scriptPubKey = Data()
        }
        
        self.height = ffiUTXO.height
        self.confirmations = ffiUTXO.confirmations
    }
}

// MARK: - Account Collection Types

/// Summary of accounts in a collection
public struct AccountCollectionSummary {
    public let bip44Indices: [UInt32]
    public let bip32Indices: [UInt32]
    public let coinJoinIndices: [UInt32]
    public let identityTopUpIndices: [UInt32]
    public let hasIdentityRegistration: Bool
    public let hasIdentityInvitation: Bool
    public let hasIdentityTopUpNotBound: Bool
    public let hasProviderVotingKeys: Bool
    public let hasProviderOwnerKeys: Bool
    public let hasProviderOperatorKeys: Bool
    public let hasProviderPlatformKeys: Bool
    
    init(ffiSummary: FFIAccountCollectionSummary) {
        // Convert BIP44 indices
        if ffiSummary.bip44_count > 0, let indices = ffiSummary.bip44_indices {
            self.bip44Indices = Array(UnsafeBufferPointer(start: indices, count: ffiSummary.bip44_count))
        } else {
            self.bip44Indices = []
        }
        
        // Convert BIP32 indices
        if ffiSummary.bip32_count > 0, let indices = ffiSummary.bip32_indices {
            self.bip32Indices = Array(UnsafeBufferPointer(start: indices, count: ffiSummary.bip32_count))
        } else {
            self.bip32Indices = []
        }
        
        // Convert CoinJoin indices
        if ffiSummary.coinjoin_count > 0, let indices = ffiSummary.coinjoin_indices {
            self.coinJoinIndices = Array(UnsafeBufferPointer(start: indices, count: ffiSummary.coinjoin_count))
        } else {
            self.coinJoinIndices = []
        }
        
        // Convert identity top-up indices
        if ffiSummary.identity_topup_count > 0, let indices = ffiSummary.identity_topup_indices {
            self.identityTopUpIndices = Array(UnsafeBufferPointer(start: indices, count: ffiSummary.identity_topup_count))
        } else {
            self.identityTopUpIndices = []
        }
        
        // Copy boolean flags
        self.hasIdentityRegistration = ffiSummary.has_identity_registration
        self.hasIdentityInvitation = ffiSummary.has_identity_invitation
        self.hasIdentityTopUpNotBound = ffiSummary.has_identity_topup_not_bound
        self.hasProviderVotingKeys = ffiSummary.has_provider_voting_keys
        self.hasProviderOwnerKeys = ffiSummary.has_provider_owner_keys
        self.hasProviderOperatorKeys = ffiSummary.has_provider_operator_keys
        self.hasProviderPlatformKeys = ffiSummary.has_provider_platform_keys
    }
}

/// Summary of managed accounts in a collection
public struct ManagedAccountCollectionSummary {
    public let bip44Indices: [UInt32]
    public let bip32Indices: [UInt32]
    public let coinJoinIndices: [UInt32]
    public let identityTopUpIndices: [UInt32]
    public let hasIdentityRegistration: Bool
    public let hasIdentityInvitation: Bool
    public let hasIdentityTopUpNotBound: Bool
    public let hasProviderVotingKeys: Bool
    public let hasProviderOwnerKeys: Bool
    public let hasProviderOperatorKeys: Bool
    public let hasProviderPlatformKeys: Bool
    
    init(ffiSummary: FFIManagedAccountCollectionSummary) {
        // Convert BIP44 indices
        if ffiSummary.bip44_count > 0, let indices = ffiSummary.bip44_indices {
            self.bip44Indices = Array(UnsafeBufferPointer(start: indices, count: ffiSummary.bip44_count))
        } else {
            self.bip44Indices = []
        }
        
        // Convert BIP32 indices
        if ffiSummary.bip32_count > 0, let indices = ffiSummary.bip32_indices {
            self.bip32Indices = Array(UnsafeBufferPointer(start: indices, count: ffiSummary.bip32_count))
        } else {
            self.bip32Indices = []
        }
        
        // Convert CoinJoin indices
        if ffiSummary.coinjoin_count > 0, let indices = ffiSummary.coinjoin_indices {
            self.coinJoinIndices = Array(UnsafeBufferPointer(start: indices, count: ffiSummary.coinjoin_count))
        } else {
            self.coinJoinIndices = []
        }
        
        // Convert identity top-up indices
        if ffiSummary.identity_topup_count > 0, let indices = ffiSummary.identity_topup_indices {
            self.identityTopUpIndices = Array(UnsafeBufferPointer(start: indices, count: ffiSummary.identity_topup_count))
        } else {
            self.identityTopUpIndices = []
        }
        
        // Copy boolean flags
        self.hasIdentityRegistration = ffiSummary.has_identity_registration
        self.hasIdentityInvitation = ffiSummary.has_identity_invitation
        self.hasIdentityTopUpNotBound = ffiSummary.has_identity_topup_not_bound
        self.hasProviderVotingKeys = ffiSummary.has_provider_voting_keys
        self.hasProviderOwnerKeys = ffiSummary.has_provider_owner_keys
        self.hasProviderOperatorKeys = ffiSummary.has_provider_operator_keys
        self.hasProviderPlatformKeys = ffiSummary.has_provider_platform_keys
    }
}

// MARK: - Error Handling

/// Key wallet errors
public enum KeyWalletError: Error {
    case invalidInput(String)
    case allocationFailed(String)
    case invalidMnemonic(String)
    case invalidDerivationPath(String)
    case invalidNetwork(String)
    case invalidAddress(String)
    case invalidTransaction(String)
    case walletError(String)
    case serializationError(String)
    case notFound(String)
    case notSupported(String)
    case invalidState(String)
    case internalError(String)
    case unknown(String)
    
    init(ffiError: FFIError) {
        let message = ffiError.message != nil ? String(cString: ffiError.message!) : "Unknown error"
        
        switch ffiError.code {
        case FFIErrorCode(rawValue: 1): // INVALID_INPUT
            self = .invalidInput(message)
        case FFIErrorCode(rawValue: 2): // ALLOCATION_FAILED
            self = .allocationFailed(message)
        case FFIErrorCode(rawValue: 3): // INVALID_MNEMONIC
            self = .invalidMnemonic(message)
        case FFIErrorCode(rawValue: 4): // INVALID_DERIVATION_PATH
            self = .invalidDerivationPath(message)
        case FFIErrorCode(rawValue: 5): // INVALID_NETWORK
            self = .invalidNetwork(message)
        case FFIErrorCode(rawValue: 6): // INVALID_ADDRESS
            self = .invalidAddress(message)
        case FFIErrorCode(rawValue: 7): // INVALID_TRANSACTION
            self = .invalidTransaction(message)
        case FFIErrorCode(rawValue: 8): // WALLET_ERROR
            self = .walletError(message)
        case FFIErrorCode(rawValue: 9): // SERIALIZATION_ERROR
            self = .serializationError(message)
        case FFIErrorCode(rawValue: 10): // NOT_FOUND
            self = .notFound(message)
        case FFIErrorCode(rawValue: 11): // INVALID_STATE
            self = .invalidState(message)
        case FFIErrorCode(rawValue: 12): // INTERNAL_ERROR
            self = .internalError(message)
        default:
            self = .unknown(message)
        }
    }
}

extension KeyWalletError: LocalizedError {
    public var errorDescription: String? {
        switch self {
        case .invalidInput(let msg): return "Invalid Input: \(msg)"
        case .allocationFailed(let msg): return "Allocation Failed: \(msg)"
        case .invalidMnemonic(let msg): return "Invalid Mnemonic: \(msg)"
        case .invalidDerivationPath(let msg): return "Invalid Derivation Path: \(msg)"
        case .invalidNetwork(let msg): return "Invalid Network: \(msg)"
        case .invalidAddress(let msg): return "Invalid Address: \(msg)"
        case .invalidTransaction(let msg): return "Invalid Transaction: \(msg)"
        case .walletError(let msg): return "Wallet Error: \(msg)"
        case .serializationError(let msg): return "Serialization Error: \(msg)"
        case .notFound(let msg): return "Not Found: \(msg)"
        case .notSupported(let msg): return "Not Supported: \(msg)"
        case .invalidState(let msg): return "Invalid State: \(msg)"
        case .internalError(let msg): return "Internal Error: \(msg)"
        case .unknown(let msg): return "Unknown Error: \(msg)"
        }
    }
}
