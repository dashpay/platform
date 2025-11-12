import Foundation

/// Main module for Dash Key Wallet functionality
///
/// The KeyWallet module provides comprehensive wallet management capabilities for Dash,
/// including HD key derivation, address generation, transaction management, and provider keys.
///
/// ## Key Features:
/// - Hierarchical Deterministic (HD) wallet support (BIP32/BIP44)
/// - Multiple account types (standard, CoinJoin, identity, provider)
/// - Address pool management with gap limits
/// - Transaction building and signing
/// - Provider key generation for masternodes
/// - BIP38 encryption/decryption
/// - Multi-wallet management
///
/// ## Usage Example:
/// ```swift
/// // Initialize the library
/// KeyWallet.initialize()
///
/// // Generate a new wallet
/// let mnemonic = try Mnemonic.generate()
/// let wallet = try Wallet(mnemonic: mnemonic, network: .testnet)
///
/// // Get a receive address
/// let managed = try ManagedWallet(wallet: wallet)
/// let address = try managed.getNextReceiveAddress(wallet: wallet)
///
/// // Check wallet balance
/// let balance = try wallet.getBalance()
/// print("Confirmed: \(balance.confirmed), Unconfirmed: \(balance.unconfirmed)")
/// ```
public class KeyWallet {
    
    /// Initialize the key wallet library
    /// Call this once at application startup
    public static func initialize() {
        _ = Wallet.initialize()
    }
    
    /// Get the library version
    public static var version: String {
        return Wallet.version
    }
    
    private init() {}
}

// Re-export all public types for convenience
public typealias KeyWalletWallet = Wallet
public typealias KeyWalletAccount = Account
public typealias KeyWalletManagedWallet = ManagedWallet
public typealias KeyWalletManager = WalletManager
public typealias KeyWalletMnemonic = Mnemonic
public typealias KeyWalletTransaction = Transaction
public typealias KeyWalletAddress = Address
// public typealias KeyWalletBIP38 = BIP38  // BIP38 functions not available in current FFI
public typealias KeyWalletDerivation = KeyDerivation
