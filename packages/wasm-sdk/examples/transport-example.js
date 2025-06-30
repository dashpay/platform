// Example of how to use the WASM SDK with JavaScript transport layer

import init, {
  // Serialization functions
  serializeGetIdentityRequest,
  deserializeGetIdentityResponse,
  serializeBroadcastRequest,
  deserializeBroadcastResponse,
  
  // Nonce management
  checkIdentityNonceCache,
  updateIdentityNonceCache,
  
  // State transition creation
  create_identity,
  
  // SDK
  WasmSdkBuilder,
} from '../pkg/wasm_sdk.js';

// Initialize the WASM module
await init();

// Create SDK instance
const sdkBuilder = WasmSdkBuilder.new_testnet();
const sdk = sdkBuilder.build();

// Example: Fetch an identity
async function fetchIdentity(identityId) {
  // 1. Check cache first
  const cachedNonce = checkIdentityNonceCache(identityId);
  if (cachedNonce !== null) {
    console.log('Using cached nonce:', cachedNonce);
  }
  
  // 2. Prepare the request
  const requestBytes = serializeGetIdentityRequest(identityId, true);
  
  // 3. Make the network call (using fetch API)
  const response = await fetch('https://your-dapi-node.com/v0/identities', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/octet-stream',
    },
    body: requestBytes,
  });
  
  // 4. Process the response
  const responseBytes = new Uint8Array(await response.arrayBuffer());
  const identity = deserializeGetIdentityResponse(responseBytes);
  
  return identity;
}

// Example: Create and broadcast an identity
async function createIdentity(assetLockProof, publicKeys) {
  // 1. Create the state transition
  const stateTransitionBytes = create_identity(assetLockProof, publicKeys);
  
  // 2. Prepare broadcast request
  const broadcastRequest = serializeBroadcastRequest(stateTransitionBytes);
  
  // 3. Send to network
  const response = await fetch('https://your-dapi-node.com/v0/broadcast', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/octet-stream',
    },
    body: broadcastRequest,
  });
  
  // 4. Process response
  const responseBytes = new Uint8Array(await response.arrayBuffer());
  const result = deserializeBroadcastResponse(responseBytes);
  
  if (result.success) {
    console.log('Identity created with transaction ID:', result.transactionId);
    
    // 5. Update nonce cache if needed
    if (identity.id) {
      updateIdentityNonceCache(identity.id, 0);
    }
  } else {
    console.error('Failed to create identity:', result.error);
  }
  
  return result;
}

// Example: Custom transport with retries and error handling
class DAPITransport {
  constructor(nodeUrls) {
    this.nodeUrls = nodeUrls;
    this.currentNodeIndex = 0;
  }
  
  async request(endpoint, requestBytes, options = {}) {
    const maxRetries = options.retries || 3;
    let lastError;
    
    for (let retry = 0; retry < maxRetries; retry++) {
      const nodeUrl = this.nodeUrls[this.currentNodeIndex];
      this.currentNodeIndex = (this.currentNodeIndex + 1) % this.nodeUrls.length;
      
      try {
        const response = await fetch(`${nodeUrl}${endpoint}`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/octet-stream',
          },
          body: requestBytes,
          signal: AbortSignal.timeout(options.timeout || 30000),
        });
        
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        return new Uint8Array(await response.arrayBuffer());
      } catch (error) {
        lastError = error;
        console.warn(`Request failed on ${nodeUrl}, trying next node...`, error);
      }
    }
    
    throw lastError;
  }
}

// Usage with custom transport
const transport = new DAPITransport([
  'https://seed-1.testnet.networks.dash.org:1443',
  'https://seed-2.testnet.networks.dash.org:1443',
  'https://seed-3.testnet.networks.dash.org:1443',
]);

async function fetchIdentityWithTransport(identityId) {
  const requestBytes = serializeGetIdentityRequest(identityId, true);
  const responseBytes = await transport.request('/v0/identities', requestBytes);
  return deserializeGetIdentityResponse(responseBytes);
}