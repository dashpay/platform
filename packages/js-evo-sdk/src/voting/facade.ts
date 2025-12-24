import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class VotingFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async contestedResourceVoteState(query: wasm.ContestedResourceVoteStateQuery): Promise<wasm.ContestedResourceVoteState> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVoteState(query);
  }

  async contestedResourceVoteStateWithProof(query: wasm.ContestedResourceVoteStateQuery): Promise<wasm.ProofMetadataResponseTyped<wasm.ContestedResourceVoteState>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVoteStateWithProofInfo(query);
  }

  async contestedResourceIdentityVotes(query: wasm.ContestedResourceIdentityVotesQuery): Promise<Map<wasm.Identifier, wasm.ResourceVote>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceIdentityVotes(query);
  }

  async contestedResourceIdentityVotesWithProof(query: wasm.ContestedResourceIdentityVotesQuery): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, wasm.ResourceVote>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceIdentityVotesWithProofInfo(query);
  }

  async votePollsByEndDate(query?: wasm.VotePollsByEndDateQuery): Promise<wasm.VotePollsByEndDateEntry[]> {
    const w = await this.sdk.getWasmSdkConnected();

    return w.getVotePollsByEndDate(query ?? null);
  }

  async votePollsByEndDateWithProof(query?: wasm.VotePollsByEndDateQuery): Promise<wasm.ProofMetadataResponseTyped<wasm.VotePollsByEndDateEntry[]>> {
    const w = await this.sdk.getWasmSdkConnected();

    return w.getVotePollsByEndDateWithProofInfo(query ?? null);
  }

  async masternodeVote(args: { masternodeProTxHash: string; contractId: wasm.IdentifierLike; documentTypeName: string; indexName: string; indexValues: string | any[]; voteChoice: string; votingKeyWif: string }): Promise<any> {
    const { masternodeProTxHash, contractId, documentTypeName, indexName, indexValues, voteChoice, votingKeyWif } = args;
    const indexValuesStr = typeof indexValues === 'string' ? indexValues : asJsonString(indexValues)!;
    const w = await this.sdk.getWasmSdkConnected();
    return w.masternodeVote(masternodeProTxHash, contractId, documentTypeName, indexName, indexValuesStr, voteChoice, votingKeyWif);
  }
}
