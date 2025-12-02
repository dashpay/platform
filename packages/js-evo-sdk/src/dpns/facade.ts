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

  async resolveName(name: string): Promise<string | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsResolveName(name);
  }

  async registerName(args: { label: string; identityId: wasm.IdentifierLike; publicKeyId: number; privateKeyWif: string; onPreorder?: Function }): Promise<wasm.RegisterDpnsNameResult> {
    const { label, identityId, publicKeyId, privateKeyWif, onPreorder } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.dpnsRegisterName(label, identityId, publicKeyId, privateKeyWif, onPreorder ?? null);
  }

  async usernames(query: wasm.DpnsUsernamesQuery): Promise<string[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernames(query);
  }

  async username(identityId: wasm.IdentifierLike): Promise<string | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsername(identityId);
  }

  async usernamesWithProof(query: wasm.DpnsUsernamesQuery): Promise<wasm.ProofMetadataResponseTyped<Array<string>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernamesWithProofInfo(query);
  }

  async usernameWithProof(identityId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<string | null>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameWithProofInfo(identityId);
  }

  async getUsernameByName(username: string): Promise<wasm.DpnsUsernameInfo> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameByName(username);
  }

  async getUsernameByNameWithProof(username: string): Promise<wasm.ProofMetadataResponseTyped<wasm.DpnsUsernameInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDpnsUsernameByNameWithProofInfo(username);
  }
}
