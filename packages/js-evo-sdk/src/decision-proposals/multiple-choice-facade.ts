import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

/**
 * Multiple choice proposals (protocol version 15): questions whose answers are the choices
 * entered for them. A proposal is a `multipleChoiceProposal` document and an answer a `choice`
 * document of the decision proposals system contract. The choices contend in a masternode
 * vote that the first choice opens, and the one with the most votes is kept. A choice is named
 * by its document id, in a vote and in the vote's state. Every read is a proved query.
 */
export class MultipleChoiceProposalsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  /**
   * Builds a proposal to create with `sdk.documents.create`, which returns the stored proposal
   * with its final id. Its owner pays the processing fee `processingFee()` gives on top of the
   * fees.
   */
  async build(options: wasm.MultipleChoiceProposalOptions): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.buildMultipleChoiceProposal(options);
  }

  /**
   * A proposal with a new title, url or description document and its next revision, to replace
   * with `sdk.documents.replace`. Platform accepts the edit for two hours after the proposal
   * is created.
   */
  async buildEdit(
    proposal: wasm.Document,
    edit: wasm.MultipleChoiceProposalEdit,
  ): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.buildMultipleChoiceProposalEdit(proposal, edit);
  }

  /**
   * Builds a choice to create with `sdk.documents.create`, which states the contest fund and
   * returns the stored choice with its final id, the id a vote towards it carries.
   */
  async buildChoice(options: wasm.MultipleChoiceProposalChoiceOptions): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.buildMultipleChoiceProposalChoice(options);
  }

  /** What creating one proposal adds to the processing fee of its batch, in credits. */
  async processingFee(): Promise<bigint> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMultipleChoiceProposalProcessingFee();
  }

  /** The fund a choice entered now for a proposal pays, in credits. */
  async choiceFund(proposalId: wasm.IdentifierLike): Promise<bigint> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMultipleChoiceProposalChoiceFund(proposalId);
  }

  /** A proposal by id, or undefined when there is none. */
  async get(proposalId: wasm.IdentifierLike): Promise<wasm.Document | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMultipleChoiceProposal(proposalId);
  }

  /** One page of an identity's proposals, by creation time. */
  async byOwner(
    query: wasm.DecisionProposalsByOwnerQuery,
  ): Promise<Map<string, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMultipleChoiceProposalsByOwner(query);
  }

  /** One page of every proposal, newest first. */
  async byCreatedAt(
    query?: wasm.DecisionProposalsPageQuery,
  ): Promise<Map<string, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMultipleChoiceProposalsByCreatedAt(query);
  }

  /**
   * The vote among a proposal's choices as a vote poll, for `sdk.voting` and the contested
   * resource queries.
   */
  async votePoll(proposalId: wasm.IdentifierLike): Promise<wasm.VotePoll> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMultipleChoiceProposalVotePoll(proposalId);
  }

  /**
   * The vote among a proposal's choices: every choice with its tally while the vote runs, and
   * its result with every choice's final tally once it ended.
   */
  async voteState(proposalId: wasm.IdentifierLike): Promise<wasm.MultipleChoiceProposalVote> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMultipleChoiceProposalVote(proposalId);
  }

  /**
   * The choice the finished vote of a proposal kept, or undefined while the vote runs and when
   * it kept none.
   */
  async keptChoice(proposalId: wasm.IdentifierLike): Promise<wasm.Document | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMultipleChoiceProposalKeptChoice(proposalId);
  }

  /**
   * The answer a masternode gave in the vote among a proposal's choices, or undefined when it
   * has not voted in it or the vote ended.
   */
  async masternodeVote(
    proTxHash: wasm.IdentifierLike,
    proposalId: wasm.IdentifierLike,
  ): Promise<wasm.MasternodeMultipleChoiceVote | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMasternodeMultipleChoiceProposalVote(proTxHash, proposalId);
  }

  /**
   * Votes in the vote among a proposal's choices for a masternode, and returns the vote as
   * Platform recorded it.
   */
  async vote(options: wasm.MultipleChoiceProposalVoteOptions): Promise<wasm.Vote> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.multipleChoiceProposalVote(options);
  }
}
