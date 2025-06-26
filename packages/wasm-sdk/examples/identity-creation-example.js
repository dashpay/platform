// Example of creating identities with asset lock proofs

import init, {
  // Asset lock proof functions
  AssetLockProof,
  createInstantProofFromParts,
  createChainProofFromParts,
  createOutPoint,
  
  // Identity creation functions
  createIdentity,
  topUpIdentity,
  updateIdentity,
  createBasicIdentity,
  createStandardIdentityKeys,
  validateIdentityPublicKeys,
  IdentityTransitionBuilder,
  
  // State transition functions
  getStateTransitionType,
  calculateStateTransitionId,
  getStateTransitionIdentityId,
  
  // Transport
  serializeBroadcastRequest,
  deserializeBroadcastResponse,
} from '../pkg/wasm_sdk.js';

// Initialize WASM
await init();

// Example 1: Create a basic identity with instant asset lock proof
async function createIdentityWithInstantLock() {
  // Step 1: Create asset lock proof
  const transactionHex = "..."; // Your asset lock transaction
  const outputIndex = 0;
  const instantLockHex = "..."; // The instant lock
  
  const assetLockProof = createInstantProofFromParts(
    transactionHex,
    outputIndex,
    instantLockHex
  );
  
  // Step 2: Generate a public key for the identity
  // In real usage, this would be from a wallet
  const publicKeyData = new Uint8Array(33); // Compressed ECDSA public key
  crypto.getRandomValues(publicKeyData);
  publicKeyData[0] = 0x02; // Ensure valid compressed key prefix
  
  // Step 3: Create the identity
  const identityCreateTransition = createBasicIdentity(
    assetLockProof.toBytes(),
    publicKeyData
  );
  
  // Step 4: Inspect the created transition
  const transitionId = calculateStateTransitionId(identityCreateTransition);
  const identityId = getStateTransitionIdentityId(identityCreateTransition);
  
  console.log('Created identity transition:', {
    transitionId,
    identityId,
  });
  
  return identityCreateTransition;
}

// Example 2: Create identity with multiple keys
async function createIdentityWithMultipleKeys() {
  // Step 1: Create asset lock proof (chain-based this time)
  const coreChainLockedHeight = 850000;
  const txId = "abcd1234..."; // Transaction ID (32 bytes hex)
  const outputIndex = 0;
  
  const assetLockProof = createChainProofFromParts(
    coreChainLockedHeight,
    txId,
    outputIndex
  );
  
  // Step 2: Define multiple public keys
  const publicKeys = [
    {
      id: 0,
      type: "ECDSA_SECP256K1",
      purpose: 0, // AUTHENTICATION
      securityLevel: 0, // MASTER
      readOnly: false,
      data: new Uint8Array(33), // Your master key
    },
    {
      id: 1,
      type: "ECDSA_SECP256K1",
      purpose: 0, // AUTHENTICATION
      securityLevel: 2, // HIGH
      readOnly: false,
      data: new Uint8Array(33), // Your high security key
    },
    {
      id: 2,
      type: "ECDSA_SECP256K1",
      purpose: 3, // TRANSFER
      securityLevel: 1, // CRITICAL
      readOnly: false,
      data: new Uint8Array(33), // Your transfer key
    },
  ];
  
  // Step 3: Validate the keys
  const validation = validateIdentityPublicKeys(publicKeys);
  console.log('Key validation:', validation);
  
  // Step 4: Create the identity
  const identityCreateTransition = createIdentity(
    assetLockProof.toBytes(),
    publicKeys
  );
  
  return identityCreateTransition;
}

// Example 3: Top up an existing identity
async function topUpExistingIdentity(identityId) {
  // Create a new asset lock proof for the top-up
  const assetLockProof = createInstantProofFromParts(
    transactionHex,
    outputIndex,
    instantLockHex
  );
  
  // Create the top-up transition
  const topUpTransition = topUpIdentity(
    identityId,
    assetLockProof.toBytes()
  );
  
  console.log('Created top-up transition for identity:', identityId);
  
  return topUpTransition;
}

