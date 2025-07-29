/**
 * TypeScript definitions for @dashevo/wasm-sdk
 * This file ensures type compatibility between js-dash-sdk and wasm-sdk
 */

declare module '@dashevo/wasm-sdk' {
  export interface WasmTransport {
    url: string;
    network: string;
  }

  export interface IdentityPublicKey {
    id: number;
    type: number; // 0 = ECDSA_SECP256K1, 1 = BLS12_381, 2 = ECDSA_HASH160
    purpose: number; // 0 = AUTHENTICATION, 1 = ENCRYPTION, 2 = DECRYPTION, 3 = TRANSFER
    securityLevel: number; // 0 = MASTER, 1 = CRITICAL, 2 = HIGH, 3 = MEDIUM
    data: string; // Base64-encoded public key
    readOnly: boolean;
  }

  export interface Identity {
    id: string;
    publicKeys: IdentityPublicKey[];
    balance: number;
    revision: number;
  }

  export interface Document {
    id: string;
    dataContractId: string;
    ownerId: string;
    revision: number;
    createdAt?: number;
    updatedAt?: number;
    data: Record<string, any>;
  }

  export interface DataContract {
    id: string;
    version: number;
    ownerId: string;
    schema: object;
    documents: Record<string, object>;
  }

  export interface StateTransitionResult {
    success: boolean;
    data?: any;
    error?: string;
  }

  export class WasmSdk {
    static new(transport: WasmTransport, proofs: boolean): Promise<WasmSdk>;

    // Identity queries
    getIdentity(id: string): Promise<Identity | null>;
    getIdentityKeys(identityId: string, keyRequestType?: string, specificKeyIds?: number[], searchPurposeMap?: string): Promise<any>;
    getIdentityNonce(identityId: string): Promise<number>;
    getIdentityContractNonce(identityId: string, contractId: string): Promise<number>;
    getIdentityBalance(id: string): Promise<number>;
    getIdentityBalanceAndRevision(id: string): Promise<{ balance: number; revision: number }>;

    // Identity state transitions
    identityCreate(assetLockProof: string, assetLockProofPrivateKey: string, publicKeys: string): Promise<StateTransitionResult>;
    identityTopUp(identityId: string, assetLockProof: string, assetLockProofPrivateKey: string): Promise<StateTransitionResult>;
    identityUpdate(identityHex: string, identityPrivateKeyHex: string, addPublicKeys?: string, disablePublicKeys?: string): Promise<StateTransitionResult>;
    identityCreditTransfer(identityHex: string, identityPrivateKeyHex: string, recipientId: string, amount: number): Promise<StateTransitionResult>;
    identityCreditWithdrawal(identityHex: string, identityPrivateKeyHex: string, amount: number, coreFeePerByte: number, pooling: string, outputScriptBytes: string): Promise<StateTransitionResult>;

    // Document queries
    getDocument(dataContractId: string, documentType: string, documentId: string): Promise<Document | null>;
    getDocuments(dataContractId: string, documentType: string, where?: string, orderBy?: string, limit?: number, startAt?: number, startAfter?: number): Promise<Document[]>;

    // Document state transitions
    documentsPut(dataContractId: string, documentType: string, documentsJson: string, identityHex: string, identityPrivateKeyHex: string, putType: string): Promise<StateTransitionResult>;
    documentsDelete(dataContractId: string, documentType: string, documentIds: string[], identityHex: string, identityPrivateKeyHex: string): Promise<StateTransitionResult>;

    // Data contract queries
    getDataContract(contractId: string): Promise<DataContract | null>;
    getDataContractHistory(identifier: string, startAtMs: bigint, limit: number, offset: number): Promise<any>;

    // Data contract state transitions
    dataContractCreate(dataContractJson: string, identityHex: string, identityPrivateKeyHex: string): Promise<StateTransitionResult>;
    dataContractUpdate(dataContractJson: string, identityHex: string, identityPrivateKeyHex: string): Promise<StateTransitionResult>;

    // Name queries
    getNameBySearch(searchString: string, parentDomainName?: string): Promise<any>;
    getNameByRecord(recordName: string, recordValue: string, parentDomainName?: string): Promise<any>;

    // Name state transitions
    dpnsRegister(identityHex: string, identityPrivateKeyHex: string, name: string, recordsJson: string): Promise<StateTransitionResult>;

    // Token queries
    getIdentityTokenBalances(identityId: string, tokenIds: string[]): Promise<any>;
    getIdentitiesTokenBalances(identityIds: string[], tokenId: string): Promise<any>;
    getIdentityTokenInfos(identityId: string, tokenIds?: string[]): Promise<any>;
    getIdentitiesTokenInfos(identityIds: string[], tokenId: string): Promise<any>;
    getTokenInfo(tokenId: string): Promise<any>;
    getTokenInfos(tokenIds: string[]): Promise<any>;
    getTokenTotalSupply(tokenId: string): Promise<any>;
    getTokenTotalSupplies(tokenIds: string[]): Promise<any>;
    getTokenCurrentSupply(tokenId: string): Promise<any>;
    getTokenCurrentSupplies(tokenIds: string[]): Promise<any>;
    getGroupInfo(groupId: string): Promise<any>;
    getGroupInfos(groupIds: string[]): Promise<any>;

    // Epoch queries
    getCurrentEpochInfo(): Promise<any>;
    getRecentEpochInfo(count: number, startEpoch?: number): Promise<any>;
    getFutureEpochInfo(count: number, startEpoch?: number): Promise<any>;
    getEpochStartTimestamp(epoch: number): Promise<any>;
    getEpochStartTimestamps(epochs: number[]): Promise<any>;

    // Other queries
    waitForStateTransitionResult(stateTransitionHash: string): Promise<any>;
    broadcastStateTransition(stateTransition: string): Promise<any>;
  }

  // Export the default initialization function
  export default function init(): Promise<void>;
}