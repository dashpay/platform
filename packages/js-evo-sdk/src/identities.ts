import * as wasm from './wasm';
import { asJsonString } from './util';

export class IdentitiesFacade {
  private _sdk: wasm.WasmSdk;

  constructor(sdk: wasm.WasmSdk) {
    this._sdk = sdk;
  }

  fetch(identityId: string): Promise<wasm.IdentityWasm> {
    return wasm.identity_fetch(this._sdk, identityId);
  }

  fetchWithProof(identityId: string): Promise<any> {
    return wasm.identity_fetch_with_proof_info(this._sdk, identityId);
  }

  fetchUnproved(identityId: string): Promise<wasm.IdentityWasm> {
    return wasm.identity_fetch_unproved(this._sdk, identityId);
  }

  getKeys(args: { identityId: string; keyRequestType: 'all' | 'specific' | 'search'; specificKeyIds?: number[]; searchPurposeMap?: unknown; limit?: number; offset?: number }): Promise<any> {
    const { identityId, keyRequestType, specificKeyIds, searchPurposeMap, limit, offset } = args;
    const mapJson = asJsonString(searchPurposeMap);
    return wasm.get_identity_keys(
      this._sdk,
      identityId,
      keyRequestType,
      specificKeyIds ?? null,
      mapJson ?? null,
      limit ?? null,
      offset ?? null,
    );
  }

  create(args: { assetLockProof: unknown; assetLockPrivateKeyWif: string; publicKeys: unknown[] }): Promise<any> {
    const { assetLockProof, assetLockPrivateKeyWif, publicKeys } = args;
    return this._sdk.identityCreate(asJsonString(assetLockProof)!, assetLockPrivateKeyWif, asJsonString(publicKeys)!);
  }

  topUp(args: { identityId: string; topUpPrivateKeyWif: string; amount: number | bigint | string; coreFeePerByte?: number }): Promise<any> {
    const { identityId, topUpPrivateKeyWif, amount, coreFeePerByte = 1 } = args;
    return this._sdk.identityTopUp(identityId, BigInt(amount), BigInt(coreFeePerByte), topUpPrivateKeyWif);
  }

  creditTransfer(args: { senderId: string; recipientId: string; amount: number | bigint | string; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { senderId, recipientId, amount, privateKeyWif, keyId } = args;
    return this._sdk.identityCreditTransfer(senderId, recipientId, BigInt(amount), privateKeyWif, keyId ?? null);
  }

  creditWithdrawal(args: { identityId: string; toAddress: string; amount: number | bigint | string; coreFeePerByte?: number; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { identityId, toAddress, amount, coreFeePerByte = 1, privateKeyWif, keyId } = args;
    return this._sdk.identityCreditWithdrawal(identityId, toAddress, BigInt(amount), coreFeePerByte ?? null, privateKeyWif, keyId ?? null);
  }

  update(args: { identityId: string; addPublicKeys?: unknown[]; disablePublicKeyIds?: number[]; privateKeyWif: string }): Promise<any> {
    const { identityId, addPublicKeys, disablePublicKeyIds, privateKeyWif } = args;
    return this._sdk.identityUpdate(
      identityId,
      addPublicKeys ? asJsonString(addPublicKeys)! : null,
      disablePublicKeyIds ? Uint32Array.from(disablePublicKeyIds) : null,
      privateKeyWif,
    );
  }
}
