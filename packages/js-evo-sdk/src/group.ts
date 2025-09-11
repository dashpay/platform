import * as wasm from './wasm';

export class GroupFacade {
  private _sdk: wasm.WasmSdk;
  constructor(sdk: wasm.WasmSdk) { this._sdk = sdk; }

  contestedResources(params: { documentTypeName: string; contractId: string; indexName: string; startAtValue?: Uint8Array; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { documentTypeName, contractId, indexName, startAtValue, limit, orderAscending } = params;
    return wasm.get_contested_resources(this._sdk, documentTypeName, contractId, indexName, startAtValue ?? null, limit ?? null, null, orderAscending ?? null);
  }

  contestedResourcesWithProof(params: { documentTypeName: string; contractId: string; indexName: string; startAtValue?: Uint8Array; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { documentTypeName, contractId, indexName, startAtValue, limit, orderAscending } = params;
    return wasm.get_contested_resources_with_proof_info(this._sdk, documentTypeName, contractId, indexName, startAtValue ?? null, limit ?? null, null, orderAscending ?? null);
  }

  contestedResourceVotersForIdentity(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; contestantId: string; startAtVoterInfo?: string; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, contestantId, startAtVoterInfo, limit, orderAscending } = params;
    return wasm.get_contested_resource_voters_for_identity(this._sdk, contractId, documentTypeName, indexName, indexValues, contestantId, startAtVoterInfo ?? null, limit ?? null, orderAscending ?? null);
  }

  contestedResourceVotersForIdentityWithProof(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; contestantId: string; startAtIdentifierInfo?: string; count?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, contestantId, startAtIdentifierInfo, count, orderAscending } = params;
    return wasm.get_contested_resource_voters_for_identity_with_proof_info(this._sdk, contractId, documentTypeName, indexName, indexValues, contestantId, startAtIdentifierInfo ?? null, count ?? null, orderAscending ?? null);
  }
}
