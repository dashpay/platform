import type {
  ContestedResourceVotersForIdentityQuery,
  GroupActionsQuery,
  GroupInfosQuery,
  GroupMembersQuery,
  IdentityGroupsQuery,
  VotePollsByDocumentTypeQuery,
} from '../wasm.js';
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

  async infos(query: GroupInfosQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfos(query);
  }

  async infosWithProof(query: GroupInfosQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfosWithProofInfo(query);
  }

  async members(query: GroupMembersQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupMembers(query);
  }

  async membersWithProof(query: GroupMembersQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupMembersWithProofInfo(query);
  }

  async identityGroups(query: IdentityGroupsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityGroups(query);
  }

  async identityGroupsWithProof(query: IdentityGroupsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityGroupsWithProofInfo(query);
  }

  async actions(query: GroupActionsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActions(query);
  }

  async actionsWithProof(query: GroupActionsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActionsWithProofInfo(query);
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

  async contestedResources(query: VotePollsByDocumentTypeQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResources(query);
  }

  async contestedResourcesWithProof(query: VotePollsByDocumentTypeQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourcesWithProofInfo(query);
  }

  async contestedResourceVotersForIdentity(query: ContestedResourceVotersForIdentityQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVotersForIdentity(query);
  }

  async contestedResourceVotersForIdentityWithProof(query: ContestedResourceVotersForIdentityQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVotersForIdentityWithProofInfo(query);
  }
}
