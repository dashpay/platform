import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import sinon from 'sinon';
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
    await client.group.infos('contract', { position: 10, included: true }, 5);
    await client.group.infosWithProof('contract');
    expect(wasmSdk.getGroupInfos).to.be.calledOnceWithExactly({
      dataContractId: 'contract',
      startAt: { position: 10, included: true },
      limit: 5,
    });
    expect(wasmSdk.getGroupInfosWithProofInfo).to.be.calledOnceWithExactly({
      dataContractId: 'contract',
    });
  });

  it('members() forwards list and optional filters', async () => {
    await client.group.members('contract', 1, { memberIds: ['a'], startAt: 's', limit: 2 });
    await client.group.membersWithProof('contract', 1);
    expect(wasmSdk.getGroupMembers).to.be.calledOnceWithExactly({
      dataContractId: 'contract',
      groupContractPosition: 1,
      memberIds: ['a'],
      startAtMemberId: 's',
      limit: 2,
    });
    expect(wasmSdk.getGroupMembersWithProofInfo).to.be.calledOnceWithExactly({
      dataContractId: 'contract',
      groupContractPosition: 1,
    });
  });

  it('identityGroups() forwards optional contract filters', async () => {
    await client.group.identityGroups('identity', {
      memberDataContracts: ['m'], ownerDataContracts: ['o'], moderatorDataContracts: ['d'],
    });
    await client.group.identityGroupsWithProof('identity');
    expect(wasmSdk.getIdentityGroups).to.be.calledOnceWithExactly(sinon.match({
      identityId: 'identity',
      memberDataContracts: ['m'],
      ownerDataContracts: ['o'],
      moderatorDataContracts: ['d'],
    }));
    expect(wasmSdk.getIdentityGroupsWithProofInfo).to.be.calledOnceWithExactly(sinon.match({
      identityId: 'identity',
    }));
  });

  it('group actions helpers forward to wasm', async () => {
    await client.group.actions('contract', 1, 'ACTIVE', {
      startAt: { actionId: 'cursor', included: true },
      limit: 3,
    });
    await client.group.actionsWithProof('contract', 1, 'CLOSED');
    await client.group.actionSigners('contract', 1, 'ACTIVE', 'action');
    await client.group.actionSignersWithProof('contract', 1, 'ACTIVE', 'action');
    expect(wasmSdk.getGroupActions).to.be.calledOnceWithExactly({
      dataContractId: 'contract',
      groupContractPosition: 1,
      status: 'ACTIVE',
      startAt: { actionId: 'cursor', included: true },
      limit: 3,
    });
    expect(wasmSdk.getGroupActionsWithProofInfo).to.be.calledOnceWithExactly({
      dataContractId: 'contract',
      groupContractPosition: 1,
      status: 'CLOSED',
    });
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
    await client.group.contestedResources({
      documentTypeName: 'dt', contractId: 'c', indexName: 'i', startAtValue: new Uint8Array([1]), limit: 2, orderAscending: false,
    });
    await client.group.contestedResourcesWithProof({ documentTypeName: 'dt', contractId: 'c', indexName: 'i' });
    await client.group.contestedResourceVotersForIdentity({
      contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v1'], contestantId: 'id', startAtVoterInfo: 's', limit: 3, orderAscending: true,
    });
    await client.group.contestedResourceVotersForIdentityWithProof({
      contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v2'], contestantId: 'id',
    });
    expect(wasmSdk.getContestedResources).to.be.calledOnceWithExactly({
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      startAtValue: new Uint8Array([1]),
      limit: 2,
      orderAscending: false,
    });
    expect(wasmSdk.getContestedResourcesWithProofInfo).to.be.calledOnceWithExactly({
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
    });
    expect(wasmSdk.getContestedResourceVotersForIdentity).to.be.calledOnceWithExactly({
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      indexValues: ['v1'],
      contestantId: 'id',
      startAtVoterId: 's',
      limit: 3,
      orderAscending: true,
    });
    expect(wasmSdk.getContestedResourceVotersForIdentityWithProofInfo).to.be.calledOnceWithExactly({
      dataContractId: 'c',
      documentTypeName: 'dt',
      indexName: 'i',
      indexValues: ['v2'],
      contestantId: 'id',
    });
  });
});
