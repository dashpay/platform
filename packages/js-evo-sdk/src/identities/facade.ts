import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class IdentitiesFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  fetch(identityId: string): Promise<wasm.IdentityWasm> {
    return wasm.identity_fetch(this.sdk.wasm, identityId);
  }

  fetchWithProof(identityId: string): Promise<any> {
    return wasm.identity_fetch_with_proof_info(this.sdk.wasm, identityId);
  }

  fetchUnproved(identityId: string): Promise<wasm.IdentityWasm> {
    return wasm.identity_fetch_unproved(this.sdk.wasm, identityId);
  }

  getKeys(args: { identityId: string; keyRequestType: 'all' | 'specific' | 'search'; specificKeyIds?: number[]; searchPurposeMap?: unknown; limit?: number; offset?: number }): Promise<any> {
    const { identityId, keyRequestType, specificKeyIds, searchPurposeMap, limit, offset } = args;
    const mapJson = asJsonString(searchPurposeMap);
    return wasm.get_identity_keys(
      this.sdk.wasm,
      identityId,
      keyRequestType,
      specificKeyIds ? Uint32Array.from(specificKeyIds) : null,
      mapJson ?? null,
      limit ?? null,
      offset ?? null,
    );
  }

  create(args: { assetLockProof: unknown; assetLockPrivateKeyWif: string; publicKeys: unknown[] }): Promise<any> {
    const { assetLockProof, assetLockPrivateKeyWif, publicKeys } = args;
    return this.sdk.wasm.identityCreate(asJsonString(assetLockProof)!, assetLockPrivateKeyWif, asJsonString(publicKeys)!);
  }

  topUp(args: { identityId: string; assetLockProof: unknown; assetLockPrivateKeyWif: string }): Promise<any> {
    const { identityId, assetLockProof, assetLockPrivateKeyWif } = args;
    return this.sdk.wasm.identityTopUp(identityId, asJsonString(assetLockProof)!, assetLockPrivateKeyWif);
  }

  creditTransfer(args: { senderId: string; recipientId: string; amount: number | bigint | string; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { senderId, recipientId, amount, privateKeyWif, keyId } = args;
    return this.sdk.wasm.identityCreditTransfer(senderId, recipientId, BigInt(amount), privateKeyWif, keyId ?? null);
  }

  creditWithdrawal(args: { identityId: string; toAddress: string; amount: number | bigint | string; coreFeePerByte?: number; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { identityId, toAddress, amount, coreFeePerByte = 1, privateKeyWif, keyId } = args;
    return this.sdk.wasm.identityCreditWithdrawal(identityId, toAddress, BigInt(amount), coreFeePerByte ?? null, privateKeyWif, keyId ?? null);
  }

  update(args: { identityId: string; addPublicKeys?: unknown[]; disablePublicKeyIds?: number[]; privateKeyWif: string }): Promise<any> {
    const { identityId, addPublicKeys, disablePublicKeyIds, privateKeyWif } = args;
    return this.sdk.wasm.identityUpdate(
      identityId,
      addPublicKeys ? asJsonString(addPublicKeys)! : null,
      disablePublicKeyIds ? Uint32Array.from(disablePublicKeyIds) : null,
      privateKeyWif,
    );
  }
}

