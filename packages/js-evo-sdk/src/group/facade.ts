import type { EvoSDK } from '../sdk.js';

export class GroupFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async contestedResources(params: { documentTypeName: string; contractId: string; indexName: string; startAtValue?: Uint8Array; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { documentTypeName, contractId, indexName, startAtValue, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResources(documentTypeName, contractId, indexName, startAtValue ?? null, limit ?? null, null, orderAscending ?? null);
  }

  async contestedResourcesWithProof(params: { documentTypeName: string; contractId: string; indexName: string; startAtValue?: Uint8Array; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { documentTypeName, contractId, indexName, startAtValue, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourcesWithProofInfo(documentTypeName, contractId, indexName, startAtValue ?? null, limit ?? null, null, orderAscending ?? null);
  }

  async contestedResourceVotersForIdentity(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; contestantId: string; startAtVoterInfo?: string; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, contestantId, startAtVoterInfo, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVotersForIdentity(contractId, documentTypeName, indexName, indexValues, contestantId, startAtVoterInfo ?? null, limit ?? null, orderAscending ?? null);
  }

  async contestedResourceVotersForIdentityWithProof(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; contestantId: string; startAtIdentifierInfo?: string; count?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, contestantId, startAtIdentifierInfo, count, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVotersForIdentityWithProofInfo(contractId, documentTypeName, indexName, indexValues, contestantId, startAtIdentifierInfo ?? null, count ?? null, orderAscending ?? null);
  }
}
