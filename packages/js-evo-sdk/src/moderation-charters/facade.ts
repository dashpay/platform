import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

/**
 * The moderation charters system contract (protocol version 14): who moderates a contract
 * that declares elected moderation, the proposals and join requests behind it, and the
 * requests members send the leader. Every read is a proved document query.
 */
export class ModerationChartersFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  /**
   * The seated charter of a contract: its `electedCharter`, or undefined when it has none.
   * Only a contest's winner is ever stored, so there is at most one.
   */
  async seatedCharter(targetContractId: wasm.IdentifierLike): Promise<wasm.Document | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getModerationSeatedCharter(targetContractId);
  }

  /** A proposal (`submittedCharter`) by id, such as a seated charter's `submittedCharterId`. */
  async submittedCharter(submittedCharterId: wasm.IdentifierLike): Promise<wasm.Document | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getModerationSubmittedCharter(submittedCharterId);
  }

  /**
   * The team that moderates a contract: the seated charter's leader plus its elected members
   * and the members the leader added, less those the leader removed. Undefined when the
   * contract has no seated charter.
   */
  async team(targetContractId: wasm.IdentifierLike): Promise<wasm.ModerationTeam | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getModerationTeam(targetContractId);
  }

  /** One page of the proposals for a contract, in filing order. */
  async submittedCharters(
    query: wasm.ModerationSubmittedChartersQuery,
  ): Promise<Map<string, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getModerationSubmittedCharters(query);
  }

  /** One page of the join requests for a proposal, in the order of their owners' ids. */
  async joinRequests(
    query: wasm.ModerationJoinRequestsQuery,
  ): Promise<Map<string, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getModerationJoinRequests(query);
  }

  /** The resignation requests for a seated charter that the leader has not acted on. */
  async pendingResignationRequests(electedCharterId: wasm.IdentifierLike): Promise<wasm.Document[]> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getModerationPendingResignationRequests(electedCharterId);
  }

  /**
   * Builds a join request whose message only the proposal's leader can read. Pass the result
   * to `documents.create`.
   */
  async buildJoinRequest(options: wasm.ModerationJoinRequestOptions): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.buildModerationJoinRequest(options);
  }

  /**
   * Builds a resignation request whose message only the leader can read. Pass the result to
   * `documents.create`; deleting it withdraws the request.
   */
  async buildResignationRequest(options: wasm.ModerationResignationRequestOptions): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.buildModerationResignationRequest(options);
  }
}
