# Mobile Device Bluetooth Interface Specification

This document specifies the Bluetooth interface that mobile wallets must implement to be compatible with the Dash SDK Bluetooth provider and wallet.

## Overview

Mobile devices act as:
1. **Context Provider**: Supplying real-time platform state information
2. **Secure Wallet**: Signing transactions and managing keys
3. **Authentication Device**: Secure pairing and session management

## Bluetooth Service Configuration

### Service UUID
```
00000000-dash-platform-bluetooth-service
```

### Characteristics

1. **Command Characteristic** (Write)
   - UUID: `00000001-dash-platform-command-char`
   - Properties: Write with response
   - Max length: 512 bytes per write

2. **Response Characteristic** (Notify)
   - UUID: `00000002-dash-platform-response-char`
   - Properties: Notify
   - Max length: 512 bytes per notification

3. **Status Characteristic** (Read/Notify)
   - UUID: `00000003-dash-platform-status-char`
   - Properties: Read, Notify
   - Format: JSON status object

## Communication Protocol

### Message Format

All messages use JSON encoding with the following structure:

```typescript
interface BluetoothMessage {
  id: string;          // Unique message ID
  type: MessageType;   // Message type enum
  payload?: any;       // Optional payload data
  timestamp: number;   // Unix timestamp
  signature?: string;  // Optional message signature
}
```

### Response Format

```typescript
interface BluetoothResponse {
  id: string;          // Original request ID
  type: MessageType;   // Same as request type
  success: boolean;    // Success/failure flag
  data?: any;         // Response data if successful
  error?: {
    code: string;
    message: string;
  };
  timestamp: number;
}
```

### Chunking Protocol

For messages larger than 512 bytes:
1. Split into chunks with 2-byte header: `[chunk_index, total_chunks]`
2. Send chunks sequentially with 50ms delay
3. Reassemble on receiving side

## Required Message Handlers

### Context Provider Messages

#### GET_PLATFORM_STATUS
Returns all platform status in one response:
```json
{
  "blockHeight": 123456,
  "blockTime": 1699564800000,
  "coreChainLockedHeight": 123400,
  "version": "1.0.0",
  "timePerBlock": 2500,
  "epoch": 850
}
```

#### GET_BLOCK_HEIGHT
Returns current platform block height:
```json
{
  "height": 123456
}
```

#### GET_BLOCK_TIME
Returns current platform block time:
```json
{
  "time": 1699564800000
}
```

#### GET_CORE_CHAIN_LOCKED_HEIGHT
Returns core chain locked height:
```json
{
  "height": 123400
}
```

#### GET_PLATFORM_VERSION
Returns platform version:
```json
{
  "version": "1.0.0"
}
```

#### GET_PROPOSER_BLOCK_COUNT
Request payload:
```json
{
  "proposerProTxHash": "..."
}
```
Response:
```json
{
  "count": 42
}
```

#### GET_TIME_PER_BLOCK
Returns average time per block in milliseconds:
```json
{
  "timePerBlock": 2500
}
```

#### GET_BLOCK_PROPOSER
Request payload:
```json
{
  "blockHeight": 123456
}
```
Response:
```json
{
  "proposer": "proposerProTxHash..."
}
```

### Wallet Messages

#### GET_ADDRESSES
Request payload (optional):
```json
{
  "accountIndex": 0
}
```
Response:
```json
{
  "walletId": "wallet-uuid",
  "network": "testnet",
  "accounts": [
    {
      "index": 0,
      "address": "yXz...",
      "balance": 100000000
    }
  ],
  "identities": [
    {
      "id": "identity-id",
      "index": 0
    }
  ],
  "addresses": ["yXz...", "yAb..."]
}
```

#### GET_IDENTITY_KEYS
Request payload:
```json
{
  "identityId": "..."
}
```
Response:
```json
{
  "keys": [
    {
      "id": 0,
      "type": "ECDSA_SECP256K1",
      "purpose": "AUTHENTICATION",
      "securityLevel": "HIGH",
      "data": "base64-encoded-public-key"
    }
  ]
}
```

#### SIGN_STATE_TRANSITION
Request payload:
```json
{
  "stateTransition": "base64-encoded-bytes",
  "identityId": "...",
  "keyIndex": 0,
  "keyType": "ECDSA"
}
```
Response:
```json
{
  "signature": "base64-encoded-signature"
}
```

