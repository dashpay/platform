import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class IdentitiesFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async fetch(identityId: string): Promise<wasm.IdentityWasm> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.identity_fetch(w, identityId);
  }

  async fetchWithProof(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.identity_fetch_with_proof_info(w, identityId);
  }

  async fetchUnproved(identityId: string): Promise<wasm.IdentityWasm> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.identity_fetch_unproved(w, identityId);
  }

  async getKeys(args: { identityId: string; keyRequestType: 'all' | 'specific' | 'search'; specificKeyIds?: number[]; searchPurposeMap?: unknown; limit?: number; offset?: number }): Promise<any> {
    const { identityId, keyRequestType, specificKeyIds, searchPurposeMap, limit, offset } = args;
    const mapJson = asJsonString(searchPurposeMap);
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_identity_keys(
      w,
      identityId,
      keyRequestType,
      specificKeyIds ? Uint32Array.from(specificKeyIds) : null,
      mapJson ?? null,
      limit ?? null,
      offset ?? null,
    );
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
