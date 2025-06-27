# Dash Platform WASM SDK API Reference

**Version**: 1.0.0  
**Generated**: November 2024  
**Platform Version**: 1.0.0+  
**License**: MIT

Complete API documentation for the Dash Platform WebAssembly SDK.

## Table of Contents

1. [Core SDK](#core-sdk)
2. [Identity Management](#identity-management)
3. [Data Contracts](#data-contracts)
4. [Documents](#documents)
5. [State Transitions](#state-transitions)
6. [Signing](#signing)
7. [Transport Layer](#transport-layer)
8. [Token Management](#token-management)
9. [Withdrawals](#withdrawals)
10. [Proof Verification](#proof-verification)
11. [Cache Management](#cache-management)
12. [Error Handling](#error-handling)
13. [Utility Functions](#utility-functions)

## Core SDK

### `start()`

Initialize the WASM module. Must be called before using any SDK functionality.

```typescript
async function start(): Promise<void>
```

**Example:**
```javascript
import { start } from 'dash-wasm-sdk';
await start();
```

### `WasmSdk`

Main SDK class for interacting with Dash Platform.

```typescript
class WasmSdk {
  constructor(
    network: "mainnet" | "testnet" | "devnet",
    contextProvider?: ContextProvider
  )
  
  get network(): string
  isReady(): boolean
}
```

**Parameters:**
- `network`: The Dash network to connect to
- `contextProvider`: Optional custom context provider

**Example:**
```javascript
const sdk = new WasmSdk('testnet');
```

### `ContextProvider`

Abstract class for providing blockchain context.

```typescript
abstract class ContextProvider {
  abstract getBlockHeight(): Promise<number>
  abstract getCoreChainLockedHeight(): Promise<number>
  abstract getTimeMillis(): Promise<number>
}
```

## Identity Management

### `fetchIdentity()`

Fetch an identity from the platform with proof verification.

```typescript
async function fetchIdentity(
  sdk: WasmSdk,
  identityId: string,
  options?: FetchOptions
): Promise<Identity>
```

**Parameters:**
- `sdk`: The SDK instance
- `identityId`: Base58-encoded identity identifier
- `options`: Optional fetch configuration

**Returns:** Identity object with verified proof

### `fetchIdentityUnproved()`

Fetch an identity without proof verification (faster).

```typescript
async function fetchIdentityUnproved(
  sdk: WasmSdk,
  identityId: string,
  options?: FetchOptions
): Promise<Identity>
```

### `createIdentity()`

Create a new identity state transition.

```typescript
function createIdentity(
  assetLockProof: Uint8Array,
  publicKeys: PublicKey[]
): Uint8Array
```

**Parameters:**
- `assetLockProof`: Serialized asset lock proof
- `publicKeys`: Array of public keys for the identity

**Returns:** Serialized identity create state transition

### `updateIdentity()`

Update an existing identity.

```typescript
function updateIdentity(
  identityId: string,
  revision: bigint,
  addPublicKeys: PublicKey[],
  disablePublicKeys: number[],
  publicKeysDisabledAt?: bigint,
  signaturePublicKeyId: number
): Uint8Array
```

### `topupIdentity()`

Top up identity balance with credits.

```typescript
function topupIdentity(
  identityId: string,
  assetLockProof: Uint8Array
): Uint8Array
```

### Identity Balance Functions

#### `fetchIdentityBalance()`

Get identity credit balance.

```typescript
async function fetchIdentityBalance(
  sdk: WasmSdk,
  identityId: string
): Promise<IdentityBalance>

interface IdentityBalance {
  readonly confirmed: number
  readonly unconfirmed: number
  readonly total: number
  toObject(): any
}
```

#### `fetchIdentityRevision()`

Get identity revision information.

```typescript
async function fetchIdentityRevision(
  sdk: WasmSdk,
  identityId: string
): Promise<IdentityRevision>

interface IdentityRevision {
  readonly revision: number
  readonly updatedAt: number
  readonly publicKeysCount: number
  toObject(): any
}
```

#### `checkIdentityBalance()`

Check if identity has sufficient balance.

```typescript
async function checkIdentityBalance(
  sdk: WasmSdk,
  identityId: string,
  requiredAmount: number,
  useUnconfirmed: boolean
): Promise<boolean>
```

#### `estimateCreditsNeeded()`

Estimate credits needed for an operation.

```typescript
function estimateCreditsNeeded(
  operationType: string,
  dataSizeBytes?: number
): number
```

**Operation Types:**
- `"document_create"`: 1000 base credits
- `"document_update"`: 500 base credits
- `"document_delete"`: 200 base credits
- `"identity_update"`: 2000 base credits
- `"identity_topup"`: 100 base credits
- `"contract_create"`: 5000 base credits
- `"contract_update"`: 3000 base credits

### Identity Nonce Management

#### `getIdentityNonce()`

Get current identity nonce.

```typescript
async function getIdentityNonce(
  sdk: WasmSdk,
  identityId: string,
  cached: boolean
): Promise<NonceResponse>

interface NonceResponse {
  nonce: bigint
  previousValue: bigint
  metadata: any
}
```

#### `incrementIdentityNonce()`

Increment identity nonce.

```typescript
async function incrementIdentityNonce(
  sdk: WasmSdk,
  identityId: string,
  count?: number
): Promise<NonceResponse>
```

## Data Contracts

### `fetchDataContract()`

Fetch a data contract with proof verification.

```typescript
async function fetchDataContract(
  sdk: WasmSdk,
  contractId: string,
  options?: FetchOptions
): Promise<DataContract>
```

### `createDataContract()`

Create a new data contract.

```typescript
function createDataContract(
  ownerId: string,
  contractDefinition: any,
  identityNonce: bigint,
  signaturePublicKeyId: number
): Uint8Array
```

**Contract Definition Structure:**
```javascript
{
  protocolVersion: number,
  documents: {
    [documentType: string]: {
      type: 'object',
      properties: {
        [propertyName: string]: {
          type: string,
          // ... other JSON Schema properties
        }
      },
      required: string[],
      additionalProperties: boolean,
      indices: Array<{
        name: string,
        properties: Array<{[property: string]: 'asc' | 'desc'}>,
        unique?: boolean
      }>
    }
  }
}
```

### `updateDataContract()`

Update an existing data contract.

```typescript
function updateDataContract(
  contractId: string,
  ownerId: string,
  contractDefinition: any,
  identityContractNonce: bigint,
  signaturePublicKeyId: number
): Uint8Array
```

## Documents

### `fetchDocuments()`

Query documents from a data contract.

```typescript
async function fetchDocuments(
  sdk: WasmSdk,
  contractId: string,
  documentType: string,
  whereClause: any,
  options?: FetchOptions & {
    orderBy?: any,
    limit?: number,
    startAt?: Uint8Array
  }
): Promise<Document[]>
```

### `DocumentQuery`

Helper class for building document queries.

```typescript
class DocumentQuery {
  constructor(contractId: string, documentType: string)
  
  addWhereClause(field: string, operator: string, value: any): void
  addOrderBy(field: string, ascending: boolean): void
  setLimit(limit: number): void
  setOffset(offset: number): void
  getWhereClauses(): any[]
  getOrderByClauses(): any[]
}
```

**Where Clause Operators:**
- `"="`: Equal
- `"!="`: Not equal
- `">"`: Greater than
- `">="`: Greater than or equal
- `"<"`: Less than
- `"<="`: Less than or equal
- `"in"`: In array
- `"contains"`: Array contains value
- `"startsWith"`: String starts with
- `"elementMatch"`: Array element matches condition

### `DocumentBatchBuilder`

Builder for creating document state transitions.

```typescript
class DocumentBatchBuilder {
  constructor(ownerId: string)
  
  addCreateDocument(
    contractId: string,
    documentType: string,
    documentId: string,
    data: any
  ): void
  
  addDeleteDocument(
    contractId: string,
    documentType: string,
    documentId: string
  ): void
  
  addReplaceDocument(
    contractId: string,
    documentType: string,
    documentId: string,
    revision: number,
    data: any
  ): void
  
  build(signaturePublicKeyId: number): Uint8Array
}
```

## State Transitions

### `broadcastStateTransition()`

Broadcast a state transition to the network.

```typescript
async function broadcastStateTransition(
  sdk: WasmSdk,
  stateTransition: Uint8Array,
  options?: BroadcastOptions
): Promise<BroadcastResponse>

interface BroadcastOptions {
  retries?: number
  timeout?: number
}

interface BroadcastResponse {
  success: boolean
  metadata?: any
  error?: string
}
```

### `IdentityTransitionBuilder`

Builder for identity state transitions.

```typescript
class IdentityTransitionBuilder {
  constructor()
  
  setIdentityId(identityId: string): void
  setRevision(revision: bigint): void
  
  buildCreateTransition(assetLockProof: Uint8Array): Uint8Array
  buildTopUpTransition(assetLockProof: Uint8Array): Uint8Array
  buildUpdateTransition(
    signaturePublicKeyId: number,
    publicKeysDisabledAt?: bigint
  ): Uint8Array
}
```

### `DataContractTransitionBuilder`

Builder for data contract state transitions.

```typescript
class DataContractTransitionBuilder {
  constructor(ownerId: string)
  
  setContractId(contractId: string): void
  setVersion(version: number): void
  setUserFeeIncrease(feeIncrease: number): void
  setIdentityNonce(nonce: bigint): void
  setIdentityContractNonce(nonce: bigint): void
  addDocumentSchema(documentType: string, schema: any): void
  setContractDefinition(definition: any): void
  
  buildCreateTransition(signaturePublicKeyId: number): Uint8Array
  buildUpdateTransition(signaturePublicKeyId: number): Uint8Array
}
```

## Signing

### `WasmSigner`

WASM-based signer for state transitions.

```typescript
class WasmSigner {
  constructor()
  
  setIdentityId(identityId: string): void
  addPrivateKey(
    publicKeyId: number,
    privateKey: Uint8Array,
    keyType: string,
    purpose: number
  ): void
  removePrivateKey(publicKeyId: number): boolean
  signData(data: Uint8Array, publicKeyId: number): Promise<Uint8Array>
  getKeyCount(): number
  hasKey(publicKeyId: number): boolean
  getKeyIds(): number[]
}
```

**Key Types:**
- `"ECDSA_SECP256K1"`: ECDSA with secp256k1 curve
- `"BLS12_381"`: BLS signature scheme
- `"ECDSA_HASH160"`: ECDSA with hash160
- `"BIP13_SCRIPT_HASH"`: BIP13 script hash
- `"EDDSA_25519_HASH160"`: EdDSA with hash160

**Key Purposes:**
- `0`: AUTHENTICATION
- `1`: ENCRYPTION
- `2`: DECRYPTION
- `3`: TRANSFER
- `4`: SYSTEM
- `5`: VOTING

### `BrowserSigner`

Browser-based signer using Web Crypto API.

```typescript
class BrowserSigner {
  constructor()
  
  generateKeyPair(
    keyType: string,
    publicKeyId: number
  ): Promise<CryptoKey>
  
  signWithStoredKey(
    data: Uint8Array,
    publicKeyId: number
  ): Promise<Uint8Array>
}
```

### `HDSigner`

Hierarchical Deterministic (HD) key signer.

```typescript
class HDSigner {
  constructor(mnemonic: string, derivationPath: string)
  
  static generateMnemonic(wordCount: number): string
  deriveKey(index: number): Uint8Array
  get derivationPath(): string
}
```

## Transport Layer

### `WasmDapiTransport`

Transport layer for DAPI communication.

```typescript
class WasmDapiTransport {
  constructor(nodeAddresses: string[])
  
  setTimeout(timeoutMs: number): void
  setMaxRetries(maxRetries: number): void
}
```

### `WasmPlatformClient`

Platform-specific DAPI client.

```typescript
class WasmPlatformClient {
  constructor(transport: WasmDapiTransport)
  
  getIdentity(identityId: string, prove: boolean): Promise<any>
  getDataContract(contractId: string, prove: boolean): Promise<any>
  broadcastStateTransition(stateTransition: Uint8Array): Promise<any>
}
```

### `WasmCoreClient`

Core chain DAPI client.

```typescript
class WasmCoreClient {
  constructor(transport: WasmDapiTransport)
  
  getBestBlockHash(): Promise<string>
  getBlock(blockHash: string): Promise<any>
}
```

## Token Management

### Token Operations

#### `mintTokens()`

Mint new tokens.

```typescript
async function mintTokens(
  sdk: WasmSdk,
  tokenId: string,
  amount: number,
  recipientIdentityId: string,
  options?: TokenOptions
): Promise<any>
```

#### `burnTokens()`

Burn existing tokens.

```typescript
async function burnTokens(
  sdk: WasmSdk,
  tokenId: string,
  amount: number,
  ownerIdentityId: string,
  options?: TokenOptions
): Promise<any>
```

#### `transferTokens()`

Transfer tokens between identities.

```typescript
async function transferTokens(
  sdk: WasmSdk,
  tokenId: string,
  amount: number,
  senderIdentityId: string,
  recipientIdentityId: string,
  options?: TokenOptions
): Promise<any>
```

#### `freezeTokens()` / `unfreezeTokens()`

Freeze or unfreeze tokens for an identity.

```typescript
async function freezeTokens(
  sdk: WasmSdk,
  tokenId: string,
  identityId: string,
  options?: TokenOptions
): Promise<any>

async function unfreezeTokens(
  sdk: WasmSdk,
  tokenId: string,
  identityId: string,
  options?: TokenOptions
): Promise<any>
```

### Token Information

#### `getTokenBalance()`

Get token balance for an identity.

```typescript
async function getTokenBalance(
  sdk: WasmSdk,
  tokenId: string,
  identityId: string,
  options?: TokenOptions
): Promise<{
  balance: number
  frozen: boolean
}>
```

#### `getTokenInfo()`

Get token metadata.

```typescript
async function getTokenInfo(
  sdk: WasmSdk,
  tokenId: string,
  options?: TokenOptions
): Promise<{
  totalSupply: number
  decimals: number
  name: string
  symbol: string
}>
```

### Token State Transitions

#### `createTokenIssuance()`

Create token issuance state transition.

```typescript
function createTokenIssuance(
  dataContractId: string,
  tokenPosition: number,
  amount: number,
  identityNonce: number,
  signaturePublicKeyId: number
): Uint8Array
```

#### `createTokenBurn()`

Create token burn state transition.

```typescript
function createTokenBurn(
  dataContractId: string,
  tokenPosition: number,
  amount: number,
  identityNonce: number,
  signaturePublicKeyId: number
): Uint8Array
```

## Withdrawals

### `withdrawFromIdentity()`

Initiate withdrawal from identity to Layer 1.

```typescript
async function withdrawFromIdentity(
  sdk: WasmSdk,
  identityId: string,
  amount: number,
  toAddress: string,
  signaturePublicKeyId: number,
  options?: WithdrawalOptions
): Promise<any>
```

### `createWithdrawalTransition()`

Create withdrawal state transition.

```typescript
function createWithdrawalTransition(
  identityId: string,
  amount: number,
  toAddress: string,
  outputScript: Uint8Array,
  identityNonce: number,
  signaturePublicKeyId: number,
  coreFeePerByte?: number
): Uint8Array
```

### `getWithdrawalStatus()`

Check withdrawal status.

```typescript
async function getWithdrawalStatus(
  sdk: WasmSdk,
  withdrawalId: string,
  options?: WithdrawalOptions
): Promise<{
  status: string
  amount: number
  transactionId: string | null
}>
```

### `calculateWithdrawalFee()`

Calculate withdrawal fee.

```typescript
function calculateWithdrawalFee(
  amount: number,
  outputScriptSize: number,
  coreFeePerByte?: number
): number
```

## Proof Verification

### `verifyIdentityProof()`

Verify identity proof.

```typescript
function verifyIdentityProof(
  proof: Uint8Array,
  identityId: string,
  isProofSubset: boolean,
  platformVersion: number
): any
```

### `verifyDataContractProof()`

Verify data contract proof.

```typescript
function verifyDataContractProof(
  proof: Uint8Array,
  contractId: string,
  isProofSubset: boolean
): any
```

### `verifyDocumentsProof()`

Verify documents proof.

```typescript
function verifyDocumentsProof(
  proof: Uint8Array,
  contract: any,
  documentType: string,
  whereClauses: any,
  orderBy: any,
  limit?: number,
  offset?: number,
  platformVersion: number
): any
```

## Cache Management

### `WasmCacheManager`

Internal cache management for improved performance.

```typescript
class WasmCacheManager {
  constructor()
  
  setTTLs(
    contractsTtl: number,
    identitiesTtl: number,
    documentsTtl: number,
    tokensTtl: number,
    quorumKeysTtl: number,
    metadataTtl: number
  ): void
  
  cacheContract(contractId: string, contractData: Uint8Array): void
  getCachedContract(contractId: string): Uint8Array | undefined
  
  cacheIdentity(identityId: string, identityData: Uint8Array): void
  getCachedIdentity(identityId: string): Uint8Array | undefined
  
  cacheDocument(documentKey: string, documentData: Uint8Array): void
  getCachedDocument(documentKey: string): Uint8Array | undefined
  
  clearAll(): void
  clearCache(cacheType: string): void
  cleanupExpired(): void
  
  getStats(): {
    contracts: number
    identities: number
    documents: number
    tokens: number
    quorumKeys: number
    metadata: number
    totalEntries: number
  }
}
```

## Error Handling

### `WasmError`

WASM-specific error type.

```typescript
class WasmError extends Error {
  readonly category: ErrorCategory
  readonly message: string
}
```

### `ErrorCategory`

Error categories for classification.

```typescript
enum ErrorCategory {
  Network = "Network",
  Serialization = "Serialization",
  Validation = "Validation",
  Platform = "Platform",
  ProofVerification = "ProofVerification",
  StateTransition = "StateTransition",
  Identity = "Identity",
  Document = "Document",
  Contract = "Contract",
  Unknown = "Unknown"
}
```

## Utility Functions

### Request Settings

#### `RequestSettings`

Configure request retry and timeout behavior.

```typescript
class RequestSettings {
  constructor()
  
  setMaxRetries(retries: number): void
  setInitialRetryDelay(delayMs: number): void
  setMaxRetryDelay(delayMs: number): void
  setBackoffMultiplier(multiplier: number): void
  setTimeout(timeoutMs: number): void
  setUseExponentialBackoff(use: boolean): void
  setRetryOnTimeout(retry: boolean): void
  setRetryOnNetworkError(retry: boolean): void
  setCustomHeaders(headers: object): void
  
  getRetryDelay(attempt: number): number
  toObject(): any
}
```

#### `executeWithRetry()`

Execute a function with retry logic.

```typescript
async function executeWithRetry(
  requestFn: () => Promise<any>,
  settings: RequestSettings
): Promise<any>
```

### Asset Lock Proofs

#### `AssetLockProof`

Asset lock proof for identity funding.

```typescript
class AssetLockProof {
  static createInstant(
    transaction: Uint8Array,
    outputIndex: number,
    instantLock: Uint8Array
  ): AssetLockProof
  
  static createChain(
    transaction: Uint8Array,
    outputIndex: number
  ): AssetLockProof
  
  static fromBytes(bytes: Uint8Array): AssetLockProof
  
  get proofType(): string
  get transaction(): Uint8Array
  get outputIndex(): number
  get instantLock(): Uint8Array | undefined
  
  toBytes(): Uint8Array
  toObject(): any
}
```

#### `validateAssetLockProof()`

Validate an asset lock proof.

```typescript
function validateAssetLockProof(
  proof: AssetLockProof,
  identityId?: string
): boolean
```

#### `calculateCreditsFromProof()`

Calculate credits from asset lock proof.

```typescript
function calculateCreditsFromProof(
  proof: AssetLockProof,
  duffsPerCredit?: number
): number
```

### Metadata

#### `Metadata`

Blockchain metadata for responses.

```typescript
class Metadata {
  constructor(
    height: number,
    coreChainLockedHeight: number,
    epoch: number,
    timeMs: number,
    protocolVersion: number,
    chainId: string
  )
  
  get height(): number
  get coreChainLockedHeight(): number
  get epoch(): number
  get timeMs(): number
  get protocolVersion(): number
  get chainId(): string
  
  toObject(): any
}
```

#### `verifyMetadata()`

Verify metadata validity.

```typescript
function verifyMetadata(
  metadata: Metadata,
  currentHeight: number,
  currentTimeMs?: number,
  config: MetadataVerificationConfig
): MetadataVerificationResult
```

### Epoch and Evonode

#### `getCurrentEpoch()`

Get current epoch information.

```typescript
async function getCurrentEpoch(sdk: WasmSdk): Promise<Epoch>

interface Epoch {
  get index(): number
  get startBlockHeight(): number
  get startBlockCoreHeight(): number
  get startTimeMs(): number
  get feeMultiplier(): number
  toObject(): any
}
```

#### `getCurrentEvonodes()`

Get current evonodes.

```typescript
async function getCurrentEvonodes(sdk: WasmSdk): Promise<Evonode[]>

interface Evonode {
  get proTxHash(): Uint8Array
  get ownerAddress(): string
  get votingAddress(): string
  get isHPMN(): boolean
  get platformP2PPort(): number
  get platformHTTPPort(): number
  get nodeIP(): string
  toObject(): any
}
```

## Type Definitions

### Public Key Structure

```typescript
interface PublicKey {
  id: number
  type: number
  purpose: number
  securityLevel: number
  data: Uint8Array
  readOnly: boolean
  disabledAt?: number
}
```

### Fetch Options

```typescript
class FetchOptions {
  constructor()
  withRetries(retries: number): FetchOptions
  withTimeout(timeout: number): FetchOptions
}
```

### Response Types

```typescript
interface FetchResponse {
  readonly data: any
  readonly found: boolean
  readonly metadataHeight: bigint
  readonly metadataCoreChainLockedHeight: number
  readonly metadataEpoch: number
  readonly metadataTimeMs: bigint
  readonly metadataProtocolVersion: number
  readonly metadataChainId: string
}
```

## Constants

### Network Types
- `"mainnet"`: Production network
- `"testnet"`: Test network
- `"devnet"`: Development network

### Key Types
- `0`: ECDSA_SECP256K1
- `1`: BLS12_381
- `2`: ECDSA_HASH160
- `3`: BIP13_SCRIPT_HASH
- `4`: EDDSA_25519_HASH160

### Key Purposes
- `0`: AUTHENTICATION
- `1`: ENCRYPTION
- `2`: DECRYPTION
- `3`: TRANSFER
- `4`: SYSTEM
- `5`: VOTING

### Security Levels
- `0`: MASTER
- `1`: HIGH
- `2`: MEDIUM
- `3`: LOW