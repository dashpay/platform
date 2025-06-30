# Dash Platform WASM SDK Usage Examples

This document provides comprehensive examples for using the Dash Platform WASM SDK in real-world applications.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Identity Management](#identity-management)
3. [Data Contracts](#data-contracts)
4. [Document Operations](#document-operations)
5. [Token Management](#token-management)
6. [Advanced Patterns](#advanced-patterns)
7. [Error Handling](#error-handling)
8. [Performance Optimization](#performance-optimization)

## Getting Started

### Basic Setup

```javascript
import { start, WasmSdk } from 'dash-wasm-sdk';

// Initialize WASM module (required once per application)
await start();

// Create SDK instances for different networks
const sdk = new WasmSdk('testnet');
const mainnetSdk = new WasmSdk('mainnet');
const devnetSdk = new WasmSdk('devnet');

// Check if SDK is ready
if (sdk.isReady()) {
  console.log('SDK initialized and ready to use');
}
```

### TypeScript Setup

```typescript
import { 
  start, 
  WasmSdk, 
  IdentityBalance,
  FetchResponse,
  ErrorCategory,
  WasmError
} from 'dash-wasm-sdk';

async function initializeSdk(): Promise<WasmSdk> {
  await start();
  return new WasmSdk('testnet');
}

// Type-safe error handling
function handleError(error: unknown): void {
  if (error instanceof WasmError) {
    console.error(`${error.category}: ${error.message}`);
  } else {
    console.error('Unknown error:', error);
  }
}
```

## Identity Management

### Creating a New Identity

```javascript
import { 
  AssetLockProof,
  createIdentityWithAssetLock,
  broadcastStateTransition,
  WasmSigner
} from 'dash-wasm-sdk';

async function createIdentity(
  transactionHex: string,
  instantLockHex: string,
  privateKeyHex: string
) {
  // Parse transaction and instant lock
  const transactionBytes = hexToBytes(transactionHex);
  const instantLockBytes = hexToBytes(instantLockHex);
  const privateKeyBytes = hexToBytes(privateKeyHex);
  
  // Create asset lock proof
  const assetLockProof = AssetLockProof.createInstant(
    transactionBytes,
    0, // output index
    instantLockBytes
  );
  
  // Calculate public key from private key
  const publicKeyBytes = await derivePublicKey(privateKeyBytes);
  
  // Define identity keys
  const publicKeys = [{
    id: 0,
    type: 0, // ECDSA_SECP256K1
    purpose: 0, // AUTHENTICATION
    securityLevel: 0, // MASTER
    data: publicKeyBytes,
    readOnly: false
  }];
  
  // Create identity state transition
  const stateTransition = await createIdentityWithAssetLock(
    assetLockProof,
    publicKeys
  );
  
  // Set up signer
  const signer = new WasmSigner();
  signer.addPrivateKey(0, privateKeyBytes, 'ECDSA_SECP256K1', 0);
  
  // Sign and broadcast
  const signedTransition = await signStateTransition(stateTransition, signer);
  const result = await broadcastStateTransition(sdk, signedTransition);
  
  if (result.success) {
    console.log('Identity created successfully');
    return parseIdentityId(stateTransition);
  } else {
    throw new Error(`Failed to create identity: ${result.error}`);
  }
}
```

### Managing Identity Keys

```javascript
async function addIdentityKey(
  identityId: string,
  currentRevision: number,
  newPublicKey: Uint8Array,
  signerKeyId: number
) {
  // Fetch current identity to get nonce
  const identity = await fetchIdentity(sdk, identityId);
  
  // Create new key definition
  const newKey = {
    id: Math.max(...identity.publicKeys.map(k => k.id)) + 1,
    type: 0,
    purpose: 0,
    securityLevel: 1, // HIGH
    data: newPublicKey,
    readOnly: false
  };
  
  // Create update transition
  const updateTransition = updateIdentity(
    identityId,
    BigInt(currentRevision + 1),
    [newKey], // keys to add
    [], // keys to disable
    undefined, // publicKeysDisabledAt
    signerKeyId
  );
  
  // Sign and broadcast
  const result = await broadcastStateTransition(sdk, updateTransition);
  return result.success;
}

async function disableIdentityKey(
  identityId: string,
  currentRevision: number,
  keyIdToDisable: number,
  signerKeyId: number
) {
  const updateTransition = updateIdentity(
    identityId,
    BigInt(currentRevision + 1),
    [], // no keys to add
    [keyIdToDisable], // keys to disable
    BigInt(Date.now()), // disable timestamp
    signerKeyId
  );
  
  const result = await broadcastStateTransition(sdk, updateTransition);
  return result.success;
}
```

### Identity Balance Management

```javascript
import {
  fetchIdentityBalance,
  checkIdentityBalance,
  estimateCreditsNeeded,
  monitorIdentityBalance
} from 'dash-wasm-sdk';

async function manageIdentityBalance(identityId: string) {
  // Check current balance
  const balance = await fetchIdentityBalance(sdk, identityId);
  console.log(`Current balance: ${balance.total} credits`);
  console.log(`Confirmed: ${balance.confirmed}`);
  console.log(`Unconfirmed: ${balance.unconfirmed}`);
  
  // Estimate credits for operations
  const operations = [
    { type: 'document_create', size: 1024 },
    { type: 'document_update', size: 512 },
    { type: 'identity_update', size: 0 },
    { type: 'contract_create', size: 4096 }
  ];
  
  let totalCreditsNeeded = 0;
  for (const op of operations) {
    const credits = estimateCreditsNeeded(op.type, op.size);
    console.log(`${op.type}: ${credits} credits`);
    totalCreditsNeeded += credits;
  }
  
  // Check if we have enough balance
  const hasEnough = await checkIdentityBalance(
    sdk,
    identityId,
    totalCreditsNeeded,
    true // include unconfirmed
  );
  
  if (!hasEnough) {
    console.warn('Insufficient balance! Need to top up.');
  }
  
  // Monitor balance changes
  const monitor = await monitorIdentityBalance(
    sdk,
    identityId,
    (newBalance) => {
      const change = newBalance.total - balance.total;
      if (change !== 0) {
        console.log(`Balance changed by ${change} credits`);
        console.log(`New total: ${newBalance.total}`);
      }
    },
    10000 // check every 10 seconds
  );
  
  // Stop monitoring after 5 minutes
  setTimeout(() => {
    monitor.active = false;
    console.log('Stopped balance monitoring');
  }, 5 * 60 * 1000);
}
```

### Top Up Identity

```javascript
async function topUpIdentity(
  identityId: string,
  assetLockTransaction: Uint8Array,
  assetLockProof: Uint8Array
) {
  // Create asset lock proof
  const proof = AssetLockProof.createInstant(
    assetLockTransaction,
    0,
    assetLockProof
  );
  
  // Create top-up transition
  const topUpTransition = topupIdentity(identityId, proof.toBytes());
  
  // Broadcast
  const result = await broadcastStateTransition(sdk, topUpTransition);
  
  if (result.success) {
    // Check new balance
    const newBalance = await fetchIdentityBalance(sdk, identityId);
    console.log(`Top-up successful! New balance: ${newBalance.total}`);
  }
  
  return result;
}
```

## Data Contracts

### Creating a Social Media Contract

```javascript
import { createDataContract, incrementIdentityNonce } from 'dash-wasm-sdk';

async function createSocialMediaContract(ownerId: string, signerKeyId: number) {
  // Get current nonce
  const nonceResult = await getIdentityNonce(sdk, ownerId, false);
  
  // Define contract schema
  const contractDefinition = {
    protocolVersion: 1,
    documents: {
      profile: {
        type: 'object',
        properties: {
          username: {
            type: 'string',
            pattern: '^[a-zA-Z0-9_]{3,20}$',
            description: 'Unique username'
          },
          displayName: {
            type: 'string',
            maxLength: 50
          },
          bio: {
            type: 'string',
            maxLength: 280
          },
          avatarUrl: {
            type: 'string',
            format: 'uri',
            maxLength: 255
          },
          createdAt: {
            type: 'integer',
            minimum: 0
          }
        },
        required: ['username', 'createdAt'],
        additionalProperties: false,
        indices: [
          {
            name: 'username',
            properties: [{ username: 'asc' }],
            unique: true
          },
          {
            name: 'createdAt',
            properties: [{ createdAt: 'desc' }]
          }
        ]
      },
      post: {
        type: 'object',
        properties: {
          authorId: {
            type: 'string',
            contentMediaType: 'application/x.dash.dpp.identifier'
          },
          content: {
            type: 'string',
            maxLength: 280
          },
          tags: {
            type: 'array',
            items: {
              type: 'string',
              pattern: '^#[a-zA-Z0-9]{1,20}$'
            },
            maxItems: 10
          },
          likes: {
            type: 'integer',
            minimum: 0
          },
          timestamp: {
            type: 'integer'
          },
          replyTo: {
            type: 'string',
            contentMediaType: 'application/x.dash.dpp.identifier',
            description: 'ID of post being replied to'
          }
        },
        required: ['authorId', 'content', 'timestamp'],
        additionalProperties: false,
        indices: [
          {
            name: 'authorTimestamp',
            properties: [
              { authorId: 'asc' },
              { timestamp: 'desc' }
            ]
          },
          {
            name: 'timestamp',
            properties: [{ timestamp: 'desc' }]
          },
          {
            name: 'tags',
            properties: [{ tags: 'asc' }]
          }
        ]
      },
      follow: {
        type: 'object',
        properties: {
          followerId: {
            type: 'string',
            contentMediaType: 'application/x.dash.dpp.identifier'
          },
          followingId: {
            type: 'string',
            contentMediaType: 'application/x.dash.dpp.identifier'
          },
          createdAt: {
            type: 'integer'
          }
        },
        required: ['followerId', 'followingId', 'createdAt'],
        additionalProperties: false,
        indices: [
          {
            name: 'followerFollowing',
            properties: [
              { followerId: 'asc' },
              { followingId: 'asc' }
            ],
            unique: true
          },
          {
            name: 'following',
            properties: [
              { followingId: 'asc' },
              { createdAt: 'desc' }
            ]
          }
        ]
      }
    }
  };
  
  // Create contract state transition
  const stateTransition = createDataContract(
    ownerId,
    contractDefinition,
    nonceResult.nonce,
    signerKeyId
  );
  
  // Increment nonce for next operation
  await incrementIdentityNonce(sdk, ownerId);
  
  // Broadcast
  const result = await broadcastStateTransition(sdk, stateTransition);
  
  if (result.success) {
    const contractId = parseContractId(stateTransition);
    console.log(`Contract created with ID: ${contractId}`);
    return contractId;
  }
  
  throw new Error(`Failed to create contract: ${result.error}`);
}
```

### Updating a Data Contract

```javascript
async function addDocumentTypeToContract(
  contractId: string,
  ownerId: string,
  signerKeyId: number
) {
  // Fetch current contract
  const contract = await fetchDataContract(sdk, contractId);
  
  // Get contract nonce
  const nonceResult = await getIdentityContractNonce(
    sdk,
    ownerId,
    contractId,
    false
  );
  
  // Add new document type
  const updatedDefinition = {
    ...contract.definition,
    documents: {
      ...contract.definition.documents,
      directMessage: {
        type: 'object',
        properties: {
          fromId: {
            type: 'string',
            contentMediaType: 'application/x.dash.dpp.identifier'
          },
          toId: {
            type: 'string',
            contentMediaType: 'application/x.dash.dpp.identifier'
          },
          encryptedContent: {
            type: 'string',
            contentMediaType: 'application/base64'
          },
          timestamp: {
            type: 'integer'
          }
        },
        required: ['fromId', 'toId', 'encryptedContent', 'timestamp'],
        additionalProperties: false,
        indices: [
          {
            name: 'conversation',
            properties: [
              { fromId: 'asc' },
              { toId: 'asc' },
              { timestamp: 'desc' }
            ]
          }
        ]
      }
    }
  };
  
  // Create update transition
  const updateTransition = updateDataContract(
    contractId,
    ownerId,
    updatedDefinition,
    nonceResult.nonce,
    signerKeyId
  );
  
  // Broadcast
  const result = await broadcastStateTransition(sdk, updateTransition);
  return result.success;
}
```

## Document Operations

### Creating Documents

```javascript
import { DocumentBatchBuilder } from 'dash-wasm-sdk';

async function createUserProfile(
  contractId: string,
  ownerId: string,
  profileData: {
    username: string;
    displayName: string;
    bio: string;
    avatarUrl?: string;
  }
) {
  const builder = new DocumentBatchBuilder(ownerId);
  
  // Create profile document
  builder.addCreateDocument(
    contractId,
    'profile',
    generateDocumentId(), // Generate unique ID
    {
      ...profileData,
      createdAt: Date.now()
    }
  );
  
  // Build and broadcast
  const stateTransition = builder.build(0); // signer key ID
  const result = await broadcastStateTransition(sdk, stateTransition);
  
  return result.success;
}

async function createPost(
  contractId: string,
  authorId: string,
  content: string,
  tags: string[] = [],
  replyTo?: string
) {
  const builder = new DocumentBatchBuilder(authorId);
  
  const postData = {
    authorId,
    content,
    tags: tags.filter(tag => tag.startsWith('#')),
    likes: 0,
    timestamp: Date.now()
  };
  
  if (replyTo) {
    postData.replyTo = replyTo;
  }
  
  builder.addCreateDocument(
    contractId,
    'post',
    generateDocumentId(),
    postData
  );
  
  const stateTransition = builder.build(0);
  const result = await broadcastStateTransition(sdk, stateTransition);
  
  return result.success;
}
```

### Querying Documents

```javascript
import { DocumentQuery, fetchDocuments } from 'dash-wasm-sdk';

async function getUserPosts(contractId: string, userId: string, limit = 20) {
  const query = new DocumentQuery(contractId, 'post');
  query.addWhereClause('authorId', '=', userId);
  query.addOrderBy('timestamp', false); // descending
  query.setLimit(limit);
  
  const posts = await fetchDocuments(
    sdk,
    contractId,
    'post',
    query.getWhereClauses(),
    { orderBy: query.getOrderByClauses(), limit }
  );
  
  return posts;
}

async function searchPostsByTag(contractId: string, tag: string) {
  const query = new DocumentQuery(contractId, 'post');
  query.addWhereClause('tags', 'contains', tag);
  query.addOrderBy('timestamp', false);
  query.setLimit(50);
  
  const posts = await fetchDocuments(
    sdk,
    contractId,
    'post',
    query.getWhereClauses(),
    { orderBy: query.getOrderByClauses(), limit: 50 }
  );
  
  return posts;
}

async function getFollowers(contractId: string, userId: string) {
  const query = new DocumentQuery(contractId, 'follow');
  query.addWhereClause('followingId', '=', userId);
  query.addOrderBy('createdAt', false);
  
  const followers = await fetchDocuments(
    sdk,
    contractId,
    'follow',
    query.getWhereClauses()
  );
  
  // Fetch follower profiles
  const followerProfiles = await Promise.all(
    followers.map(async (follow) => {
      const profileQuery = new DocumentQuery(contractId, 'profile');
      profileQuery.addWhereClause('$ownerId', '=', follow.followerId);
      
      const profiles = await fetchDocuments(
        sdk,
        contractId,
        'profile',
        profileQuery.getWhereClauses()
      );
      
      return profiles[0];
    })
  );
  
  return followerProfiles.filter(Boolean);
}
```

### Updating Documents

```javascript
async function updateProfile(
  contractId: string,
  ownerId: string,
  documentId: string,
  currentRevision: number,
  updates: Partial<{
    displayName: string;
    bio: string;
    avatarUrl: string;
  }>
) {
  // Fetch current document
  const currentDoc = await fetchDocument(sdk, contractId, 'profile', documentId);
  
  // Merge updates
  const updatedData = {
    ...currentDoc.data,
    ...updates,
    updatedAt: Date.now()
  };
  
  // Create update
  const builder = new DocumentBatchBuilder(ownerId);
  builder.addReplaceDocument(
    contractId,
    'profile',
    documentId,
    currentRevision + 1,
    updatedData
  );
  
  const stateTransition = builder.build(0);
  const result = await broadcastStateTransition(sdk, stateTransition);
  
  return result.success;
}

async function incrementPostLikes(
  contractId: string,
  postOwnerId: string,
  postId: string,
  currentRevision: number
) {
  const post = await fetchDocument(sdk, contractId, 'post', postId);
  
  const builder = new DocumentBatchBuilder(postOwnerId);
  builder.addReplaceDocument(
    contractId,
    'post',
    postId,
    currentRevision + 1,
    {
      ...post.data,
      likes: (post.data.likes || 0) + 1
    }
  );
  
  const stateTransition = builder.build(0);
  return await broadcastStateTransition(sdk, stateTransition);
}
```

### Batch Document Operations

```javascript
async function performBatchOperations(
  contractId: string,
  ownerId: string,
  operations: Array<{
    type: 'create' | 'update' | 'delete';
    documentType: string;
    documentId?: string;
    data?: any;
    revision?: number;
  }>
) {
  const builder = new DocumentBatchBuilder(ownerId);
  
  for (const op of operations) {
    switch (op.type) {
      case 'create':
        builder.addCreateDocument(
          contractId,
          op.documentType,
          op.documentId || generateDocumentId(),
          op.data
        );
        break;
        
      case 'update':
        if (!op.documentId || !op.revision) {
          throw new Error('Update requires documentId and revision');
        }
        builder.addReplaceDocument(
          contractId,
          op.documentType,
          op.documentId,
          op.revision,
          op.data
        );
        break;
        
      case 'delete':
        if (!op.documentId) {
          throw new Error('Delete requires documentId');
        }
        builder.addDeleteDocument(
          contractId,
          op.documentType,
          op.documentId
        );
        break;
    }
  }
  
  const stateTransition = builder.build(0);
  const result = await broadcastStateTransition(sdk, stateTransition);
  
  return {
    success: result.success,
    operationCount: operations.length,
    error: result.error
  };
}
```

## Token Management

### Creating and Managing Tokens

```javascript
import {
  createTokenIssuance,
  mintTokens,
  transferTokens,
  getTokenBalance,
  getTokenInfo
} from 'dash-wasm-sdk';

async function createGameToken(
  contractId: string,
  ownerId: string,
  tokenPosition: number,
  initialSupply: number
) {
  // Get nonce
  const nonceResult = await getIdentityContractNonce(
    sdk,
    ownerId,
    contractId,
    false
  );
  
  // Create token issuance
  const issuanceTransition = createTokenIssuance(
    contractId,
    tokenPosition,
    initialSupply,
    nonceResult.nonce.toNumber(),
    0 // signer key ID
  );
  
  // Broadcast
  const result = await broadcastStateTransition(sdk, issuanceTransition);
  
  if (result.success) {
    // Get token info
    const tokenId = `${contractId}-${tokenPosition}`;
    const info = await getTokenInfo(sdk, tokenId);
    console.log('Token created:', info);
  }
  
  return result;
}

async function rewardPlayer(
  tokenId: string,
  fromIdentityId: string,
  toIdentityId: string,
  amount: number
) {
  // Check sender balance
  const senderBalance = await getTokenBalance(sdk, tokenId, fromIdentityId);
  
  if (senderBalance.balance < amount) {
    throw new Error('Insufficient token balance');
  }
  
  if (senderBalance.frozen) {
    throw new Error('Sender tokens are frozen');
  }
  
  // Transfer tokens
  const result = await transferTokens(
    sdk,
    tokenId,
    amount,
    fromIdentityId,
    toIdentityId
  );
  
  if (result.success) {
    // Check new balances
    const newSenderBalance = await getTokenBalance(sdk, tokenId, fromIdentityId);
    const recipientBalance = await getTokenBalance(sdk, tokenId, toIdentityId);
    
    console.log(`Transfer complete!`);
    console.log(`Sender balance: ${newSenderBalance.balance}`);
    console.log(`Recipient balance: ${recipientBalance.balance}`);
  }
  
  return result;
}
```

### Token Economy Example

```javascript
async function implementTokenEconomy(contractId: string, adminId: string) {
  // Define token types
  const tokens = {
    governance: { position: 0, supply: 1000000 },
    rewards: { position: 1, supply: 10000000 },
    premium: { position: 2, supply: 100000 }
  };
  
  // Create tokens
  for (const [name, config] of Object.entries(tokens)) {
    await createGameToken(
      contractId,
      adminId,
      config.position,
      config.supply
    );
    console.log(`Created ${name} token`);
  }
  
  // Distribute initial tokens
  const recipients = [
    { id: 'identity1', governance: 100, rewards: 1000 },
    { id: 'identity2', governance: 50, rewards: 500 },
    { id: 'identity3', governance: 25, rewards: 250 }
  ];
  
  for (const recipient of recipients) {
    // Transfer governance tokens
    await transferTokens(
      sdk,
      `${contractId}-0`,
      recipient.governance,
      adminId,
      recipient.id
    );
    
    // Transfer reward tokens
    await transferTokens(
      sdk,
      `${contractId}-1`,
      recipient.rewards,
      adminId,
      recipient.id
    );
  }
  
  // Set up reward system
  async function rewardUserAction(userId: string, action: string) {
    const rewardAmounts = {
      post_created: 10,
      post_liked: 1,
      profile_completed: 50,
      daily_login: 5
    };
    
    const amount = rewardAmounts[action] || 0;
    if (amount > 0) {
      await transferTokens(
        sdk,
        `${contractId}-1`, // rewards token
        amount,
        adminId,
        userId
      );
      console.log(`Rewarded ${userId} with ${amount} tokens for ${action}`);
    }
  }
  
  return { tokens, rewardUserAction };
}
```

## Advanced Patterns

### Retry and Error Recovery

```javascript
import { RequestSettings, executeWithRetry } from 'dash-wasm-sdk';

async function robustFetch<T>(
  operation: () => Promise<T>,
  maxAttempts = 5
): Promise<T> {
  const settings = new RequestSettings();
  settings.setMaxRetries(maxAttempts);
  settings.setInitialRetryDelay(1000);
  settings.setBackoffMultiplier(2);
  settings.setUseExponentialBackoff(true);
  settings.setRetryOnTimeout(true);
  settings.setRetryOnNetworkError(true);
  
  try {
    return await executeWithRetry(operation, settings);
  } catch (error) {
    console.error(`Failed after ${maxAttempts} attempts:`, error);
    throw error;
  }
}

// Usage
const identity = await robustFetch(() => 
  fetchIdentity(sdk, 'identity-id')
);
```

### Caching Strategy

```javascript
import { WasmCacheManager } from 'dash-wasm-sdk';

class CachedSDK {
  private sdk: WasmSdk;
  private cache: WasmCacheManager;
  
  constructor(network: string) {
    this.sdk = new WasmSdk(network);
    this.cache = new WasmCacheManager();
    
    // Configure cache TTLs
    this.cache.setTTLs(
      3600,  // contracts: 1 hour
      1800,  // identities: 30 minutes
      300,   // documents: 5 minutes
      600,   // tokens: 10 minutes
      7200,  // quorum keys: 2 hours
      60     // metadata: 1 minute
    );
  }
  
  async fetchIdentity(id: string): Promise<any> {
    // Check cache first
    const cached = this.cache.getCachedIdentity(id);
    if (cached) {
      return JSON.parse(new TextDecoder().decode(cached));
    }
    
    // Fetch from network
    const identity = await fetchIdentity(this.sdk, id);
    
    // Cache the result
    this.cache.cacheIdentity(
      id,
      new TextEncoder().encode(JSON.stringify(identity))
    );
    
    return identity;
  }
  
  async fetchDataContract(id: string): Promise<any> {
    const cached = this.cache.getCachedContract(id);
    if (cached) {
      return JSON.parse(new TextDecoder().decode(cached));
    }
    
    const contract = await fetchDataContract(this.sdk, id);
    this.cache.cacheContract(
      id,
      new TextEncoder().encode(JSON.stringify(contract))
    );
    
    return contract;
  }
  
  clearCache(): void {
    this.cache.clearAll();
  }
  
  getCacheStats() {
    return this.cache.getStats();
  }
}
```

### State Synchronization

```javascript
class PlatformStateSync {
  private sdk: WasmSdk;
  private subscriptions: Map<string, Function>;
  private pollInterval: number;
  
  constructor(sdk: WasmSdk, pollInterval = 5000) {
    this.sdk = sdk;
    this.subscriptions = new Map();
    this.pollInterval = pollInterval;
  }
  
  subscribeToIdentity(
    identityId: string,
    callback: (identity: any) => void
  ): () => void {
    let lastRevision = -1;
    
    const checkForUpdates = async () => {
      try {
        const identity = await fetchIdentity(this.sdk, identityId);
        if (identity.revision > lastRevision) {
          lastRevision = identity.revision;
          callback(identity);
        }
      } catch (error) {
        console.error('Failed to fetch identity:', error);
      }
    };
    
    // Initial fetch
    checkForUpdates();
    
    // Set up polling
    const intervalId = setInterval(checkForUpdates, this.pollInterval);
    const unsubscribe = () => {
      clearInterval(intervalId);
      this.subscriptions.delete(identityId);
    };
    
    this.subscriptions.set(identityId, unsubscribe);
    return unsubscribe;
  }
  
  subscribeToDocuments(
    contractId: string,
    documentType: string,
    query: DocumentQuery,
    callback: (documents: any[]) => void
  ): () => void {
    let lastCheck = Date.now();
    
    const checkForUpdates = async () => {
      try {
        // Add time-based filter
        const timeQuery = query.clone();
        timeQuery.addWhereClause('updatedAt', '>', lastCheck);
        
        const documents = await fetchDocuments(
          this.sdk,
          contractId,
          documentType,
          timeQuery.getWhereClauses()
        );
        
        if (documents.length > 0) {
          lastCheck = Date.now();
          callback(documents);
        }
      } catch (error) {
        console.error('Failed to fetch documents:', error);
      }
    };
    
    const intervalId = setInterval(checkForUpdates, this.pollInterval);
    const key = `${contractId}-${documentType}`;
    
    const unsubscribe = () => {
      clearInterval(intervalId);
      this.subscriptions.delete(key);
    };
    
    this.subscriptions.set(key, unsubscribe);
    return unsubscribe;
  }
  
  unsubscribeAll(): void {
    for (const unsubscribe of this.subscriptions.values()) {
      unsubscribe();
    }
    this.subscriptions.clear();
  }
}
```

## Error Handling

### Comprehensive Error Handling

```javascript
import { WasmError, ErrorCategory } from 'dash-wasm-sdk';

class ErrorHandler {
  static async handle<T>(
    operation: () => Promise<T>,
    context: string
  ): Promise<T | null> {
    try {
      return await operation();
    } catch (error) {
      return this.processError(error, context);
    }
  }
  
  private static processError(error: unknown, context: string): null {
    if (error instanceof WasmError) {
      switch (error.category) {
        case ErrorCategory.Network:
          console.error(`Network error in ${context}:`, error.message);
          this.notifyUser('Network connection issue. Please try again.');
          break;
          
        case ErrorCategory.Validation:
          console.error(`Validation error in ${context}:`, error.message);
          this.notifyUser('Invalid data provided. Please check your input.');
          break;
          
        case ErrorCategory.ProofVerification:
          console.error(`Proof verification failed in ${context}:`, error.message);
          this.notifyUser('Data verification failed. This might indicate tampering.');
          break;
          
        case ErrorCategory.StateTransition:
          console.error(`State transition error in ${context}:`, error.message);
          this.notifyUser('Transaction failed. Please check your balance.');
          break;
          
        case ErrorCategory.Identity:
          console.error(`Identity error in ${context}:`, error.message);
          this.notifyUser('Identity operation failed.');
          break;
          
        case ErrorCategory.Document:
          console.error(`Document error in ${context}:`, error.message);
          this.notifyUser('Document operation failed.');
          break;
          
        case ErrorCategory.Contract:
          console.error(`Contract error in ${context}:`, error.message);
          this.notifyUser('Contract operation failed.');
          break;
          
        default:
          console.error(`Unknown error in ${context}:`, error.message);
          this.notifyUser('An unexpected error occurred.');
      }
    } else {
      console.error(`Unexpected error in ${context}:`, error);
      this.notifyUser('An unexpected error occurred.');
    }
    
    return null;
  }
  
  private static notifyUser(message: string): void {
    // Implement your notification system
    console.log(`USER NOTIFICATION: ${message}`);
  }
}

// Usage
const identity = await ErrorHandler.handle(
  () => fetchIdentity(sdk, 'identity-id'),
  'fetchIdentity'
);

if (identity) {
  console.log('Identity fetched successfully');
}
```

## Performance Optimization

### Batch Operations

```javascript
import { fetchBatchUnproved } from 'dash-wasm-sdk';

async function fetchMultipleIdentities(identityIds: string[]) {
  // Create batch requests
  const requests = identityIds.map(id => ({
    type: 'identity' as const,
    id
  }));
  
  // Fetch all at once
  const results = await fetchBatchUnproved(sdk, requests);
  
  // Map results back to IDs
  const identitiesMap = new Map();
  identityIds.forEach((id, index) => {
    identitiesMap.set(id, results[index]);
  });
  
  return identitiesMap;
}

async function prefetchUserData(userId: string, contractId: string) {
  // Parallel fetching
  const [identity, profile, posts, followers] = await Promise.all([
    fetchIdentity(sdk, userId),
    fetchDocuments(sdk, contractId, 'profile', { $ownerId: userId }),
    fetchDocuments(sdk, contractId, 'post', { authorId: userId }, { limit: 10 }),
    fetchDocuments(sdk, contractId, 'follow', { followingId: userId })
  ]);
  
  return {
    identity,
    profile: profile[0],
    recentPosts: posts,
    followerCount: followers.length
  };
}
```

### Lazy Loading

```javascript
class LazyDataLoader {
  private cache: Map<string, Promise<any>>;
  
  constructor() {
    this.cache = new Map();
  }
  
  async getIdentity(id: string): Promise<any> {
    const key = `identity:${id}`;
    
    if (!this.cache.has(key)) {
      this.cache.set(key, fetchIdentity(sdk, id));
    }
    
    return this.cache.get(key);
  }
  
  async getContract(id: string): Promise<any> {
    const key = `contract:${id}`;
    
    if (!this.cache.has(key)) {
      this.cache.set(key, fetchDataContract(sdk, id));
    }
    
    return this.cache.get(key);
  }
  
  async getDocuments(
    contractId: string,
    type: string,
    query: any
  ): Promise<any[]> {
    const key = `docs:${contractId}:${type}:${JSON.stringify(query)}`;
    
    if (!this.cache.has(key)) {
      this.cache.set(
        key,
        fetchDocuments(sdk, contractId, type, query)
      );
    }
    
    return this.cache.get(key);
  }
  
  clear(): void {
    this.cache.clear();
  }
}
```

### Resource Management

```javascript
class ResourceManager {
  private monitors: Map<string, any>;
  private subscriptions: Set<() => void>;
  
  constructor() {
    this.monitors = new Map();
    this.subscriptions = new Set();
  }
  
  async startBalanceMonitor(
    identityId: string,
    callback: (balance: any) => void
  ): Promise<void> {
    // Stop existing monitor if any
    this.stopBalanceMonitor(identityId);
    
    const monitor = await monitorIdentityBalance(
      sdk,
      identityId,
      callback,
      10000
    );
    
    this.monitors.set(`balance:${identityId}`, monitor);
  }
  
  stopBalanceMonitor(identityId: string): void {
    const key = `balance:${identityId}`;
    const monitor = this.monitors.get(key);
    
    if (monitor) {
      monitor.active = false;
      this.monitors.delete(key);
    }
  }
  
  addSubscription(unsubscribe: () => void): void {
    this.subscriptions.add(unsubscribe);
  }
  
  cleanup(): void {
    // Stop all monitors
    for (const monitor of this.monitors.values()) {
      monitor.active = false;
    }
    this.monitors.clear();
    
    // Unsubscribe all
    for (const unsubscribe of this.subscriptions) {
      unsubscribe();
    }
    this.subscriptions.clear();
  }
}

// Usage with automatic cleanup
const resources = new ResourceManager();

// Start monitoring
await resources.startBalanceMonitor('identity-id', (balance) => {
  console.log('Balance updated:', balance);
});

// Clean up when done
window.addEventListener('beforeunload', () => {
  resources.cleanup();
});
```

## Utility Functions

```javascript
// Helper functions used in examples

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}

function generateDocumentId(): string {
  const array = new Uint8Array(32);
  crypto.getRandomValues(array);
  return Array.from(array)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

async function derivePublicKey(privateKey: Uint8Array): Promise<Uint8Array> {
  // This is a placeholder - use proper crypto library
  // For example: @dashevo/dashcore-lib
  return new Uint8Array(33); // Compressed public key
}

function parseIdentityId(stateTransition: Uint8Array): string {
  // Extract identity ID from state transition
  // This is implementation-specific
  return 'parsed-identity-id';
}

function parseContractId(stateTransition: Uint8Array): string {
  // Extract contract ID from state transition
  return 'parsed-contract-id';
}

async function signStateTransition(
  stateTransition: Uint8Array,
  signer: WasmSigner
): Promise<Uint8Array> {
  // Sign the state transition
  // This would involve proper serialization and signing
  return stateTransition;
}

async function fetchDocument(
  sdk: WasmSdk,
  contractId: string,
  documentType: string,
  documentId: string
): Promise<any> {
  const query = new DocumentQuery(contractId, documentType);
  query.addWhereClause('$id', '=', documentId);
  
  const docs = await fetchDocuments(
    sdk,
    contractId,
    documentType,
    query.getWhereClauses()
  );
  
  return docs[0];
}
```

## Best Practices

1. **Always initialize the WASM module** before using any SDK functions
2. **Use type-safe TypeScript** for better development experience
3. **Implement proper error handling** for all async operations
4. **Cache frequently accessed data** to reduce network calls
5. **Batch operations** when possible for better performance
6. **Clean up resources** (monitors, subscriptions) when done
7. **Use unproved fetching** when cryptographic verification isn't required
8. **Monitor identity balances** before performing credit-consuming operations
9. **Implement retry logic** for network operations
10. **Use appropriate indices** in data contracts for efficient querying