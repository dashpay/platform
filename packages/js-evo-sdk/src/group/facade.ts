import type { EvoSDK } from '../sdk.js';

interface VotePollsByDocumentTypeQueryInput {
  dataContractId: string;
  documentTypeName: string;
  indexName: string;
  startIndexValues?: unknown[];
  endIndexValues?: unknown[];
  startAtValue?: unknown;
  startAtValueIncluded?: boolean;
  limit?: number;
  orderAscending?: boolean;
}

interface ContestedResourceVotersForIdentityQueryInput {
  dataContractId: string;
  documentTypeName: string;
  indexName: string;
  indexValues?: unknown[];
  contestantId: string;
  limit?: number;
  startAtVoterId?: string;
  startAtIncluded?: boolean;
  orderAscending?: boolean;
}

interface GroupActionsStartAt {
  actionId: string;
  included?: boolean;
}

interface GroupActionsQueryInput {
  dataContractId: string;
  groupContractPosition: number;
  status: 'ACTIVE' | 'CLOSED';
  startAt?: GroupActionsStartAt;
  limit?: number;
}

interface GroupInfosStartAt {
  position: number;
  included?: boolean;
}

interface GroupInfosQueryInput {
  dataContractId: string;
  startAt?: GroupInfosStartAt;
  limit?: number;
}

interface GroupMembersQueryInput {
  dataContractId: string;
  groupContractPosition: number;
  memberIds?: string[];
  startAtMemberId?: string;
  limit?: number;
}

