import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class IdentitiesFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async fetch(identityId: string): Promise<wasm.Identity | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentity(identityId);
  }

  async fetchWithProof(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityWithProofInfo(identityId);
  }

  async fetchUnproved(identityId: string): Promise<wasm.Identity> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityUnproved(identityId);
  }

  async getKeys(args: { identityId: string; keyRequestType: 'all' | 'specific' | 'search'; specificKeyIds?: number[]; searchPurposeMap?: unknown; limit?: number; offset?: number }): Promise<any> {
    const { identityId, keyRequestType, specificKeyIds, searchPurposeMap, limit, offset } = args;
    const mapJson = asJsonString(searchPurposeMap);
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityKeys(
      identityId,
      keyRequestType,
      specificKeyIds ? Uint32Array.from(specificKeyIds) : null,
      mapJson ?? null,
      limit ?? null,
      offset ?? null,
    );
  }

  async getKeysWithProof(args: { identityId: string; keyRequestType: 'all' | 'specific' | 'search'; specificKeyIds?: number[]; limit?: number; offset?: number }): Promise<any> {
    const { identityId, keyRequestType, specificKeyIds, limit, offset } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityKeysWithProofInfo(
      identityId,
      keyRequestType,
      specificKeyIds ? Uint32Array.from(specificKeyIds) : null,
      limit ?? null,
      offset ?? null,
    );
  }

  async nonce(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityNonce(identityId);
  }

  async nonceWithProof(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityNonceWithProofInfo(identityId);
  }

  async contractNonce(identityId: string, contractId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityContractNonce(identityId, contractId);
  }

  async contractNonceWithProof(identityId: string, contractId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityContractNonceWithProofInfo(identityId, contractId);
  }

  async balance(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalance(identityId);
  }

  async balanceWithProof(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceWithProofInfo(identityId);
  }

  async balances(identityIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesBalances(identityIds);
  }

  async balancesWithProof(identityIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesBalancesWithProofInfo(identityIds);
  }

  async balanceAndRevision(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceAndRevision(identityId);
  }

  async balanceAndRevisionWithProof(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityBalanceAndRevisionWithProofInfo(identityId);
  }

  async byPublicKeyHash(publicKeyHash: string): Promise<wasm.Identity> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByPublicKeyHash(publicKeyHash);
  }

  async byPublicKeyHashWithProof(publicKeyHash: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByPublicKeyHashWithProofInfo(publicKeyHash);
  }

  async byNonUniquePublicKeyHash(publicKeyHash: string, opts: { startAfter?: string } = {}): Promise<any> {
    const { startAfter } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByNonUniquePublicKeyHash(publicKeyHash, startAfter ?? null);
  }

  async byNonUniquePublicKeyHashWithProof(publicKeyHash: string, opts: { startAfter?: string } = {}): Promise<any> {
    const { startAfter } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityByNonUniquePublicKeyHashWithProofInfo(publicKeyHash, startAfter ?? null);
  }

  async contractKeys(args: { identityIds: string[]; contractId: string; purposes?: number[] }): Promise<any> {
    const { identityIds, contractId, purposes } = args;
    const purposesArray = purposes && purposes.length > 0 ? Uint32Array.from(purposes) : null;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesContractKeys(identityIds, contractId, purposesArray);
  }

  async contractKeysWithProof(args: { identityIds: string[]; contractId: string; purposes?: number[] }): Promise<any> {
    const { identityIds, contractId, purposes } = args;
    const purposesArray = purposes && purposes.length > 0 ? Uint32Array.from(purposes) : null;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesContractKeysWithProofInfo(identityIds, contractId, purposesArray);
  }

  async tokenBalances(identityId: string, tokenIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenBalances(identityId, tokenIds);
  }

  async tokenBalancesWithProof(identityId: string, tokenIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenBalancesWithProofInfo(identityId, tokenIds);
  }

  async create(args: { assetLockProof: unknown; assetLockPrivateKeyWif: string; publicKeys: unknown[] }): Promise<any> {
    const { assetLockProof, assetLockPrivateKeyWif, publicKeys } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreate(asJsonString(assetLockProof)!, assetLockPrivateKeyWif, asJsonString(publicKeys)!);
  }

  async topUp(args: { identityId: string; assetLockProof: unknown; assetLockPrivateKeyWif: string }): Promise<any> {
    const { identityId, assetLockProof, assetLockPrivateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityTopUp(identityId, asJsonString(assetLockProof)!, assetLockPrivateKeyWif);
  }

  async creditTransfer(args: { senderId: string; recipientId: string; amount: number | bigint | string; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { senderId, recipientId, amount, privateKeyWif, keyId } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreditTransfer(senderId, recipientId, BigInt(amount), privateKeyWif, keyId ?? null);
  }

  async creditWithdrawal(args: { identityId: string; toAddress: string; amount: number | bigint | string; coreFeePerByte?: number; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { identityId, toAddress, amount, coreFeePerByte = 1, privateKeyWif, keyId } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.identityCreditWithdrawal(identityId, toAddress, BigInt(amount), coreFeePerByte ?? null, privateKeyWif, keyId ?? null);
  }

  async update(args: { identityId: string; addPublicKeys?: unknown[]; disablePublicKeyIds?: number[]; privateKeyWif: string }): Promise<any> {
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
