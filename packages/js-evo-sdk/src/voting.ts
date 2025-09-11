import * as wasm from './wasm';
import { asJsonString } from './util';

export class VotingFacade {
  private _sdk: wasm.WasmSdk;
  constructor(sdk: wasm.WasmSdk) { this._sdk = sdk; }

  contestedResourceVoteState(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; resultType: string; allowIncludeLockedAndAbstainingVoteTally?: boolean; startAtIdentifierInfo?: string; count?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally, startAtIdentifierInfo, count, orderAscending } = params;
    return wasm.get_contested_resource_vote_state(this._sdk, contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally ?? null, startAtIdentifierInfo ?? null, count ?? null, orderAscending ?? null);
  }

  contestedResourceVoteStateWithProof(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; resultType: string; allowIncludeLockedAndAbstainingVoteTally?: boolean; startAtIdentifierInfo?: string; count?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally, startAtIdentifierInfo, count, orderAscending } = params;
    return wasm.get_contested_resource_vote_state_with_proof_info(this._sdk, contractId, documentTypeName, indexName, indexValues, resultType, allowIncludeLockedAndAbstainingVoteTally ?? null, startAtIdentifierInfo ?? null, count ?? null, orderAscending ?? null);
  }

  contestedResourceIdentityVotes(identityId: string, opts: { limit?: number; startAtVotePollIdInfo?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAtVotePollIdInfo, orderAscending } = opts;
    return wasm.get_contested_resource_identity_votes(this._sdk, identityId, limit ?? null, startAtVotePollIdInfo ?? null, orderAscending ?? null);
  }

  contestedResourceIdentityVotesWithProof(identityId: string, opts: { limit?: number; offset?: number; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, offset, orderAscending } = opts;
    return wasm.get_contested_resource_identity_votes_with_proof_info(this._sdk, identityId, limit ?? null, offset ?? null, orderAscending ?? null);
  }

  votePollsByEndDate(opts: { startTimeInfo?: string; endTimeInfo?: string; limit?: number; orderAscending?: boolean } = {}): Promise<any> {
    const { startTimeInfo, endTimeInfo, limit, orderAscending } = opts;
    return wasm.get_vote_polls_by_end_date(this._sdk, startTimeInfo ?? null, endTimeInfo ?? null, limit ?? null, orderAscending ?? null);
  }

  votePollsByEndDateWithProof(opts: { startTimeMs?: number | bigint | null; endTimeMs?: number | bigint | null; limit?: number; offset?: number; orderAscending?: boolean } = {}): Promise<any> {
    const { startTimeMs, endTimeMs, limit, offset, orderAscending } = opts;
    return wasm.get_vote_polls_by_end_date_with_proof_info(this._sdk, startTimeMs ?? null, endTimeMs ?? null, limit ?? null, offset ?? null, orderAscending ?? null);
  }

  masternodeVote(args: { masternodeProTxHash: string; contractId: string; documentTypeName: string; indexName: string; indexValues: string | any[]; voteChoice: string; votingKeyWif: string }): Promise<any> {
    const { masternodeProTxHash, contractId, documentTypeName, indexName, indexValues, voteChoice, votingKeyWif } = args;
    const indexValuesStr = typeof indexValues === 'string' ? indexValues : asJsonString(indexValues)!;
    return this._sdk.masternodeVote(masternodeProTxHash, contractId, documentTypeName, indexName, indexValuesStr, voteChoice, votingKeyWif);
  }
}
