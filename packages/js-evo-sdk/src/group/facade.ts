import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class GroupFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async info(contractId: wasm.IdentifierLike, groupContractPosition: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfo(contractId, groupContractPosition);
  }

  async infoWithProof(contractId: wasm.IdentifierLike, groupContractPosition: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfoWithProofInfo(contractId, groupContractPosition);
  }

  async infos(query: wasm.GroupInfosQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfos(query);
  }

  async infosWithProof(query: wasm.GroupInfosQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupInfosWithProofInfo(query);
  }

  async members(query: wasm.GroupMembersQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupMembers(query);
  }

  async membersWithProof(query: wasm.GroupMembersQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupMembersWithProofInfo(query);
  }

  async identityGroups(query: wasm.IdentityGroupsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityGroups(query);
  }

  async identityGroupsWithProof(query: wasm.IdentityGroupsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityGroupsWithProofInfo(query);
  }

  async actions(query: wasm.GroupActionsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActions(query);
  }

  async actionsWithProof(query: wasm.GroupActionsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActionsWithProofInfo(query);
  }

  async actionSigners(query: wasm.GroupActionSignersQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActionSigners(query);
  }

  async actionSignersWithProof(query: wasm.GroupActionSignersQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupActionSignersWithProofInfo(query);
  }

  async groupsDataContracts(dataContractIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupsDataContracts(dataContractIds);
  }

  async groupsDataContractsWithProof(dataContractIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getGroupsDataContractsWithProofInfo(dataContractIds);
  }

  async contestedResources(query: wasm.VotePollsByDocumentTypeQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResources(query);
  }

  async contestedResourcesWithProof(query: wasm.VotePollsByDocumentTypeQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourcesWithProofInfo(query);
  }

  async contestedResourceVotersForIdentity(query: wasm.ContestedResourceVotersForIdentityQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVotersForIdentity(query);
  }

  async contestedResourceVotersForIdentityWithProof(query: wasm.ContestedResourceVotersForIdentityQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestedResourceVotersForIdentityWithProofInfo(query);
  }
}
