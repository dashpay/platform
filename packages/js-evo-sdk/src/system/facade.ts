import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class SystemFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async status(): Promise<wasm.StatusResponse> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getStatus();
  }

  async currentQuorumsInfo(): Promise<wasm.CurrentQuorumsInfo> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getCurrentQuorumsInfo();
  }

  async totalCreditsInPlatform(): Promise<bigint> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTotalCreditsInPlatform();
  }

  async totalCreditsInPlatformWithProof(): Promise<wasm.ProofMetadataResponseTyped<bigint | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTotalCreditsInPlatformWithProofInfo();
  }

  async prefundedSpecializedBalance(
    identityId: wasm.IdentifierLike,
  ): Promise<wasm.PrefundedSpecializedBalance> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getPrefundedSpecializedBalance(identityId);
  }

  async prefundedSpecializedBalanceWithProof(
    identityId: wasm.IdentifierLike,
  ): Promise<wasm.ProofMetadataResponseTyped<
    wasm.PrefundedSpecializedBalance | undefined
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getPrefundedSpecializedBalanceWithProofInfo(identityId);
  }

  async waitForStateTransitionResult(stateTransitionHash: string): Promise<wasm.StateTransitionResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.waitForStateTransitionResult(stateTransitionHash);
  }

  async pathElements(path: string[], keys: string[]): Promise<wasm.PathElement[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getPathElements(path, keys);
  }

  async pathElementsWithProof(path: string[], keys: string[]):
    Promise<wasm.ProofMetadataResponseTyped<wasm.PathElement[]>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getPathElementsWithProofInfo(path, keys);
  }
}
