# Bluetooth Mobile Wallet Integration

This module enables web applications to use a mobile device as both a context provider and secure wallet through Bluetooth Low Energy (BLE) communication.

## Features

- 🔐 **Secure Communication**: End-to-end encryption with ECDH key exchange and AES-256-GCM
- 📱 **Mobile Wallet**: Sign transactions and manage keys on mobile device
- 🌐 **Context Provider**: Get real-time platform state from mobile node
- 🔄 **Auto-reconnect**: Automatic reconnection on connection loss
- 🛡️ **Authentication**: Challenge-response authentication with signatures
- 📦 **Chunking**: Handle large messages with automatic chunking/reassembly

## Requirements

- Browser with Web Bluetooth API support (Chrome, Edge, Opera)
- HTTPS connection (required for Web Bluetooth)
- Mobile app implementing the Dash Bluetooth protocol

## Quick Start

```typescript
import { createBluetoothSDK } from 'dash';

// Simple setup
const sdk = await createBluetoothSDK();

// Use SDK normally - all operations will use Bluetooth
const identity = await sdk.identities.get('...');
```

## Advanced Usage

```typescript
import { setupBluetoothSDK } from 'dash';

// Advanced setup with options
const { sdk, provider, wallet, connection } = await setupBluetoothSDK({
  requireAuthentication: true,
  autoReconnect: true,
  timeout: 60000
});

// Monitor connection status
connection.on('disconnected', () => {
  console.log('Bluetooth disconnected');
});

connection.on('authenticated', (device) => {
  console.log('Authenticated with', device.name);
});

// Use provider directly
const blockHeight = await provider.getLatestPlatformBlockHeight();

// Use wallet directly
const addresses = await wallet.getAddresses();
```

## Security

### Pairing Process

1. **Device Discovery**: User selects device from browser's Bluetooth picker
2. **Pairing Code**: 9-digit code displayed on mobile, entered in web app
3. **Key Exchange**: ECDH P-256 key exchange for session encryption
4. **Authentication**: Challenge-response with signature verification

### Encryption

- **Key Exchange**: ECDH with P-256 curve
- **Session Encryption**: AES-256-GCM
- **Message Authentication**: ECDSA signatures
- **Replay Protection**: Nonce counter and timestamp validation

## Architecture

```
Web Application                    Mobile Device
     |                                  |
     |-------- BLE Connection --------->|
     |                                  |
     |<------- Encrypted Channel -------|
     |                                  |
     |-- Request (encrypted) ---------->|
     |                                  |
     |<-- Response (encrypted) ---------|
     |                                  |
```

## Message Flow

### Context Provider Operations

```typescript
// Automatic caching reduces Bluetooth traffic
const height = await provider.getLatestPlatformBlockHeight();
// Cached for 5 seconds by default

// Get all status in one request
const status = await provider.getPlatformStatus();
// Returns: blockHeight, blockTime, coreChainLockedHeight, version, timePerBlock
```

### Wallet Operations

```typescript
// Sign state transition
const signature = await wallet.signStateTransition(
  stateTransitionBytes,
  identityId,
  keyIndex,
  'ECDSA'
);

// Create asset lock proof
const proof = await wallet.createAssetLockProof({
  amount: 100000000,
  accountIndex: 0
});
```

## Browser Compatibility

| Browser | Support | Notes |
|---------|---------|-------|
| Chrome | ✅ | Version 56+ |
| Edge | ✅ | Version 79+ |
| Opera | ✅ | Version 43+ |
| Firefox | ❌ | No Web Bluetooth support |
| Safari | ❌ | No Web Bluetooth support |

## Mobile Implementation

See [MOBILE_INTERFACE.md](./MOBILE_INTERFACE.md) for the complete specification that mobile wallets must implement.

Key requirements:
- Bluetooth service UUID: `00000000-dash-platform-bluetooth-service`
- Three characteristics: Command, Response, Status
- JSON message protocol with chunking support
- Secure pairing and encryption

## Error Handling

```typescript
try {
  const sdk = await createBluetoothSDK();
} catch (error) {
  if (error.message.includes('not available')) {
    // Browser doesn't support Web Bluetooth
  } else if (error.message.includes('User cancelled')) {
    // User cancelled device selection
  } else if (error.message.includes('Authentication failed')) {
    // Pairing or authentication failed
  }
}
```

## Performance Considerations

- **Caching**: Context provider caches responses for 5 seconds
- **Chunking**: Large messages split into 512-byte chunks
- **Compression**: Consider implementing compression for large payloads
- **Batch Requests**: Use `getPlatformStatus()` instead of individual calls

## Development Tips

1. **Testing**: Use Chrome DevTools for Bluetooth debugging
2. **Security**: Always use HTTPS in development
3. **Reconnection**: Handle disconnections gracefully
4. **User Experience**: Show pairing instructions clearly
5. **Error Messages**: Provide helpful error messages for common issues

## Troubleshooting

### "Bluetooth not available"
- Check browser compatibility
- Ensure HTTPS connection
- Enable Bluetooth on computer

### "User cancelled"
- User closed device picker
- No compatible devices found

### "Authentication failed"
- Pairing code mismatch
- Mobile app rejected connection
- Timeout during pairing

### "Connection lost"
- Device out of range
- Mobile app closed
- Bluetooth disabled

## Future Enhancements

- WebSocket fallback for unsupported browsers
- Compression for large messages
- Batch request optimization
- Connection sharing between tabs
- QR code pairing option