// Example 4: Update identity keys
async function updateIdentityKeys(identityId) {
  const newKey = {
    id: 3,
    type: "ECDSA_SECP256K1",
    purpose: 0, // AUTHENTICATION
    securityLevel: 3, // MEDIUM
    readOnly: false,
    data: new Uint8Array(33),
  };
  
  const disableKeyIds = [1]; // Disable key with ID 1
  
  const updateTransition = updateIdentity(
    identityId,
    1, // revision
    0, // nonce
    [newKey], // keys to add
    disableKeyIds, // keys to disable
    null, // public_keys_disabled_at
    0 // signature_public_key_id
  );
  
  return updateTransition;
}

// Example 5: Using the builder pattern
async function createIdentityWithBuilder() {
  const builder = new IdentityTransitionBuilder();
  
  // Add keys one by one
  builder.addPublicKey({
    id: 0,
    type: "ECDSA_SECP256K1",
    purpose: 0,
    securityLevel: 0,
    readOnly: false,
    data: new Uint8Array(33),
  });
  
  // Create asset lock proof
  const assetLockProof = createChainProofFromParts(
    850000,
    "txid...",
    0
  );
  
  // Build the create transition
  const createTransition = builder.buildCreateTransition(
    assetLockProof.toBytes()
  );
  
  return createTransition;
}

// Example 6: Full identity creation and broadcast flow
async function fullIdentityCreationFlow(transport) {
  // Step 1: Get standard key template
  const keyTemplate = createStandardIdentityKeys();
  console.log('Key template:', keyTemplate);
  
  // Step 2: Fill in actual public key data
  const publicKeys = keyTemplate.map((template, index) => ({
    ...template,
    data: generatePublicKey(index), // Your key generation logic
  }));
  
  // Step 3: Create asset lock proof
  const assetLockProof = await createAssetLockTransaction();
  
  // Step 4: Create identity
  const createTransition = createIdentity(
    assetLockProof.toBytes(),
    publicKeys
  );
  
  // Step 5: Get the identity ID (for reference)
  const identityId = getStateTransitionIdentityId(createTransition);
  console.log('New identity ID will be:', identityId);
  
  // Step 6: Broadcast
  const broadcastRequest = serializeBroadcastRequest(createTransition);
  const response = await transport.request('/v0/broadcast', broadcastRequest);
  const result = deserializeBroadcastResponse(response);
  
  if (result.success) {
    console.log('Identity created successfully!');
    console.log('Transaction ID:', result.transactionId);
    
    // Wait for confirmation
    await waitForConfirmation(
      calculateStateTransitionId(createTransition),
      transport
    );
    
    return identityId;
  } else {
    throw new Error(`Failed to create identity: ${result.error}`);
  }
}

// Helper functions
function generatePublicKey(index) {
  // In real usage, derive from HD wallet
  const key = new Uint8Array(33);
  crypto.getRandomValues(key);
  key[0] = 0x02; // Compressed key prefix
  return key;
}

async function createAssetLockTransaction() {
  // This would interact with a Dash wallet to create the transaction
  // For now, return a mock proof
  return createChainProofFromParts(850000, "mock_tx_id", 0);
}

async function waitForConfirmation(transitionHash, transport) {
  // Implementation would poll for confirmation
  console.log('Waiting for confirmation of:', transitionHash);
}

// Run examples
(async () => {
  try {
    // Example 1: Basic identity
    const basicIdentity = await createIdentityWithInstantLock();
    console.log('Basic identity created');
    
    // Example 2: Multi-key identity
    const multiKeyIdentity = await createIdentityWithMultipleKeys();
    console.log('Multi-key identity created');
    
    // Example 3: Full flow with transport
    const transport = new DAPITransport([...]);
    const identityId = await fullIdentityCreationFlow(transport);
    console.log('Full identity creation completed:', identityId);
    
  } catch (error) {
    console.error('Error:', error);
  }
})();