#### CREATE_ASSET_LOCK_PROOF
Request payload:
```json
{
  "amount": 100000000,
  "accountIndex": 0,
  "oneTimePrivateKey": "base64-encoded-key"
}
```
Response:
```json
{
  "type": "instant",
  "instantLock": "base64-encoded-islock",
  "transaction": "base64-encoded-tx",
  "outputIndex": 0
}
```

#### DERIVE_KEY
Request payload:
```json
{
  "derivationPath": "m/44'/5'/0'/0/0",
  "keyType": "ECDSA"
}
```
Response:
```json
{
  "publicKey": "base64-encoded-pubkey",
  "chainCode": "base64-encoded-chaincode"
}
```

### Authentication Messages

#### AUTH_CHALLENGE
Request payload:
```json
{
  "challenge": [/* 32 random bytes */]
}
```
Response should include signed challenge.

#### PING/PONG
Simple connectivity check. PING request should return PONG response.

## Security Requirements

### Pairing Process
1. Display 9-digit pairing code on mobile device
2. User enters code in web application
3. Establish ECDH key exchange
4. Derive session keys for encryption

### Encryption
- All messages after pairing must be encrypted using AES-256-GCM
- Use derived session key from ECDH exchange
- Include nonce to prevent replay attacks

### Authentication Flow
1. Web app sends AUTH_CHALLENGE with 32 random bytes
2. Mobile device signs challenge with its identity key
3. Web app verifies signature
4. Session marked as authenticated

## Status Updates

The Status characteristic should emit JSON updates for:
```json
{
  "connected": true,
  "authenticated": true,
  "network": "testnet",
  "syncStatus": "synced",
  "blockHeight": 123456
}
```

## Implementation Guidelines

### Mobile App Requirements

1. **Bluetooth Permissions**: Request Bluetooth and location permissions
2. **Background Service**: Maintain connection in background
3. **Security**: Store keys in secure enclave/keystore
4. **UI**: Show connection status, pairing code, approve signing requests

### Connection Lifecycle

1. **Discovery**: Advertise service UUID
2. **Connection**: Accept GATT connection
3. **Pairing**: Exchange keys and establish encryption
4. **Authentication**: Verify client identity
5. **Operation**: Handle requests/responses
6. **Disconnection**: Clean up session data

### Error Handling

Standard error codes:
- `AUTH_REQUIRED`: Authentication needed
- `INVALID_REQUEST`: Malformed request
- `NOT_FOUND`: Resource not found
- `INSUFFICIENT_BALANCE`: Not enough funds
- `SIGNING_FAILED`: Failed to sign
- `NETWORK_ERROR`: Network connectivity issue

## Example Implementation (React Native)

```javascript
import BleManager from 'react-native-ble-manager';

class DashBluetoothService {
  async setupService() {
    // Initialize BLE Manager
    await BleManager.start();
    
    // Add service
    await BleManager.addService({
      service: '00000000-dash-platform-bluetooth-service',
      characteristics: [
        {
          uuid: '00000001-dash-platform-command-char',
          properties: ['Write'],
          onWriteRequest: this.handleCommand
        },
        {
          uuid: '00000002-dash-platform-response-char',
          properties: ['Notify']
        },
        {
          uuid: '00000003-dash-platform-status-char',
          properties: ['Read', 'Notify'],
          onReadRequest: this.handleStatusRead
        }
      ]
    });
    
    // Start advertising
    await BleManager.startAdvertising({
      serviceUUIDs: ['00000000-dash-platform-bluetooth-service'],
      localName: 'Dash Wallet'
    });
  }
  
  async handleCommand(data, offset, withoutResponse, callback) {
    try {
      const message = JSON.parse(data.toString('utf8'));
      const response = await this.processMessage(message);
      await this.sendResponse(response);
      callback(BleManager.RESULT_SUCCESS);
    } catch (error) {
      callback(BleManager.RESULT_UNLIKELY_ERROR);
    }
  }
}
```

## Testing

### Test Scenarios
1. Pairing and authentication flow
2. Message chunking for large payloads
3. Disconnection and reconnection
4. Concurrent requests
5. Error conditions
6. Security (replay attacks, invalid signatures)