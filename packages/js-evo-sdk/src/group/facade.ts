import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class GroupFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async contestedResources(params: { documentTypeName: string; contractId: string; indexName: string; startAtValue?: Uint8Array; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { documentTypeName, contractId, indexName, startAtValue, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_contested_resources(w, documentTypeName, contractId, indexName, startAtValue ?? null, limit ?? null, null, orderAscending ?? null);
  }

  async contestedResourcesWithProof(params: { documentTypeName: string; contractId: string; indexName: string; startAtValue?: Uint8Array; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { documentTypeName, contractId, indexName, startAtValue, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_contested_resources_with_proof_info(w, documentTypeName, contractId, indexName, startAtValue ?? null, limit ?? null, null, orderAscending ?? null);
  }

  async contestedResourceVotersForIdentity(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; contestantId: string; startAtVoterInfo?: string; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, contestantId, startAtVoterInfo, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_contested_resource_voters_for_identity(w, contractId, documentTypeName, indexName, indexValues, contestantId, startAtVoterInfo ?? null, limit ?? null, orderAscending ?? null);
  }

  async contestedResourceVotersForIdentityWithProof(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; contestantId: string; startAtIdentifierInfo?: string; count?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, contestantId, startAtIdentifierInfo, count, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_contested_resource_voters_for_identity_with_proof_info(w, contractId, documentTypeName, indexName, indexValues, contestantId, startAtIdentifierInfo ?? null, count ?? null, orderAscending ?? null);
  }
}