interface IdentityGroupsQueryInput {
  identityId: string;
  memberDataContracts?: string[];
  ownerDataContracts?: string[];
  moderatorDataContracts?: string[];
}

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

  async infos(contractId: string, startAt?: GroupInfosStartAt, limit?: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    const query: GroupInfosQueryInput = {
      dataContractId: contractId,
    };
    if (startAt !== undefined) query.startAt = startAt;
    if (limit !== undefined) query.limit = limit;
    return w.getGroupInfos(query);
  }

  async infosWithProof(contractId: string, startAt?: GroupInfosStartAt, limit?: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    const query: GroupInfosQueryInput = {
      dataContractId: contractId,
    };
    if (startAt !== undefined) query.startAt = startAt;
    if (limit !== undefined) query.limit = limit;
    return w.getGroupInfosWithProofInfo(query);
  }

  async members(contractId: string, groupContractPosition: number, opts: { memberIds?: string[]; startAt?: string; limit?: number } = {}): Promise<any> {
    const { memberIds, startAt, limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    const query: GroupMembersQueryInput = {
      dataContractId: contractId,
      groupContractPosition,
    };
    if (memberIds !== undefined) query.memberIds = memberIds;
    if (startAt !== undefined) query.startAtMemberId = startAt;
    if (limit !== undefined) query.limit = limit;
    return w.getGroupMembers(query);
  }

  async membersWithProof(contractId: string, groupContractPosition: number, opts: { memberIds?: string[]; startAt?: string; limit?: number } = {}): Promise<any> {
    const { memberIds, startAt, limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    const query: GroupMembersQueryInput = {
      dataContractId: contractId,
      groupContractPosition,
    };
    if (memberIds !== undefined) query.memberIds = memberIds;
    if (startAt !== undefined) query.startAtMemberId = startAt;
    if (limit !== undefined) query.limit = limit;
    return w.getGroupMembersWithProofInfo(query);
  }

  async identityGroups(identityId: string, opts: { memberDataContracts?: string[]; ownerDataContracts?: string[]; moderatorDataContracts?: string[] } = {}): Promise<any> {
    const { memberDataContracts, ownerDataContracts, moderatorDataContracts } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityGroups({
      identityId,
      memberDataContracts,
      ownerDataContracts,
      moderatorDataContracts,
    });
  }

  async identityGroupsWithProof(identityId: string, opts: { memberDataContracts?: string[]; ownerDataContracts?: string[]; moderatorDataContracts?: string[] } = {}): Promise<any> {
    const { memberDataContracts, ownerDataContracts, moderatorDataContracts } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityGroupsWithProofInfo({
      identityId,
      memberDataContracts,
      ownerDataContracts,
      moderatorDataContracts,
    });
  }

  async actions(contractId: string, groupContractPosition: number, status: 'ACTIVE' | 'CLOSED', opts: { startAt?: GroupActionsStartAt; limit?: number } = {}): Promise<any> {
    const { startAt, limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    const query: GroupActionsQueryInput = {
      dataContractId: contractId,
      groupContractPosition,
      status,
    };
    if (startAt !== undefined) query.startAt = startAt;
    if (limit !== undefined) query.limit = limit;
    return w.getGroupActions(query);
  }

  async actionsWithProof(contractId: string, groupContractPosition: number, status: 'ACTIVE' | 'CLOSED', opts: { startAt?: GroupActionsStartAt; limit?: number } = {}): Promise<any> {
    const { startAt, limit } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    const query: GroupActionsQueryInput = {
      dataContractId: contractId,
      groupContractPosition,
      status,
    };
    if (startAt !== undefined) query.startAt = startAt;
    if (limit !== undefined) query.limit = limit;
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

  async contestedResources(params: { documentTypeName: string; contractId: string; indexName: string; startAtValue?: Uint8Array; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { documentTypeName, contractId, indexName, startAtValue, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    const query: VotePollsByDocumentTypeQueryInput = {
      dataContractId: contractId,
      documentTypeName,
      indexName,
    };
    if (startAtValue !== undefined) query.startAtValue = startAtValue;
    if (limit !== undefined) query.limit = limit;
    if (orderAscending !== undefined) query.orderAscending = orderAscending;
    return w.getContestedResources(query);
  }

  async contestedResourcesWithProof(params: { documentTypeName: string; contractId: string; indexName: string; startAtValue?: Uint8Array; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { documentTypeName, contractId, indexName, startAtValue, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    const query: VotePollsByDocumentTypeQueryInput = {
      dataContractId: contractId,
      documentTypeName,
      indexName,
    };
    if (startAtValue !== undefined) query.startAtValue = startAtValue;
    if (limit !== undefined) query.limit = limit;
    if (orderAscending !== undefined) query.orderAscending = orderAscending;
    return w.getContestedResourcesWithProofInfo(query);
  }

  async contestedResourceVotersForIdentity(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; contestantId: string; startAtVoterInfo?: string; limit?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, contestantId, startAtVoterInfo, limit, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    const query: ContestedResourceVotersForIdentityQueryInput = {
      dataContractId: contractId,
      documentTypeName,
      indexName,
      indexValues,
      contestantId,
    };
    if (startAtVoterInfo !== undefined) query.startAtVoterId = startAtVoterInfo;
    if (limit !== undefined) query.limit = limit;
    if (orderAscending !== undefined) query.orderAscending = orderAscending;
    return w.getContestedResourceVotersForIdentity(query);
  }

  async contestedResourceVotersForIdentityWithProof(params: { contractId: string; documentTypeName: string; indexName: string; indexValues: any[]; contestantId: string; startAtIdentifierInfo?: string; count?: number; orderAscending?: boolean }): Promise<any> {
    const { contractId, documentTypeName, indexName, indexValues, contestantId, startAtIdentifierInfo, count, orderAscending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    const query: ContestedResourceVotersForIdentityQueryInput = {
      dataContractId: contractId,
      documentTypeName,
      indexName,
      indexValues,
      contestantId,
    };
    if (startAtIdentifierInfo !== undefined) query.startAtVoterId = startAtIdentifierInfo;
    if (count !== undefined) query.limit = count;
    if (orderAscending !== undefined) query.orderAscending = orderAscending;
    return w.getContestedResourceVotersForIdentityWithProofInfo(query);
  }
}
