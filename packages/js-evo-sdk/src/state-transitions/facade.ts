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

  /**
   * Verifies that this specific transition executed. An authenticated
   * affected-state snapshot is rejected with `ExecutionNotProved`; use
   * `waitForAffectedState` when a snapshot is the intended result.
   */
  async waitForResponse(
    stateTransition: wasm.StateTransition,
    settings?: wasm.PutSettings,
  ): Promise<wasm.StateTransitionProofResultType> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.waitForResponse(stateTransition, settings);
  }

  /**
   * Broadcasts, then strictly verifies execution of this transition.
   * `ExecutionNotProved` means the accepted broadcast was followed by an
   * authenticated snapshot, which establishes neither execution nor failure
   * of this transition. To wait for an authenticated snapshot, call
   * `waitForAffectedState` on the submitted transition without broadcasting
   * it again; the new wait may observe a later block.
   */
  async broadcastAndWait(
    stateTransition: wasm.StateTransition,
    settings?: wasm.PutSettings,
  ): Promise<wasm.StateTransitionProofResultType> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.broadcastAndWait(stateTransition, settings);
  }

  /**
   * Like `waitForResponse`, but accepts proofs that only authenticate the
   * state the transition affects (balance top-ups, credit transfers and
   * withdrawals, address funds movements, shields, no-history token
   * operations, key limits updates, contract updates, contract moderation
   * and fee claims, and creates and deletes of indexOnly documents).
   * Execution-proved outcomes are accepted too. A snapshot authenticates
   * state at the proof's block; it does not prove this transition executed.
   * This generic JavaScript result does not expose the proof height.
   */
  async waitForAffectedState(
    stateTransition: wasm.StateTransition,
    settings?: wasm.PutSettings,
  ): Promise<wasm.StateTransitionProofResultType> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.waitForAffectedState(stateTransition, settings);
  }

  /**
   * Like `broadcastAndWait`, but accepts affected-state snapshot outcomes
   * (see `waitForAffectedState`).
   */
  async broadcastAndWaitForAffectedState(
    stateTransition: wasm.StateTransition,
    settings?: wasm.PutSettings,
  ): Promise<wasm.StateTransitionProofResultType> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.broadcastAndWaitForAffectedState(stateTransition, settings);
  }

  /**
   * Looks up a transition's status by hash. This method does not authenticate
   * a returned proof; use a full-transition wait for verified results.
   */
  async waitForStateTransitionResult(stateTransitionHash: string): Promise<wasm.StateTransitionResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.waitForStateTransitionResult(stateTransitionHash);
  }
}
