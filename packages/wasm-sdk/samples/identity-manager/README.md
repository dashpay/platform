# Identity Manager - Sample Application

A comprehensive identity management application demonstrating the Dash Platform WASM SDK capabilities for identity operations.

## Features

### 🔍 Identity Lookup
- Look up any identity by Base58-encoded ID
- View complete identity information including public keys and metadata
- Support for both testnet and mainnet networks
- Real-time validation and error handling

### 💰 Balance Management
- Check identity balance in credits and DASH
- View identity revision information
- Real-time balance updates

### 🔑 Public Key Management
- Display all public keys associated with an identity
- Show key details including type, purpose, and security level
- Support for different key types (ECDSA, BLS, etc.)
- Hex data visualization

### ➕ Identity Creation
- Create new identities with asset lock proof
- Support for custom public key configurations
- Form validation and error handling
- Sample key generation for testing

### 📊 Operations Logging
- Real-time logging of all operations
- Success/error status tracking
- Exportable log data for debugging

## Quick Start

### Prerequisites
- Modern web browser with WebAssembly support
- Local web server (required for WASM module loading)

### Running the Application

1. **Start a local web server** from the wasm-sdk directory:
   ```bash
   # Using Python
   python3 -m http.server 8888
   
   # Or using Node.js
   npx http-server -p 8888
   ```

2. **Open in browser**:
   ```
   http://localhost:8888/samples/identity-manager/
   ```

3. **Test with sample data**:
   - Click "Use Sample ID" to load a known testnet identity
   - Or enter your own identity ID for lookup

## Usage Examples

### Basic Identity Lookup

1. Select network (testnet recommended for testing)
2. Enter a valid identity ID or click "Use Sample ID"
3. Click "Lookup" to fetch identity data
4. View the complete identity information in the results panel

### Checking Identity Balance

1. After looking up an identity, click "Check Balance"
2. View balance in both credits and DASH
3. See identity revision information

### Viewing Public Keys

1. After looking up an identity, click "View Keys"
2. Explore all public keys with detailed information
3. View key purposes, types, and hex data

### Creating New Identity

⚠️ **Warning**: Identity creation requires real asset lock proofs and private keys. Use with caution on mainnet.

1. Fill in asset lock proof (transaction hex)
2. Provide asset lock private key
3. Add public keys in JSON format (or use "Generate Sample Keys")
4. Click "Create Identity"

## Network Support

### Testnet (Recommended)
- **Endpoint**: Uses trusted testnet quorums
- **Sample Identity**: `5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk`
- **Perfect for testing and development**

### Mainnet
- **Endpoint**: Uses trusted mainnet quorums  
- **Real DASH required for operations**
- **Use with caution**

## API Methods Demonstrated

This application showcases the following WASM SDK methods:

```javascript
// Identity operations
await sdk.get_identity(identityId)
await sdk.get_identity_balance(identityId)
await sdk.get_identity_keys(identityId, 'all', null, null, null, null)

// Identity creation
await sdk.identity_create(assetLockProof, privateKey, publicKeysJson)
```

## Error Handling

The application includes comprehensive error handling for:

- **Network connectivity issues**
- **Invalid identity IDs**
- **Missing identity data**
- **Malformed input data**
- **SDK initialization failures**

All errors are displayed with user-friendly messages and logged for debugging.

## Data Export

### Identity Data Export
- Export complete identity information as JSON
- Includes timestamp and network information
- Downloadable file for record keeping

### Operations Log Export
- Export all operations log as text file
- Includes timestamps and success/error status
- Useful for debugging and auditing

## Browser Compatibility

- **Chrome 80+**: Full support
- **Firefox 75+**: Full support  
- **Safari 13+**: Full support
- **Edge 80+**: Full support

## Development

### File Structure
```
identity-manager/
├── index.html          # Main application HTML
├── styles.css          # Application styles
├── app.js             # Core application logic
└── README.md          # This documentation
```

### Key Components

#### IdentityManager Class
- Main application controller
- Handles SDK initialization and operations
- Manages UI state and interactions

#### SDK Integration
- Uses WASM SDK builder pattern
- Supports both testnet and mainnet
- Automatic error handling and retry logic

#### UI Components
- Responsive design for desktop and mobile
- Real-time status indicators
- Form validation and user feedback

## Troubleshooting

### Common Issues

**WASM Module Loading Fails**
- Ensure you're running from a web server (not file://)
- Check browser console for detailed error messages
- Verify WASM files are present in `../../pkg/` directory

**Identity Not Found**
- Verify the identity ID is correct and Base58 encoded
- Check that you're on the correct network (testnet vs mainnet)
- Some identities may not exist on all networks

**Connection Issues**
- Check internet connectivity
- Network endpoints may be temporarily unavailable
- Try switching between testnet and mainnet

**Performance Issues**
- WASM initialization may take 5-15 seconds on first load
- Large identity datasets may take time to process
- Consider using the operations log to track progress

### Debug Mode

Enable detailed logging by opening browser developer tools and checking the console for additional error information.

## Security Considerations

- **Private Keys**: Never enter real private keys in the creation form unless you understand the risks
- **Mainnet Operations**: Use extreme caution when operating on mainnet with real funds
- **Data Storage**: No sensitive data is stored locally - all operations are performed in memory

## Integration Examples

### Using in Your Application

```javascript
import { IdentityManager } from './identity-manager/app.js';

// Initialize the manager
const manager = new IdentityManager();
await manager.init();

// Look up an identity
const identity = await manager.lookupIdentity('your-identity-id');

// Check balance
const balance = await manager.checkBalance();
```

### Custom Network Configuration

```javascript
// Modify the SDK builder configuration
const builder = WasmSdkBuilder.new_testnet()
    .with_settings(3000, 60000, 5, true); // Custom timeout settings
```

This sample application serves as a comprehensive reference for building identity management features using the Dash Platform WASM SDK.