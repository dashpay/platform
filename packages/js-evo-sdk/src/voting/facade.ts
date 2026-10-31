import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class VotingFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async contestedResourceVoteState(
    query: wasm.ContestedResourceVoteStateQuery,
  ): Promise<wasm.ContestedResourceVoteState> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVoteState(query);
  }

  async contestedResourceVoteStateWithProof(
    query: wasm.ContestedResourceVoteStateQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<
    wasm.ContestedResourceVoteState
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVoteStateWithProofInfo(query);
  }

  async contestedResourceIdentityVotes(
    query: wasm.ContestedResourceIdentityVotesQuery,
  ): Promise<Map<string, wasm.ResourceVoteWithCount>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceIdentityVotes(query);
  }

  async contestedResourceIdentityVotesWithProof(
    query: wasm.ContestedResourceIdentityVotesQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<
    Map<string, wasm.ResourceVoteWithCount>
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceIdentityVotesWithProofInfo(query);
  }

  async votePollsByEndDate(query?: wasm.VotePollsByEndDateQuery): Promise<wasm.VotePollsByEndDateEntry[]> {
    const w = await this.sdk.getWasmSdkConnected();

    return w.getVotePollsByEndDate(query);
  }

  async votePollsByEndDateWithProof(
    query?: wasm.VotePollsByEndDateQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<
    wasm.VotePollsByEndDateEntry[]
  >> {
    const w = await this.sdk.getWasmSdkConnected();

    return w.getVotePollsByEndDateWithProofInfo(query);
  }

  /**
   * The status and tallies of a yes/no vote poll (protocol version 15 and later), or
   * `undefined` for a poll never opened. The poll is a `VotePoll` as `votePollsByEndDate`
   * returns it.
   */
  async yesNoVotePollState(votePoll: wasm.VotePoll): Promise<wasm.YesNoVotePollState | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getYesNoVotePollState(votePoll);
  }

  async yesNoVotePollStateWithProof(
    votePoll: wasm.VotePoll,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.YesNoVotePollState | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getYesNoVotePollStateWithProofInfo(votePoll);
  }

  /**
   * A masternode's answer on a yes/no vote poll (protocol version 15 and later), or
   * `undefined` when it has not voted on the poll or the poll finished.
   */
  async masternodeYesNoVote(
    proTxHash: wasm.IdentifierLike,
    votePoll: wasm.VotePoll,
  ): Promise<wasm.MasternodeYesNoVote | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMasternodeYesNoVote(proTxHash, votePoll);
  }

  async masternodeYesNoVoteWithProof(
    proTxHash: wasm.IdentifierLike,
    votePoll: wasm.VotePoll,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.MasternodeYesNoVote | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getMasternodeYesNoVoteWithProofInfo(proTxHash, votePoll);
  }

  async masternodeVote(options: wasm.MasternodeVoteOptions): Promise<wasm.Vote> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.masternodeVote(options);
  }
}
