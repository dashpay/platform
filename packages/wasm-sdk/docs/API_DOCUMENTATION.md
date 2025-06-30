# WASM SDK API Documentation

Comprehensive API reference for the Dash Platform WASM SDK.

## Table of Contents

- [Core Classes](#core-classes)
- [Identity Management](#identity-management)
- [Document Operations](#document-operations)
- [State Transitions](#state-transitions)
- [BIP39 & Key Management](#bip39--key-management)
- [DAPI Client](#dapi-client)
- [Subscriptions](#subscriptions)
- [Monitoring](#monitoring)
- [Caching](#caching)
- [Error Types](#error-types)

## Core Classes

### WasmSdk

The main SDK class for interacting with Dash Platform.

```typescript
class WasmSdk {
  constructor(network: 'mainnet' | 'testnet', contextProvider?: ContextProvider);
  
  // Get network name
  network(): string;
  
  // Get or set context provider
  contextProvider(): ContextProvider | undefined;
  setContextProvider(provider: ContextProvider): void;
}
```

### ContextProvider

Manages wallet context and signing capabilities.

```typescript
class ContextProvider {
  constructor();
  
  // Set wallet context
  setWalletContext(context: any): void;
  
  // Get current context
  getWalletContext(): any;
}
```

## Identity Management

### Functions

#### getIdentityInfo

Get comprehensive information about an identity.

```typescript
async function getIdentityInfo(
  sdk: WasmSdk, 
  identityId: string
): Promise<{
  id: string;
  balance: number;
  revision: number;
  publicKeys: Array<{
    id: number;
    type: string;
    purpose: number;
    securityLevel: number;
    data: string;
    readOnly: boolean;
    disabledAt?: number;
  }>;
}>
```

#### getIdentityBalance

Get the current balance of an identity.

```typescript
async function getIdentityBalance(
  sdk: WasmSdk, 
  identityId: string
): Promise<number>
```

#### checkIdentityExists

Check if an identity exists on the platform.

```typescript
async function checkIdentityExists(
  sdk: WasmSdk, 
  identityId: string
): Promise<boolean>
```

#### topUpIdentity

Add credits to an identity balance.

```typescript
async function topUpIdentity(
  sdk: WasmSdk,
  identityId: string,
  amount: number,
  signer: WasmSigner
): Promise<void>
```

#### transferCredits

Transfer credits between identities.

```typescript
async function transferCredits(
  sdk: WasmSdk,
  fromIdentityId: string,
  toIdentityId: string,
  amount: number,
  signer: WasmSigner
): Promise<void>
```

## Document Operations

### Functions

#### createDocument

Create a new document.

```typescript
async function createDocument(
  sdk: WasmSdk,
  contractId: string,
  ownerId: string,
  documentType: string,
  data: object,
  signer: WasmSigner
): Promise<any>
```

#### updateDocument

Update an existing document.

```typescript
async function updateDocument(
  sdk: WasmSdk,
  contractId: string,
  ownerId: string,
  documentType: string,
  documentId: string,
  data: object,
  signer: WasmSigner
): Promise<any>
```

#### deleteDocument

Delete a document.

```typescript
async function deleteDocument(
  sdk: WasmSdk,
  contractId: string,
  ownerId: string,
  documentType: string,
  documentId: string,
  signer: WasmSigner
): Promise<any>
```

### DocumentQuery

Query documents with filters and sorting.

```typescript
class DocumentQuery {
  constructor(contractId: string, documentType: string);
  
  where(field: string, operator: string, value: any): DocumentQuery;
  orderBy(field: string, direction: 'asc' | 'desc'): DocumentQuery;
  limit(count: number): DocumentQuery;
  startAt(documentId: string): DocumentQuery;
  startAfter(documentId: string): DocumentQuery;
}
```

## State Transitions

### Identity State Transitions

```typescript
// Create identity
function createIdentityStateTransition(
  assetLockProof: Uint8Array,
  publicKeys: Array<{
    id: number;
    type: number;
    purpose: number;
    securityLevel: number;
    data: Uint8Array;
    readOnly: boolean;
  }>
): Uint8Array

// Update identity
function createIdentityUpdateTransition(
  identityId: string,
  revision: number,
  addPublicKeys?: Array<PublicKey>,
  disablePublicKeys?: Array<number>,
  publicKeysDisabledAt?: number
): Uint8Array
```

### Data Contract State Transitions

```typescript
function createDataContractStateTransition(
  ownerId: string,
  contractDefinition: object,
  entropy: Uint8Array
): Uint8Array

function updateDataContractStateTransition(
  contractId: string,
  ownerId: string,
  contractDefinition: object,
  revision: number
): Uint8Array
```

## BIP39 & Key Management

### Mnemonic

BIP39 mnemonic phrase management.

```typescript
class Mnemonic {
  static generate(
    strength: MnemonicStrength, 
    language: WordListLanguage
  ): Mnemonic;
  
  static fromPhrase(
    phrase: string, 
    language: WordListLanguage
  ): Mnemonic;
  
  phrase(): string;
  wordCount(): number;
  words(): string[];
  validate(): boolean;
  toSeed(passphrase?: string): Uint8Array;
  toHDPrivateKey(passphrase?: string, network: string): string;
}

enum MnemonicStrength {
  Words12 = 128,
  Words15 = 160,
  Words18 = 192,
  Words21 = 224,
  Words24 = 256
}

enum WordListLanguage {
  English,
  Japanese,
  Korean,
  Spanish,
  ChineseSimplified,
  ChineseTraditional,
  French,
  Italian,
  Czech,
  Portuguese
}
```

### WasmSigner

Signing interface for state transitions.

```typescript
class WasmSigner {
  constructor();
  
  setIdentityId(identityId: string): void;
  addPrivateKey(
    publicKeyId: number,
    privateKey: Uint8Array,
    keyType: string,
    purpose: number
  ): void;
  removePrivateKey(publicKeyId: number): boolean;
  signData(data: Uint8Array, publicKeyId: number): Promise<Uint8Array>;
  hasKey(publicKeyId: number): boolean;
  getKeyIds(): number[];
}
```

### BrowserSigner

Browser-native crypto signing.

```typescript
class BrowserSigner {
  constructor();
  
  generateKeyPair(
    keyType: string, 
    publicKeyId: number
  ): Promise<CryptoKey>;
  
  signWithStoredKey(
    data: Uint8Array, 
    publicKeyId: number
  ): Promise<Uint8Array>;
}
```

## DAPI Client

### DapiClient

Low-level DAPI client for custom requests.

```typescript
class DapiClient {
  constructor(config: DapiClientConfig);
  
  rawRequest(path: string, payload: object): Promise<any>;
  getProtocolVersion(): Promise<number>;
  getEpoch(index: number): Promise<object>;
  getIdentity(identityId: string): Promise<object>;
  getIdentityBalance(identityId: string): Promise<number>;
  getDataContract(contractId: string): Promise<object>;
  getDocuments(
    contractId: string, 
    documentType: string, 
    query: object
  ): Promise<Array<object>>;
  broadcastStateTransition(stBytes: Uint8Array): Promise<object>;
}
```

### DapiClientConfig

Configuration for DAPI client.

```typescript
class DapiClientConfig {
  constructor(network: string);
  
  setTimeout(ms: number): void;
  setRetries(count: number): void;
  addAddress(address: string): void;
}
```

## Subscriptions

### SubscriptionClient

Real-time subscriptions via WebSocket.

```typescript
class SubscriptionClient {
  constructor(network: string);
  
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  
  subscribeToDocuments(
    contractId: string,
    documentType: string,
    callback: (update: any) => void
  ): Promise<string>;
  
  subscribeToIdentity(
    identityId: string,
    callback: (update: any) => void
  ): Promise<string>;
  
  subscribeToTransactions(
    callback: (tx: any) => void
  ): Promise<string>;
  
  unsubscribe(subscriptionId: string): Promise<void>;
  unsubscribeAll(): Promise<void>;
}
```

## Monitoring

### SdkMonitor

Performance and operation monitoring.

```typescript
class SdkMonitor {
  constructor(enabled: boolean, maxMetrics?: number);
  
  enable(): void;
  disable(): void;
  enabled(): boolean;
  
  startOperation(operationId: string, operationName: string): void;
  endOperation(
    operationId: string, 
    success: boolean, 
    error?: string
  ): void;
  
  addOperationMetadata(
    operationId: string, 
    key: string, 
    value: string
  ): void;
  
  getMetrics(): PerformanceMetrics[];
  getMetricsByOperation(operationName: string): PerformanceMetrics[];
  getOperationStats(): object;
  clearMetrics(): void;
}
```

### Global Monitoring Functions

```typescript
function initializeMonitoring(
  enabled: boolean, 
  maxMetrics?: number
): void;

function getGlobalMonitor(): SdkMonitor | null;

async function performHealthCheck(sdk: WasmSdk): Promise<{
  status: 'healthy' | 'unhealthy';
  checks: Map<string, object>;
  timestamp: number;
}>;

function getResourceUsage(): {
  memory?: object;
  activeOperations?: number;
  timestamp: number;
};
```

## Caching

### Cache Functions

```typescript
async function initCache(): Promise<void>;

async function cacheGet(key: string): Promise<any | null>;

async function cacheSet(
  key: string, 
  value: any, 
  ttlMs?: number
): Promise<void>;

async function cacheDelete(key: string): Promise<void>;

async function cacheClear(): Promise<void>;

async function getCacheStats(): Promise<{
  size: number;
  hits: number;
  misses: number;
  evictions: number;
}>;
```

## Error Types

### WasmError

Base error class for all SDK errors.

```typescript
class WasmError extends Error {
  category: ErrorCategory;
  code: string;
  details?: any;
}

enum ErrorCategory {
  Network = 'network',
  Validation = 'validation',
  StateTransition = 'state_transition',
  ProofVerification = 'proof_verification',
  Serialization = 'serialization',
  Unknown = 'unknown'
}
```

### Specific Error Types

```typescript
class DapiClientError extends WasmError {
  endpoint?: string;
  statusCode?: number;
}

class StateTransitionError extends WasmError {
  transitionType?: string;
  validationErrors?: Array<object>;
}

class ProofVerificationError extends WasmError {
  proofType?: string;
  reason?: string;
}