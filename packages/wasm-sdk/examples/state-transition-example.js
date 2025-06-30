// Example of using the state transition serialization interface

import init, {
  // State transition creation
  create_identity,
  create_data_contract,
  create_document_batch_transition,
  
  // State transition serialization interface
  deserializeStateTransition,
  getStateTransitionType,
  calculateStateTransitionId,
  validateStateTransitionStructure,
  isIdentitySignedStateTransition,
  getStateTransitionIdentityId,
  getStateTransitionSignableBytes,
  
  // Transport serialization
  serializeBroadcastRequest,
  deserializeBroadcastResponse,
  prepareStateTransitionForBroadcast,
  getRequiredSignaturesForStateTransition,
  
  // Types
  StateTransitionTypeWasm,
} from '../pkg/wasm_sdk.js';

// Initialize WASM
await init();

// Example: Create and serialize an identity create transition
async function createIdentityExample() {
  // Create asset lock proof (from previous example)
  const assetLockProof = createInstantAssetLockProof(
    transactionHex,
    outputIndex,
    instantLockHex
  );
  
  // Create public keys
  const publicKeys = [
    {
      id: 0,
      purpose: 0, // Authentication
      securityLevel: 0, // Master
      keyType: 0, // ECDSA
      readOnly: false,
      data: publicKeyBytes,
    }
  ];
  
  // Create identity state transition
  const stBytes = create_identity(assetLockProof.toBytes(), publicKeys);
  
  // Get information about the state transition
  const stType = getStateTransitionType(stBytes);
  console.log('State transition type:', stType); // Should be IdentityCreate
  
  const stId = calculateStateTransitionId(stBytes);
  console.log('State transition ID:', stId);
  
  const validation = validateStateTransitionStructure(stBytes);
  console.log('Validation result:', validation);
  
  const requiresIdentitySig = isIdentitySignedStateTransition(stBytes);
  console.log('Requires identity signature:', requiresIdentitySig); // false for IdentityCreate
  
  // Prepare for broadcast
  const broadcastInfo = prepareStateTransitionForBroadcast(stBytes);
  console.log('Ready for broadcast:', broadcastInfo);
  
  return stBytes;
}

// Example: Deserialize and inspect a state transition
async function inspectStateTransition(stBytes) {
  // Deserialize to inspect
  const stObject = deserializeStateTransition(stBytes);
  console.log('Deserialized state transition:', stObject);
  
  // Get identity ID if applicable
  const identityId = getStateTransitionIdentityId(stBytes);
  if (identityId) {
    console.log('Identity ID:', identityId);
  }
  
  // Check signature requirements
  const sigRequirements = getRequiredSignaturesForStateTransition(stBytes);
  console.log('Signature requirements:', sigRequirements);
  
  // Get signable bytes for signing
  if (sigRequirements.identitySignature) {
    const signableBytes = getStateTransitionSignableBytes(stBytes);
    // Sign with identity key...
  }
}

// Example: Broadcast a state transition
async function broadcastStateTransition(stBytes, transport) {
  // Serialize for network transport
  const broadcastRequest = serializeBroadcastRequest(stBytes);
  
  // Send via transport layer
  const response = await transport.request('/v0/broadcast', broadcastRequest);
  
  // Process response
  const result = deserializeBroadcastResponse(response);
  
  if (result.success) {
    console.log('State transition broadcasted:', result.transactionId);
    
    // Wait for confirmation
    const hash = calculateStateTransitionId(stBytes);
    await waitForStateTransition(hash, transport);
  } else {
    console.error('Broadcast failed:', result.error);
  }
}

// Example: Create different types of state transitions
async function createVariousStateTransitions() {
  // 1. Data Contract Create
  const contractDefinition = {
    documents: {
      user: {
        type: "object",
        properties: {
          username: { type: "string" },
          email: { type: "string" }
        },
        required: ["username"],
        additionalProperties: false
      }
    }
  };
  
  const contractCreateBytes = create_data_contract(
    ownerId,
    contractDefinition,
    entropy
  );
  
  // 2. Document Batch Transition
  const documents = [
    {
      action: "create",
      dataContractId: "...",
      type: "user",
      data: {
        username: "alice",
        email: "alice@example.com"
      }
    }
  ];
  
  const batchBytes = create_document_batch_transition(
    ownerId,
    documents,
    nonce
  );
  
  // Inspect each one
  for (const [name, bytes] of [
    ['Contract Create', contractCreateBytes],
    ['Document Batch', batchBytes]
  ]) {
    console.log(`\n${name}:`);
    const type = getStateTransitionType(bytes);
    const id = calculateStateTransitionId(bytes);
    const needsSig = isIdentitySignedStateTransition(bytes);
    
    console.log(`- Type: ${StateTransitionTypeWasm[type]}`);
    console.log(`- ID: ${id}`);
    console.log(`- Needs identity signature: ${needsSig}`);
  }
}

// Example: Handle state transition results
async function waitForStateTransition(stHash, transport) {
  const waitRequest = serializeWaitForStateTransitionRequest(stHash, true);
  
  // Poll for result
  let executed = false;
  let attempts = 0;
  
  while (!executed && attempts < 30) {
    const response = await transport.request(
      '/v0/state-transition-result',
      waitRequest
    );
    
    const result = deserializeWaitForStateTransitionResponse(response);
    
    if (result.executed) {
      console.log('State transition executed at block:', result.blockHeight);
      executed = true;
    } else if (result.error) {
      throw new Error(`State transition failed: ${result.error}`);
    }
    
    attempts++;
    if (!executed) {
      await new Promise(resolve => setTimeout(resolve, 2000)); // Wait 2 seconds
    }
  }
  
  if (!executed) {
    throw new Error('State transition timed out');
  }
}

// Run examples
(async () => {
  // Create transport instance
  const transport = new DAPITransport([...]);
  
  // Create identity
  const identitySTBytes = await createIdentityExample();
  await inspectStateTransition(identitySTBytes);
  await broadcastStateTransition(identitySTBytes, transport);
  
  // Create other state transitions
  await createVariousStateTransitions();
})();