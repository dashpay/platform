import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class IdentitiesFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async fetch(identityId: wasm.IdentifierLike): Promise<wasm.Identity | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentity(identityId);
  }

  async fetchWithProof(identityId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.Identity>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityWithProofInfo(identityId);
  }

  async fetchUnproved(identityId: wasm.IdentifierLike): Promise<wasm.Identity> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityUnproved(identityId);
  }

  async getKeys(query: wasm.IdentityKeysQuery): Promise<wasm.IdentityPublicKey[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityKeys(query);
  }

  async getKeysWithProof(query: wasm.IdentityKeysQuery): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityPublicKey[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityKeysWithProofInfo(query);
  }

  async nonce(identityId: wasm.IdentifierLike): Promise<bigint | null | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityNonce(identityId);
  }

  async nonceWithProof(identityId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<bigint | null | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityNonceWithProofInfo(identityId);
  }

  async contractNonce(identityId: wasm.IdentifierLike, contractId: wasm.IdentifierLike): Promise<bigint | null | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityContractNonce(identityId, contractId);
  }

  async contractNonceWithProof(identityId: wasm.IdentifierLike, contractId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<bigint | null | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityContractNonceWithProofInfo(identityId, contractId);
  }

  async balance(identityId: wasm.IdentifierLike): Promise<bigint | null | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalance(identityId);
  }

  async balanceWithProof(identityId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<bigint | null | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceWithProofInfo(identityId);
  }

  async balances(identityIds: wasm.IdentifierLike[]): Promise<Map<wasm.Identifier, bigint | null>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesBalances(identityIds);
  }

  async balancesWithProof(identityIds: wasm.IdentifierLike[]): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, bigint | null>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesBalancesWithProofInfo(identityIds);
  }

  async balanceAndRevision(identityId: wasm.IdentifierLike): Promise<wasm.IdentityBalanceAndRevision | null | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceAndRevision(identityId);
  }

  async balanceAndRevisionWithProof(identityId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityBalanceAndRevision | null | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceAndRevisionWithProofInfo(identityId);
  }

  async byPublicKeyHash(publicKeyHash: string | Uint8Array): Promise<wasm.Identity | null | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByPublicKeyHash(publicKeyHash);
  }

  async byPublicKeyHashWithProof(publicKeyHash: string | Uint8Array): Promise<wasm.ProofMetadataResponseTyped<wasm.Identity | null | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByPublicKeyHashWithProofInfo(publicKeyHash);
  }

  async byNonUniquePublicKeyHash(publicKeyHash: string | Uint8Array, startAfter?: wasm.IdentifierLike): Promise<wasm.Identity[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByNonUniquePublicKeyHash(publicKeyHash, startAfter);
  }

  async byNonUniquePublicKeyHashWithProof(publicKeyHash: string | Uint8Array, startAfter?: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.Identity[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByNonUniquePublicKeyHashWithProofInfo(publicKeyHash, startAfter || undefined);
  }

  async contractKeys(query: wasm.IdentitiesContractKeysQuery): Promise<wasm.IdentityContractKeys[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesContractKeys(query);
  }

  async contractKeysWithProof(query: wasm.IdentitiesContractKeysQuery): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityContractKeys[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesContractKeysWithProofInfo(query);
  }

  async tokenBalances(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<Map<wasm.Identifier, bigint>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenBalances(identityId, tokenIds);
  }

  async tokenBalancesWithProof(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, bigint>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenBalancesWithProofInfo(identityId, tokenIds);
  }

  async create(options: wasm.IdentityCreateOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreate(options);
  }

  async topUp(options: wasm.IdentityTopUpOptions): Promise<bigint> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityTopUp(options);
  }

  async creditTransfer(options: wasm.IdentityCreditTransferOptions): Promise<wasm.IdentityCreditTransferResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreditTransfer(options);
  }

  async creditWithdrawal(options: wasm.IdentityCreditWithdrawalOptions): Promise<bigint> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreditWithdrawal(options);
  }

  async update(options: wasm.IdentityUpdateOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityUpdate(options);
  }
}
