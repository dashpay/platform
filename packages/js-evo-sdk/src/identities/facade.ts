import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
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

  async getKeys(query: wasm.IdentityKeysQuery): Promise<wasm.IdentityKeyInfo[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityKeys(query);
  }

  async getKeysWithProof(query: wasm.IdentityKeysQuery): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityKeyInfo[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityKeysWithProofInfo(query);
  }

  async nonce(identityId: wasm.IdentifierLike): Promise<wasm.IdentityNonce> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityNonce(identityId);
  }

  async nonceWithProof(identityId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityNonce>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityNonceWithProofInfo(identityId);
  }

  async contractNonce(identityId: wasm.IdentifierLike, contractId: wasm.IdentifierLike): Promise<wasm.IdentityNonce> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityContractNonce(identityId, contractId);
  }

  async contractNonceWithProof(identityId: wasm.IdentifierLike, contractId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityNonce>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityContractNonceWithProofInfo(identityId, contractId);
  }

  async balance(identityId: wasm.IdentifierLike): Promise<wasm.IdentityBalanceInfo> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalance(identityId);
  }

  async balanceWithProof(identityId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityBalanceInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceWithProofInfo(identityId);
  }

  async balances(identityIds: wasm.IdentifierLike[]): Promise<wasm.IdentityBalanceEntry[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesBalances(identityIds);
  }

  async balancesWithProof(identityIds: wasm.IdentifierLike[]): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityBalanceEntry[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesBalancesWithProofInfo(identityIds);
  }

  async balanceAndRevision(identityId: wasm.IdentifierLike): Promise<wasm.IdentityBalanceAndRevision> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceAndRevision(identityId);
  }

  async balanceAndRevisionWithProof(identityId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.IdentityBalanceAndRevision>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceAndRevisionWithProofInfo(identityId);
  }

  async byPublicKeyHash(publicKeyHash: string | Uint8Array): Promise<wasm.Identity> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByPublicKeyHash(publicKeyHash);
  }

  async byPublicKeyHashWithProof(publicKeyHash: string | Uint8Array): Promise<wasm.ProofMetadataResponseTyped<wasm.Identity>> {
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

  async create(args: { assetLockProof: unknown; assetLockPrivateKeyWif: string; publicKeys: unknown[] }): Promise<any> {
    const { assetLockProof, assetLockPrivateKeyWif, publicKeys } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreate(asJsonString(assetLockProof)!, assetLockPrivateKeyWif, asJsonString(publicKeys)!);
  }

  async topUp(args: { identityId: wasm.IdentifierLike; assetLockProof: unknown; assetLockPrivateKeyWif: string }): Promise<any> {
    const { identityId, assetLockProof, assetLockPrivateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityTopUp(identityId, asJsonString(assetLockProof)!, assetLockPrivateKeyWif);
  }

  async creditTransfer(args: { senderId: wasm.IdentifierLike; recipientId: wasm.IdentifierLike; amount: number | bigint | string; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { senderId, recipientId, amount, privateKeyWif, keyId } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreditTransfer(senderId, recipientId, BigInt(amount), privateKeyWif, keyId ?? null);
  }

  async creditWithdrawal(args: { identityId: wasm.IdentifierLike; toAddress: string; amount: number | bigint | string; coreFeePerByte?: number; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { identityId, toAddress, amount, coreFeePerByte = 1, privateKeyWif, keyId } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreditWithdrawal(identityId, toAddress, BigInt(amount), coreFeePerByte ?? null, privateKeyWif, keyId ?? null);
  }

  async update(args: { identityId: wasm.IdentifierLike; addPublicKeys?: unknown[]; disablePublicKeyIds?: number[]; privateKeyWif: string }): Promise<any> {
    const { identityId, addPublicKeys, disablePublicKeyIds, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityUpdate(
      identityId,
      addPublicKeys ? asJsonString(addPublicKeys)! : null,
      disablePublicKeyIds ? Uint32Array.from(disablePublicKeyIds) : null,
      privateKeyWif,
    );
  }
}
