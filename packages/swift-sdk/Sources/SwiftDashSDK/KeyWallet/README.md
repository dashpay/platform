# Dash Key Wallet Swift Interface

This directory contains the Swift wrapper for the Dash key-wallet-ffi library, providing comprehensive wallet management capabilities for iOS and macOS applications.

## Overview

The KeyWallet module provides:
- HD wallet support (BIP32/BIP44)
- Multiple account types (standard, CoinJoin, identity, provider)
- Enhanced address pool management with ManagedAccount
- Transaction building and signing
- Provider key generation for masternodes (BLS and EdDSA)
- BIP38 encryption/decryption
- Multi-wallet management with managed account collections

## Architecture

### Core Components

1. **Wallet** - Main wallet class for key derivation and account management
2. **ManagedWallet** - Extended wallet with address pool management and transaction checking
3. **WalletManager** - Multi-wallet manager for handling multiple wallets
4. **Account** - Individual account within a wallet
5. **ManagedAccount** - Enhanced account with address pool management
6. **ManagedAccountCollection** - Collection of all managed accounts in a wallet
7. **AccountCollection** - Collection of regular accounts with provider key support
8. **AddressPool** - Manages external/internal address pools for an account
9. **BLSAccount** - Specialized account for BLS provider keys
10. **EdDSAAccount** - Specialized account for EdDSA platform P2P keys
11. **Mnemonic** - Mnemonic generation and validation utilities
12. **Transaction** - Transaction building, signing, and checking
13. **ProviderKeys** - Provider key generation for masternode operations
14. **Address** - Address validation and type detection
15. **BIP38** - BIP38 encryption/decryption for private keys
16. **KeyDerivation** - Low-level key derivation utilities

### FFI Integration

The Swift interface uses the C FFI bindings from key-wallet-ffi through the CKeyWalletFFI module. Memory management is handled automatically using Swift's ARC and proper cleanup in deinit methods.

## Usage Examples

### Basic Wallet Creation

```swift
import SwiftDashSDK

// Initialize the library
KeyWallet.initialize()

// Generate a new mnemonic
let mnemonic = try Mnemonic.generate(wordCount: 24)

// Create wallet from mnemonic
let wallet = try Wallet(
    mnemonic: mnemonic,
    passphrase: nil,
    network: .testnet
)

// Get wallet ID
let walletId = try wallet.id
print("Wallet ID: \(walletId.toHexString())")
```

### Address Generation

```swift
// Create managed wallet for address pool management
let managed = try ManagedWallet(wallet: wallet)

// Get next receive address
let receiveAddress = try managed.getNextReceiveAddress(wallet: wallet)
print("Receive address: \(receiveAddress)")

// Get next change address
let changeAddress = try managed.getNextChangeAddress(wallet: wallet)
print("Change address: \(changeAddress)")

// Get a range of addresses
let addresses = try managed.getExternalAddressRange(
    wallet: wallet,
    accountIndex: 0,
    startIndex: 0,
    endIndex: 10
)
```

### Transaction Management

```swift
// Build a transaction
let outputs = [
    Transaction.Output(address: "XqHiz8EXYbTAtBEYs4pWTHh7ipEDQcNQeT", amount: 100000000)
]

let txData = try Transaction.build(
    wallet: wallet,
    accountIndex: 0,
    outputs: outputs,
    feePerKB: 1000
)

// Sign the transaction
let signedTx = try Transaction.sign(wallet: wallet, transactionData: txData)

// Check if a transaction belongs to the wallet
let checkResult = try Transaction.check(
    wallet: wallet,
    transactionData: signedTx,
    context: .mempool
)

if checkResult.isRelevant {
    print("Transaction affects this wallet")
    print("Received: \(checkResult.totalReceived)")
    print("Sent: \(checkResult.totalSent)")
}
```

### Provider Keys for Masternodes

```swift
// Generate provider voting key
let votingKey = try ProviderKeys.generateKey(
    wallet: wallet,
    keyType: .voting,
    keyIndex: 0,
    includePrivate: true
)

print("Voting public key: \(votingKey.publicKey.toHexString())")
print("Derivation path: \(votingKey.derivationPath)")

// Get address for funding
let fundingAddress = try ProviderKeys.getAddress(
    wallet: wallet,
    keyType: .voting,
    keyIndex: 0
)
```

### Multi-Wallet Management

```swift
// Create wallet manager
let manager = try WalletManager()

// Add wallets
let walletId1 = try manager.addWallet(
    mnemonic: mnemonic1,
    network: .mainnet
)

let walletId2 = try manager.addWallet(
    mnemonic: mnemonic2,
    network: .mainnet
)

// Get all wallet IDs
let walletIds = try manager.getWalletIds()

// Get next receive address for a wallet
let address = try manager.getReceiveAddress(
    walletId: walletId1,
    network: .mainnet,
    accountIndex: 0
)

// Process transaction across all wallets
let isRelevant = try manager.processTransaction(
    txData,
    network: .mainnet,
    contextDetails: TransactionContextDetails(
        context: .inBlock,
        height: 1000000,
        blockHash: blockHashData,
        timestamp: UInt32(Date().timeIntervalSince1970)
    ),
    updateStateIfFound: true
)
```

### Managed Accounts (New API)

