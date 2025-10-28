import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

interface ContestedResourceVoteStateQueryInput {
  dataContractId: string;
  documentTypeName: string;
  indexName: string;
  indexValues?: unknown[];
  resultType?: 'documents' | 'voteTally' | 'documentsAndVoteTally';
  limit?: number;
  startAtContenderId?: string;
  startAtIncluded?: boolean;
  includeLockedAndAbstaining?: boolean;
}

interface ContestedResourceIdentityVotesQueryInput {
  identityId: string;
  limit?: number;
  startAtVoteId?: string;
  startAtIncluded?: boolean;
  orderAscending?: boolean;
}

interface VotePollsByEndDateQueryInput {
  startTimeMs?: number;
  startTimeIncluded?: boolean;
  endTimeMs?: number;
  endTimeIncluded?: boolean;
  limit?: number;
  offset?: number;
  orderAscending?: boolean;
}

export class VotingFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async contestedResourceVoteState(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; resultType?: string; allowIncludeLockedAndAbstainingVoteTally?: boolean; startAtIdentifierInfo?: string; startAtIncluded?: boolean; count?: number }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally, startAtIdentifierInfo, startAtIncluded, count } = params;
    const w = await this.sdk.getWasmSdkConnected();
    const query: ContestedResourceVoteStateQueryInput = {
      dataContractId: contractId,
      documentTypeName,
      indexName,
      indexValues,
    };
    if (resultType !== undefined) query.resultType = resultType as ContestedResourceVoteStateQueryInput['resultType'];
    if (allowIncludeLockedAndAbstainingVoteTally !== undefined) {
      query.includeLockedAndAbstaining = allowIncludeLockedAndAbstainingVoteTally;
    }
    if (startAtIdentifierInfo !== undefined) query.startAtContenderId = startAtIdentifierInfo;
    if (startAtIncluded !== undefined) query.startAtIncluded = startAtIncluded;
    if (count !== undefined) query.limit = count;
    return w.getContestedResourceVoteState(query);
  }

  async contestedResourceVoteStateWithProof(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; resultType?: string; allowIncludeLockedAndAbstainingVoteTally?: boolean; startAtIdentifierInfo?: string; startAtIncluded?: boolean; count?: number }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally, startAtIdentifierInfo, startAtIncluded, count } = params;
    const w = await this.sdk.getWasmSdkConnected();
    const query: ContestedResourceVoteStateQueryInput = {
      dataContractId: contractId,
      documentTypeName,
      indexName,
      indexValues,
    };
    if (resultType !== undefined) query.resultType = resultType as ContestedResourceVoteStateQueryInput['resultType'];
    if (allowIncludeLockedAndAbstainingVoteTally !== undefined) {
      query.includeLockedAndAbstaining = allowIncludeLockedAndAbstainingVoteTally;
    }
    if (startAtIdentifierInfo !== undefined) query.startAtContenderId = startAtIdentifierInfo;
    if (startAtIncluded !== undefined) query.startAtIncluded = startAtIncluded;
    if (count !== undefined) query.limit = count;
    return w.getContestedResourceVoteStateWithProofInfo(query);
  }

  async contestedResourceIdentityVotes(identityId: string, opts: { limit?: number; startAtVotePollIdInfo?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAtVotePollIdInfo, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    const query: ContestedResourceIdentityVotesQueryInput = { identityId };
    if (limit !== undefined) query.limit = limit;
    if (startAtVotePollIdInfo !== undefined) query.startAtVoteId = startAtVotePollIdInfo;
    if (orderAscending !== undefined) query.orderAscending = orderAscending;
    return w.getContestedResourceIdentityVotes(query);
  }

  async contestedResourceIdentityVotesWithProof(identityId: string, opts: { limit?: number; startAtVotePollIdInfo?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAtVotePollIdInfo, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    const query: ContestedResourceIdentityVotesQueryInput = { identityId };
    if (limit !== undefined) query.limit = limit;
    if (startAtVotePollIdInfo !== undefined) query.startAtVoteId = startAtVotePollIdInfo;
    if (orderAscending !== undefined) query.orderAscending = orderAscending;
    return w.getContestedResourceIdentityVotesWithProofInfo(query);
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

    const query: VotePollsByEndDateQueryInput = {};
    const start = normalizeTime(startTimeMs);
    if (start !== undefined) query.startTimeMs = start;
    if (startTimeIncluded !== undefined) query.startTimeIncluded = startTimeIncluded;

    const end = normalizeTime(endTimeMs);
    if (end !== undefined) query.endTimeMs = end;
    if (endTimeIncluded !== undefined) query.endTimeIncluded = endTimeIncluded;

    if (limit !== undefined) query.limit = limit;
    if (offset !== undefined) query.offset = offset;
    if (orderAscending !== undefined) query.orderAscending = orderAscending;

    return w.getVotePollsByEndDate(query);
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

    const query: VotePollsByEndDateQueryInput = {};
    const start = normalizeTime(startTimeMs);
    if (start !== undefined) query.startTimeMs = start;
    const end = normalizeTime(endTimeMs);
    if (end !== undefined) query.endTimeMs = end;
    if (limit !== undefined) query.limit = limit;
    if (offset !== undefined) query.offset = offset;
    if (orderAscending !== undefined) query.orderAscending = orderAscending;

    return w.getVotePollsByEndDateWithProofInfo(query);
  }

  async masternodeVote(args: { masternodeProTxHash: string; contractId: string; documentTypeName: string; indexName: string; indexValues: string | any[]; voteChoice: string; votingKeyWif: string }): Promise<any> {
    const { masternodeProTxHash, contractId, documentTypeName, indexName, indexValues, voteChoice, votingKeyWif } = args;
    const indexValuesStr = typeof indexValues === 'string' ? indexValues : asJsonString(indexValues)!;
    const w = await this.sdk.getWasmSdkConnected();
    return w.masternodeVote(masternodeProTxHash, contractId, documentTypeName, indexName, indexValuesStr, voteChoice, votingKeyWif);
  }
}
