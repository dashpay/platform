import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class DpnsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async convertToHomographSafe(input: string): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.dpnsConvertToHomographSafe(input);
  }

  async isValidUsername(label: string): Promise<boolean> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.dpnsIsValidUsername(label);
  }

  async isContestedUsername(label: string): Promise<boolean> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.dpnsIsContestedUsername(label);
  }

  async isNameAvailable(label: string): Promise<boolean> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsIsNameAvailable(label);
  }

  async resolveName(name: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsResolveName(name);
  }

  async registerName(args: { label: string; identityId: string; publicKeyId: number; privateKeyWif: string; onPreorder?: Function }): Promise<any> {
    const { label, identityId, publicKeyId, privateKeyWif, onPreorder } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsRegisterName(label, identityId, publicKeyId, privateKeyWif, onPreorder ?? null);
  }

  async usernames(identityId: string, opts: { limit?: number } = {}): Promise<any> {
    const { limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernames(identityId, limit ?? null);
  }

  async username(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsername(identityId);
  }

  async usernamesWithProof(identityId: string, opts: { limit?: number } = {}): Promise<any> {
    const { limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernamesWithProofInfo(identityId, limit ?? null);
  }

  async usernameWithProof(identityId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameWithProofInfo(identityId);
  }

  async getUsernameByName(username: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameByName(username);
  }

  async getUsernameByNameWithProof(username: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameByNameWithProofInfo(username);
  }
}
