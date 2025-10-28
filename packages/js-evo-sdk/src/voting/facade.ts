import type {
  ContestedResourceIdentityVotesQuery,
  ContestedResourceVoteStateQuery,
  VotePollsByEndDateQuery,
} from '@dashevo/wasm-sdk';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class VotingFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async contestedResourceVoteState(query: ContestedResourceVoteStateQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVoteState(query);
  }

  async contestedResourceVoteStateWithProof(query: ContestedResourceVoteStateQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVoteStateWithProofInfo(query);
  }

  async contestedResourceIdentityVotes(query: ContestedResourceIdentityVotesQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceIdentityVotes(query);
  }

  async contestedResourceIdentityVotesWithProof(query: ContestedResourceIdentityVotesQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceIdentityVotesWithProofInfo(query);
  }

  async votePollsByEndDate(query?: VotePollsByEndDateQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();

    return w.getVotePollsByEndDate(query ?? null);
  }

  async votePollsByEndDateWithProof(query?: VotePollsByEndDateQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();

    return w.getVotePollsByEndDateWithProofInfo(query ?? null);
  }

  async masternodeVote(args: { masternodeProTxHash: string; contractId: string; documentTypeName: string; indexName: string; indexValues: string | any[]; voteChoice: string; votingKeyWif: string }): Promise<any> {
    const { masternodeProTxHash, contractId, documentTypeName, indexName, indexValues, voteChoice, votingKeyWif } = args;
    const indexValuesStr = typeof indexValues === 'string' ? indexValues : asJsonString(indexValues)!;
    const w = await this.sdk.getWasmSdkConnected();
    return w.masternodeVote(masternodeProTxHash, contractId, documentTypeName, indexName, indexValuesStr, voteChoice, votingKeyWif);
  }
}
