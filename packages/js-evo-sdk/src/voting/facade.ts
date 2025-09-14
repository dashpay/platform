import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class VotingFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async contestedResourceVoteState(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; resultType: string; allowIncludeLockedAndAbstainingVoteTally?: boolean; startAtIdentifierInfo?: string; count?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally, startAtIdentifierInfo, count, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVoteState(contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally ?? null, startAtIdentifierInfo ?? null, count ?? null, orderAscending ?? null);
  }

  async contestedResourceVoteStateWithProof(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; resultType: string; allowIncludeLockedAndAbstainingVoteTally?: boolean; startAtIdentifierInfo?: string; count?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally, startAtIdentifierInfo, count, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVoteStateWithProofInfo(contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally ?? null, startAtIdentifierInfo ?? null, count ?? null, orderAscending ?? null);
  }

  async contestedResourceIdentityVotes(identityId: string, opts: { limit?: number; startAtVotePollIdInfo?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAtVotePollIdInfo, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceIdentityVotes(identityId, limit ?? null, startAtVotePollIdInfo ?? null, orderAscending ?? null);
  }

  async contestedResourceIdentityVotesWithProof(identityId: string, opts: { limit?: number; offset?: number; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, offset, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceIdentityVotesWithProofInfo(identityId, limit ?? null, offset ?? null, orderAscending ?? null);
  }

  async votePollsByEndDate(opts: { startTimeInfo?: string; endTimeInfo?: string; limit?: number; orderAscending?: boolean } = {}): Promise<any> {
    const { startTimeInfo, endTimeInfo, limit, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getVotePollsByEndDate(startTimeInfo ?? null, endTimeInfo ?? null, limit ?? null, orderAscending ?? null);
  }

  async votePollsByEndDateWithProof(opts: { startTimeMs?: number | bigint | null; endTimeMs?: number | bigint | null; limit?: number; offset?: number; orderAscending?: boolean } = {}): Promise<any> {
    const { startTimeMs, endTimeMs, limit, offset, orderAscending } = opts;
    const start = startTimeMs != null ? BigInt(startTimeMs) : null;
    const end = endTimeMs != null ? BigInt(endTimeMs) : null;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getVotePollsByEndDateWithProofInfo(start ?? null, end ?? null, limit ?? null, offset ?? null, orderAscending ?? null);
  }

  async masternodeVote(args: { masternodeProTxHash: string; contractId: string; documentTypeName: string; indexName: string; indexValues: string | any[]; voteChoice: string; votingKeyWif: string }): Promise<any> {
    const { masternodeProTxHash, contractId, documentTypeName, indexName, indexValues, voteChoice, votingKeyWif } = args;
    const indexValuesStr = typeof indexValues === 'string' ? indexValues : asJsonString(indexValues)!;
    const w = await this.sdk.getWasmSdkConnected();
    return w.masternodeVote(masternodeProTxHash, contractId, documentTypeName, indexName, indexValuesStr, voteChoice, votingKeyWif);
  }
}
