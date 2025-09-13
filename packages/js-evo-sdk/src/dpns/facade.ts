import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class DpnsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
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
    return wasm.dpns_is_name_available(this.sdk.wasm, label);
  }

  resolveName(name: string): Promise<any> {
    return wasm.dpns_resolve_name(this.sdk.wasm, name);
  }

  registerName(args: { label: string; identityId: string; publicKeyId: number; privateKeyWif: string; onPreorder?: Function }): Promise<any> {
    const { label, identityId, publicKeyId, privateKeyWif, onPreorder } = args;
    return wasm.dpns_register_name(this.sdk.wasm, label, identityId, publicKeyId, privateKeyWif, onPreorder ?? null);
  }

  usernames(identityId: string, opts: { limit?: number } = {}): Promise<any> {
    const { limit } = opts;
    return wasm.get_dpns_usernames(this.sdk.wasm, identityId, limit ?? null);
  }

  username(identityId: string): Promise<any> {
    return wasm.get_dpns_username(this.sdk.wasm, identityId);
  }

  usernamesWithProof(identityId: string, opts: { limit?: number } = {}): Promise<any> {
    const { limit } = opts;
    return wasm.get_dpns_usernames_with_proof_info(this.sdk.wasm, identityId, limit ?? null);
  }

  usernameWithProof(identityId: string): Promise<any> {
    return wasm.get_dpns_username_with_proof_info(this.sdk.wasm, identityId);
  }

  getUsernameByName(username: string): Promise<any> {
    return wasm.get_dpns_username_by_name(this.sdk.wasm, username);
  }

  getUsernameByNameWithProof(username: string): Promise<any> {
    return wasm.get_dpns_username_by_name_with_proof_info(this.sdk.wasm, username);
  }
}

