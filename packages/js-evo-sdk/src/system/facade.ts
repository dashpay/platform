import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class SystemFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async status(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getStatus(); }
  async currentQuorumsInfo(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getCurrentQuorumsInfo(); }
  async totalCreditsInPlatform(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getTotalCreditsInPlatform(); }
  async totalCreditsInPlatformWithProof(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getTotalCreditsInPlatformWithProofInfo(); }
  async prefundedSpecializedBalance(identityId: string): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getPrefundedSpecializedBalance(identityId); }
  async prefundedSpecializedBalanceWithProof(identityId: string): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getPrefundedSpecializedBalanceWithProofInfo(identityId); }
  async waitForStateTransitionResult(stateTransitionHash: string): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.waitForStateTransitionResult(stateTransitionHash); }
  async pathElements(path: string[], keys: string[]): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getPathElements(path, keys); }
  async pathElementsWithProof(path: string[], keys: string[]): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getPathElementsWithProofInfo(path, keys); }
}
