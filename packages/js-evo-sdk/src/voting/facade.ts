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

  async votePollsByEndDate(opts: { startTimeMs?: number | string | bigint | null; startTimeIncluded?: boolean; endTimeMs?: number | string | bigint | null; endTimeIncluded?: boolean; limit?: number; offset?: number; orderAscending?: boolean } = {}): Promise<any> {
    const { startTimeMs, startTimeIncluded, endTimeMs, endTimeIncluded, limit, offset, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();

    const normalizeTime = (value?: number | string | bigint | null) => {
      if (value === null || value === undefined) {
        return undefined;
      }

      if (typeof value === 'number') {
        return Number.isFinite(value) ? value : undefined;
      }

      if (typeof value === 'bigint') {
        return Number(value);
      }

      const trimmed = value.trim();
      if (!trimmed) {
        return undefined;
      }

      const parsed = Number(trimmed);
      return Number.isFinite(parsed) ? parsed : undefined;
    };

    const options: Record<string, unknown> = {};
    const start = normalizeTime(startTimeMs);
    if (start !== undefined) options.startTimeMs = start;
    if (startTimeIncluded !== undefined) options.startTimeIncluded = startTimeIncluded;

    const end = normalizeTime(endTimeMs);
    if (end !== undefined) options.endTimeMs = end;
    if (endTimeIncluded !== undefined) options.endTimeIncluded = endTimeIncluded;

    if (limit !== undefined) options.limit = limit;
    if (offset !== undefined) options.offset = offset;
    if (orderAscending !== undefined) options.orderAscending = orderAscending;

    return w.getVotePollsByEndDate(options);
  }

  async votePollsByEndDateWithProof(opts: { startTimeMs?: number | bigint | null; endTimeMs?: number | bigint | null; limit?: number; offset?: number; orderAscending?: boolean } = {}): Promise<any> {
    const { startTimeMs, endTimeMs, limit, offset, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();

    const normalizeTime = (value?: number | bigint | null) => {
      if (value === null || value === undefined) {
        return undefined;
      }

      if (typeof value === 'number') {
        return Number.isFinite(value) ? value : undefined;
      }

      return Number(value);
    };

    const options: Record<string, unknown> = {};
    const start = normalizeTime(startTimeMs);
    if (start !== undefined) options.startTimeMs = start;
    const end = normalizeTime(endTimeMs);
    if (end !== undefined) options.endTimeMs = end;
    if (limit !== undefined) options.limit = limit;
    if (offset !== undefined) options.offset = offset;
    if (orderAscending !== undefined) options.orderAscending = orderAscending;

    return w.getVotePollsByEndDateWithProofInfo(options);
  }

  async masternodeVote(args: { masternodeProTxHash: string; contractId: string; documentTypeName: string; indexName: string; indexValues: string | any[]; voteChoice: string; votingKeyWif: string }): Promise<any> {
    const { masternodeProTxHash, contractId, documentTypeName, indexName, indexValues, voteChoice, votingKeyWif } = args;
    const indexValuesStr = typeof indexValues === 'string' ? indexValues : asJsonString(indexValues)!;
    const w = await this.sdk.getWasmSdkConnected();
    return w.masternodeVote(masternodeProTxHash, contractId, documentTypeName, indexName, indexValuesStr, voteChoice, votingKeyWif);
  }
}
