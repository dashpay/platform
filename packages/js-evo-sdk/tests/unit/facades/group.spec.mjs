import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('GroupFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getGroupInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupInfoWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupInfos').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupInfosWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupMembers').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupMembersWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityGroups').resolves('ok');
    this.sinon.stub(wasmSdk, 'getIdentityGroupsWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupActions').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupActionsWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupActionSigners').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupActionSignersWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupsDataContracts').resolves('ok');
    this.sinon.stub(wasmSdk, 'getGroupsDataContractsWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResources').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourcesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceVotersForIdentity').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceVotersForIdentityWithProofInfo').resolves('ok');
  });

  it('info queries forward to wasm', async () => {
    await client.group.info('contract', 1);
    await client.group.infoWithProof('contract', 2);
    expect(wasmSdk.getGroupInfo).to.be.calledOnceWithExactly('contract', 1);
    expect(wasmSdk.getGroupInfoWithProofInfo).to.be.calledOnceWithExactly('contract', 2);
  });

  it('infos() forwards optional args with null defaults', async () => {
    const query = { dataContractId: 'contract', startAt: { position: 10, included: true }, limit: 5 };
    await client.group.infos(query);
    const proofQuery = { dataContractId: 'contract' };
    await client.group.infosWithProof(proofQuery);
    expect(wasmSdk.getGroupInfos).to.be.calledOnceWithExactly(query);
    expect(wasmSdk.getGroupInfosWithProofInfo).to.be.calledOnceWithExactly(proofQuery);
  });

  it('members() forwards list and optional filters', async () => {
    const query = {
      dataContractId: 'contract',
      groupContractPosition: 1,
      memberIds: ['a'],
      startAtMemberId: 's',
      limit: 2,
    };
    await client.group.members(query);
    const proofQuery = { dataContractId: 'contract', groupContractPosition: 1 };
    await client.group.membersWithProof(proofQuery);
    expect(wasmSdk.getGroupMembers).to.be.calledOnceWithExactly(query);
    expect(wasmSdk.getGroupMembersWithProofInfo).to.be.calledOnceWithExactly(proofQuery);
  });

  it('identityGroups() forwards optional contract filters', async () => {
    const query = {
      identityId: 'identity',
      memberDataContracts: ['m'],
      ownerDataContracts: ['o'],
      moderatorDataContracts: ['d'],
    };
    await client.group.identityGroups(query);
    const proofQuery = { identityId: 'identity' };
    await client.group.identityGroupsWithProof(proofQuery);
    expect(wasmSdk.getIdentityGroups).to.be.calledOnceWithExactly(query);
    expect(wasmSdk.getIdentityGroupsWithProofInfo).to.be.calledOnceWithExactly(proofQuery);
  });

  it('group actions helpers forward to wasm', async () => {
    const query = {
      dataContractId: 'contract',
      groupContractPosition: 1,
      status: 'ACTIVE',
      startAt: { actionId: 'cursor', included: true },
      limit: 3,
    };
    await client.group.actions(query);
    const proofQuery = {
      dataContractId: 'contract',
      groupContractPosition: 1,
      status: 'CLOSED',
    };
    await client.group.actionsWithProof(proofQuery);
    await client.group.actionSigners('contract', 1, 'ACTIVE', 'action');
    await client.group.actionSignersWithProof('contract', 1, 'ACTIVE', 'action');
    expect(wasmSdk.getGroupActions).to.be.calledOnceWithExactly(query);
    expect(wasmSdk.getGroupActionsWithProofInfo).to.be.calledOnceWithExactly(proofQuery);
    expect(wasmSdk.getGroupActionSigners).to.be.calledOnceWithExactly('contract', 1, 'ACTIVE', 'action');
    expect(wasmSdk.getGroupActionSignersWithProofInfo).to.be.calledOnceWithExactly('contract', 1, 'ACTIVE', 'action');
  });

  it('groupsDataContracts() forwards', async () => {
    await client.group.groupsDataContracts(['a', 'b']);
    await client.group.groupsDataContractsWithProof(['a']);
    expect(wasmSdk.getGroupsDataContracts).to.be.calledOnceWithExactly(['a', 'b']);
    expect(wasmSdk.getGroupsDataContractsWithProofInfo).to.be.calledOnceWithExactly(['a']);
  });

  it('forwards contestedResources and voters queries', async () => {
    const contestedQuery = {
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      startAtValue: new Uint8Array([1]),
      limit: 2,
      orderAscending: false,
    };
    await client.group.contestedResources(contestedQuery);
    const contestedProofQuery = {
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
    };
    await client.group.contestedResourcesWithProof(contestedProofQuery);
    const votersQuery = {
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      indexValues: ['v1'],
      contestantId: 'id',
      startAtVoterId: 's',
      limit: 3,
      orderAscending: true,
    };
    await client.group.contestedResourceVotersForIdentity(votersQuery);
    const votersProofQuery = {
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      indexValues: ['v2'],
      contestantId: 'id',
    };
    await client.group.contestedResourceVotersForIdentityWithProof(votersProofQuery);
    expect(wasmSdk.getContestedResources).to.be.calledOnceWithExactly(contestedQuery);
    expect(wasmSdk.getContestedResourcesWithProofInfo).to.be.calledOnceWithExactly(contestedProofQuery);
    expect(wasmSdk.getContestedResourceVotersForIdentity).to.be.calledOnceWithExactly(votersQuery);
    expect(wasmSdk.getContestedResourceVotersForIdentityWithProofInfo)
      .to.be.calledOnceWithExactly(votersProofQuery);
  });
});
