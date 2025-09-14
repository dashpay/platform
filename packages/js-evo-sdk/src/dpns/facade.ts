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

  async isNameAvailable(label: string): Promise<boolean> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.dpns_is_name_available(w, label);
  }

  async resolveName(name: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.dpns_resolve_name(w, name);
  }

  async registerName(args: { label: string; identityId: string; publicKeyId: number; privateKeyWif: string; onPreorder?: Function }): Promise<any> {
    const { label, identityId, publicKeyId, privateKeyWif, onPreorder } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.dpns_register_name(w, label, identityId, publicKeyId, privateKeyWif, onPreorder ?? null);
  }

  async usernames(identityId: string, opts: { limit?: number } = {}): Promise<any> {
    const { limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_dpns_usernames(w, identityId, limit ?? null);
  }

  async username(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_dpns_username(w, identityId);
  }

  async usernamesWithProof(identityId: string, opts: { limit?: number } = {}): Promise<any> {
    const { limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_dpns_usernames_with_proof_info(w, identityId, limit ?? null);
  }

  async usernameWithProof(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_dpns_username_with_proof_info(w, identityId);
  }

  async getUsernameByName(username: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_dpns_username_by_name(w, username);
  }

  async getUsernameByNameWithProof(username: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_dpns_username_by_name_with_proof_info(w, username);
  }
}