```swift
// Get a managed account from wallet manager
let managedAccount = try manager.getManagedAccount(
    walletId: walletId,
    network: .mainnet,
    accountIndex: 0,
    accountType: .standardBIP44
)

// Get account properties
print("Network: \(managedAccount.network)")
print("Account type: \(managedAccount.accountType)")
print("Is watch-only: \(managedAccount.isWatchOnly)")
print("Transaction count: \(managedAccount.transactionCount)")

// Get balance
let balance = try managedAccount.getBalance()
print("Confirmed: \(balance.confirmed), Unconfirmed: \(balance.unconfirmed)")

// Access address pools
if let externalPool = managedAccount.getExternalAddressPool() {
    // Get specific address
    let addressInfo = try externalPool.getAddress(at: 0)
    print("Address: \(addressInfo.address)")
    print("Path: \(addressInfo.path)")
    print("Used: \(addressInfo.used)")
    
    // Get range of addresses
    let addresses = try externalPool.getAddresses(from: 0, to: 10)
    for addr in addresses {
        print("\(addr.index): \(addr.address)")
    }
}

// Get managed account collection
let collection = try manager.getManagedAccountCollection(
    walletId: walletId,
    network: .mainnet
)

// Access different account types
if let bip44Account = collection.getBIP44Account(at: 0) {
    print("BIP44 account found")
}

if collection.hasIdentityRegistration {
    if let identityAccount = collection.getIdentityRegistrationAccount() {
        print("Identity registration account available")
    }
}

// Get summary of all accounts
if let summary = collection.getSummary() {
    print("BIP44 accounts: \(summary.bip44Indices)")
    print("Has provider keys: \(summary.hasProviderVotingKeys)")
}
```

### Account Collections

```swift
// Get account collection from wallet
let accountCollection = try wallet.getAccountCollection()

// Get provider accounts
if let blsOperatorAccount = accountCollection.getProviderOperatorKeys() {
    // BLS operator keys account
    print("BLS operator account available")
}

if let eddsaPlatformAccount = accountCollection.getProviderPlatformKeys() {
    // EdDSA platform P2P keys account
    print("EdDSA platform account available")
}

// Get collection summary
if let summary = accountCollection.getSummary() {
    print("Account summary:")
    print("- BIP44 indices: \(summary.bip44Indices)")
    print("- Identity accounts: Registration=\(summary.hasIdentityRegistration)")
    print("- Provider accounts: Voting=\(summary.hasProviderVotingKeys)")
}
```

### BIP38 Encryption

```swift
// Encrypt a private key
let encrypted = try BIP38.encrypt(
    privateKey: "cVRnH5vFxVxWFWEXLBXLcNYFKgLiC7kDiXjHEcRFQ8gfFfqH7eQA",
    passphrase: "mypassword",
    network: .mainnet
)

// Decrypt
let decrypted = try BIP38.decrypt(
    encryptedKey: encrypted,
    passphrase: "mypassword"
)
```

## Account Types

The wallet supports multiple account types:

- **StandardBIP44**: Regular BIP44 accounts (m/44'/5'/account'/x/x)
- **StandardBIP32**: BIP32 accounts (m/account'/x/x)
- **CoinJoin**: Privacy-enhanced transactions
- **IdentityRegistration**: Funding for identity registration
- **IdentityTopUp**: Funding for identity top-ups (with registration index)
- **IdentityTopUpNotBound**: Identity top-up not bound to specific identity
- **IdentityInvitation**: Funding for identity invitations
- **ProviderVotingKeys**: Masternode voting keys (BLS)
- **ProviderOwnerKeys**: Masternode owner keys (BLS)
- **ProviderOperatorKeys**: Masternode operator keys (BLS)
- **ProviderPlatformKeys**: Platform P2P keys (EdDSA)

### Account Creation Options

When creating a wallet, you can specify account creation options:

- **`.default`**: Create default accounts (BIP44 account 0, CoinJoin account 0, and special accounts)
- **`.allAccounts`**: Create all specified accounts plus all special purpose accounts
- **`.bip44AccountsOnly`**: Create only BIP44 accounts (no CoinJoin or special accounts)
- **`.specificAccounts`**: Create specific accounts with full control
- **`.none`**: Create no accounts at all (uses `NO_ACCOUNTS` enum value)

## Network Support

The library supports all Dash networks:
- Mainnet
- Testnet
- Regtest
- Devnet

## Error Handling

All operations that can fail throw `KeyWalletError` with detailed error information:

```swift
do {
    let wallet = try Wallet(mnemonic: mnemonic, network: .testnet)
} catch KeyWalletError.invalidMnemonic(let message) {
    print("Invalid mnemonic: \(message)")
} catch KeyWalletError.invalidState(let message) {
    print("Invalid state: \(message)")
} catch {
    print("Unexpected error: \(error)")
}
```

## Memory Management

The Swift interface handles all memory management automatically:
- FFI resources are properly freed in deinit methods
- Temporary C strings are managed with proper lifetime
- Arrays and buffers are correctly allocated and freed
- No manual memory management required

## Thread Safety

The underlying Rust library provides thread-safe operations. However, Swift wrapper objects should be used from a single thread or properly synchronized when shared across threads.

## Requirements

- iOS 13.0+ / macOS 10.15+
- Swift 5.0+
- Linked with key_wallet_ffi static library

## Building

1. Build the key-wallet-ffi library for iOS:
   ```bash
   cd /path/to/rust-dashcore/key-wallet-ffi
   ./build_ios.sh
   ```

2. Link the generated xcframework in your Xcode project

3. Import the module:
   ```swift
   import SwiftDashSDK
   ```