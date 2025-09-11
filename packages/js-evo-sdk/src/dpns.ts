import * as wasm from './wasm';
import { asJsonString } from './util';

export class DpnsFacade {
  private _sdk: wasm.WasmSdk;

  constructor(sdk: wasm.WasmSdk) {
    this._sdk = sdk;
  }

  convertToHomographSafe(input: string): string {
    return wasm.dpns_convert_to_homograph_safe(input);
  }

  isValidUsername(label: string): boolean {
    return wasm.dpns_is_valid_username(label);
  }

  isContestedUsername(label: string): boolean {
    return wasm.dpns_is_contested_username(label);
  }

  isNameAvailable(label: string): Promise<boolean> {
    return wasm.dpns_is_name_available(this._sdk, label);
  }

  resolveName(name: string): Promise<any> {
    return wasm.dpns_resolve_name(this._sdk, name);
  }

  registerName(args: { label: string; identityId: string; publicKeyId: number; privateKeyWif: string; onPreorder?: Function }): Promise<any> {
    const { label, identityId, publicKeyId, privateKeyWif, onPreorder } = args;
    return wasm.dpns_register_name(this._sdk, label, identityId, publicKeyId, privateKeyWif, onPreorder ?? null);
  }

  usernames(identityId: string, opts: { limit?: number } = {}): Promise<any> {
    const { limit } = opts;
    return wasm.get_dpns_usernames(this._sdk, identityId, limit ?? null);
  }

  username(identityId: string): Promise<any> {
    return wasm.get_dpns_username(this._sdk, identityId);
  }

  usernamesWithProof(identityId: string, opts: { limit?: number } = {}): Promise<any> {
    const { limit } = opts;
    return wasm.get_dpns_usernames_with_proof_info(this._sdk, identityId, limit ?? null);
  }

  usernameWithProof(identityId: string): Promise<any> {
    return wasm.get_dpns_username_with_proof_info(this._sdk, identityId);
  }

  getUsernameByName(username: string): Promise<any> {
    return wasm.get_dpns_username_by_name(this._sdk, username);
  }

  getUsernameByNameWithProof(username: string): Promise<any> {
    return wasm.get_dpns_username_by_name_with_proof_info(this._sdk, username);
  }
}
