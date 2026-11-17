import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

/**
 * Decision proposals (protocol version 15): yes/no questions any identity puts to the
 * masternodes. A proposal is a `proposal` document of the decision proposals system contract;
 * creating it opens its vote poll, and its owner pays the decision proposal fund, 0.5 Dash.
 * Nothing acts on a result. Every read is a proved query.
 */
export class DecisionProposalsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  /**
   * Builds a proposal to create with `sdk.documents.create`, which states the fund itself and
   * returns the stored proposal with its final id.
   */
  async build(options: wasm.DecisionProposalOptions): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.buildDecisionProposal(options);
  }

  /**
   * A proposal with new text and its next revision, to replace with `sdk.documents.replace`.
   * Platform accepts the edit for a day after the proposal is created.
   */
  async buildEdit(proposal: wasm.Document, edit: wasm.DecisionProposalEdit): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.buildDecisionProposalEdit(proposal, edit);
  }

  /** A proposal by id, or undefined when there is none. */
  async get(proposalId: wasm.IdentifierLike): Promise<wasm.Document | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDecisionProposal(proposalId);
  }

  /**
   * One page of the proposals by end date, then by creation time. `endDateFrom: Date.now()`
   * lists the open ones.
   */
  async byEndDate(
    query?: wasm.DecisionProposalsByEndDateQuery,
  ): Promise<Map<string, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDecisionProposalsByEndDate(query);
  }

  /** One page of an identity's proposals, by creation time. */
  async byOwner(
    query: wasm.DecisionProposalsByOwnerQuery,
  ): Promise<Map<string, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDecisionProposalsByOwner(query);
  }

  /** One page of every proposal, newest first. */
  async byCreatedAt(
    query?: wasm.DecisionProposalsPageQuery,
  ): Promise<Map<string, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDecisionProposalsByCreatedAt(query);
  }

  /** The id of a proposal's poll, as `sdk.voting.votePollsByEndDate` lists it. */
  async pollId(proposalId: wasm.IdentifierLike): Promise<wasm.Identifier> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDecisionProposalPollId(proposalId);
  }

  /**
   * The status and tallies of a proposal's poll: started, with the voting power cast so far,
   * or finished, with its result. Undefined for a proposal never created.
   */
  async pollState(proposalId: wasm.IdentifierLike): Promise<wasm.YesNoVotePollState | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDecisionProposalPollState(proposalId);
  }

  /**
   * A masternode's answer on a proposal, or undefined when it has not voted on it or the poll
   * finished.
   */
  async masternodeVote(
    proTxHash: wasm.IdentifierLike,
    proposalId: wasm.IdentifierLike,
  ): Promise<wasm.MasternodeYesNoVote | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMasternodeDecisionProposalVote(proTxHash, proposalId);
  }

  /** Votes on a proposal for a masternode, and returns the vote as Platform recorded it. */
  async vote(options: wasm.DecisionProposalVoteOptions): Promise<wasm.Vote> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.decisionProposalVote(options);
  }
}
