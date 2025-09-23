import type { EvoSDK } from '../sdk.js';

export class GroupFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async info(contractId: string, groupContractPosition: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfo(contractId, groupContractPosition);
  }

  async infoWithProof(contractId: string, groupContractPosition: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfoWithProofInfo(contractId, groupContractPosition);
  }

  async infos(contractId: string, startAtInfo?: unknown, count?: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfos(contractId, startAtInfo ?? null, count ?? null);
  }

  async infosWithProof(contractId: string, startAtInfo?: unknown, count?: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfosWithProofInfo(contractId, startAtInfo ?? null, count ?? null);
  }

  async members(contractId: string, groupContractPosition: number, opts: { memberIds?: string[]; startAt?: string; limit?: number } = {}): Promise<any> {
    const { memberIds, startAt, limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupMembers(contractId, groupContractPosition, memberIds ?? null, startAt ?? null, limit ?? null);
  }

  async membersWithProof(contractId: string, groupContractPosition: number, opts: { memberIds?: string[]; startAt?: string; limit?: number } = {}): Promise<any> {
    const { memberIds, startAt, limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupMembersWithProofInfo(contractId, groupContractPosition, memberIds ?? null, startAt ?? null, limit ?? null);
  }

  async identityGroups(identityId: string, opts: { memberDataContracts?: string[]; ownerDataContracts?: string[]; moderatorDataContracts?: string[] } = {}): Promise<any> {
    const { memberDataContracts, ownerDataContracts, moderatorDataContracts } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityGroups(
      identityId,
      memberDataContracts ?? null,
      ownerDataContracts ?? null,
      moderatorDataContracts ?? null,
    );
  }

  async identityGroupsWithProof(identityId: string, opts: { memberDataContracts?: string[]; ownerDataContracts?: string[]; moderatorDataContracts?: string[] } = {}): Promise<any> {
    const { memberDataContracts, ownerDataContracts, moderatorDataContracts } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityGroupsWithProofInfo(
      identityId,
      memberDataContracts ?? null,
      ownerDataContracts ?? null,
      moderatorDataContracts ?? null,
    );
  }

  async actions(contractId: string, groupContractPosition: number, status: string, opts: { startAtInfo?: unknown; count?: number } = {}): Promise<any> {
    const { startAtInfo, count } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActions(contractId, groupContractPosition, status, startAtInfo ?? null, count ?? null);
  }

  async actionsWithProof(contractId: string, groupContractPosition: number, status: string, opts: { startAtInfo?: unknown; count?: number } = {}): Promise<any> {
    const { startAtInfo, count } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActionsWithProofInfo(contractId, groupContractPosition, status, startAtInfo ?? null, count ?? null);
  }

  async actionSigners(contractId: string, groupContractPosition: number, status: string, actionId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActionSigners(contractId, groupContractPosition, status, actionId);
  }

  async actionSignersWithProof(contractId: string, groupContractPosition: number, status: string, actionId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActionSignersWithProofInfo(contractId, groupContractPosition, status, actionId);
  }

  async groupsDataContracts(dataContractIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupsDataContracts(dataContractIds);
  }

  async groupsDataContractsWithProof(dataContractIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupsDataContractsWithProofInfo(dataContractIds);
  }

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
