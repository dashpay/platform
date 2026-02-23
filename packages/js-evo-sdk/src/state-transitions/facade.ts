import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class StateTransitionsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async broadcastStateTransition(
    stateTransition: wasm.StateTransition,
    settings?: wasm.PutSettings,
  ): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.broadcastStateTransition(stateTransition, settings);
  }

  async waitForResponse(
    stateTransition: wasm.StateTransition,
    settings?: wasm.PutSettings,
  ): Promise<wasm.StateTransitionProofResultType> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.waitForResponse(stateTransition, settings);
  }

  async broadcastAndWait(
    stateTransition: wasm.StateTransition,
    settings?: wasm.PutSettings,
  ): Promise<wasm.StateTransitionProofResultType> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.broadcastAndWait(stateTransition, settings);
  }

  async waitForStateTransitionResult(stateTransitionHash: string): Promise<wasm.StateTransitionResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.waitForStateTransitionResult(stateTransitionHash);
  }
}
