# Swift Example App

An iOS example application demonstrating the Swift Dash SDK capabilities.

## Features

- **Identities Tab**: Load existing identities, create local identities, fetch from network, view balances
- **Tokens Tab**: Select an identity and perform token operations (transfer, mint, burn)
- **Documents Tab**: Create and manage documents on Dash Platform
- **Contracts Tab**: Browse and fetch data contracts

## Requirements

- iOS 16.0+
- Xcode 14.0+
- Swift Package Manager

## Setup

1. Open the project in Xcode:
   ```bash
   cd packages/swift-sdk/SwiftExampleApp
   xed .
   ```

2. Build and run the app in the iOS Simulator or on a device

## Architecture

The app uses SwiftUI and follows MVVM architecture:

- **Models**: Data structures for Identity, Token, Document, and Contract
- **Views**: SwiftUI views for each tab and feature
- **AppState**: Central state management using ObservableObject
- **SDK Integration**: Uses the Swift Dash SDK for platform operations

## Token Actions

The app supports all token actions from the Dash Platform:

### Basic Operations
- ✅ **Transfer**: Send tokens to another identity with optional notes
- ✅ **Mint**: Create new tokens with optional recipient specification
- ✅ **Burn**: Permanently destroy tokens (with warning)

### Distribution & Claims
- ✅ **Claim**: Claim tokens from rewards and airdrops
  - Shows available distributions
  - Automatic claiming process

### Security Features
- ✅ **Freeze**: Temporarily lock tokens with reason tracking
  - Prevents transfer until unfrozen
  - Optional reason documentation
- ✅ **Unfreeze**: Restore previously frozen tokens
  - Shows frozen balance
  - Immediate availability after unfreezing
- ✅ **Destroy Frozen Funds**: Permanently remove frozen tokens
  - Requires confirmation reason
  - Audit trail for compliance

### Trading
- ✅ **Direct Purchase**: Buy tokens at set prices
  - Shows current price (0.001 DASH per token)
  - Real-time cost calculation
  - Deducted from identity balance

## Development Notes

- The app uses a test signer for development purposes
- Sample data is loaded for demonstration
- Real SDK operations are simulated with success messages
- Error handling displays alerts to the user

## Identity Loading

The Load Identity feature allows you to import existing identities:

### For User Identities:
- Enter Identity ID (Hex or Base58)
- Optionally add private keys for signing transactions
- Set an alias for easy identification

### For Masternode/Evonode Identities:
- Enter ProTxHash
- Add voting private key
- Add owner private key
- For Evonodes: Add payout address private key

### Testnet Features:
- **Fill Random HPMN**: Auto-fills with random High Performance Masternode data
- **Fill Random Masternode**: Auto-fills with random Masternode data
- Sample testnet nodes are included for testing

## Testing

The app includes sample identities and tokens for testing:
- Alice: Local test identity with 10 DASH balance
- Bob: Local test identity with 5 DASH balance  
- Charlie: Local test identity with 2.5 DASH balance
- Support for loading Masternode and Evonode identities with proper key management