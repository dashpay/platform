import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('GroupFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  // Stub references for type-safe assertions
  let getGroupInfoStub: SinonStub;
  let getGroupInfoWithProofInfoStub: SinonStub;
  let getGroupInfosStub: SinonStub;
  let getGroupInfosWithProofInfoStub: SinonStub;
  let getGroupMembersStub: SinonStub;
  let getGroupMembersWithProofInfoStub: SinonStub;
  let getIdentityGroupsStub: SinonStub;
  let getIdentityGroupsWithProofInfoStub: SinonStub;
  let getGroupActionsStub: SinonStub;
  let getGroupActionsWithProofInfoStub: SinonStub;
  let getGroupActionSignersStub: SinonStub;
  let getGroupActionSignersWithProofInfoStub: SinonStub;
  let getGroupsDataContractsStub: SinonStub;
  let getGroupsDataContractsWithProofInfoStub: SinonStub;
  let getContestedResourcesStub: SinonStub;
  let getContestedResourcesWithProofInfoStub: SinonStub;
  let getContestedResourceVotersForIdentityStub: SinonStub;
  let getContestedResourceVotersForIdentityWithProofInfoStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getGroupInfoStub = this.sinon.stub(wasmSdk, 'getGroupInfo').resolves('ok');
    getGroupInfoWithProofInfoStub = this.sinon.stub(wasmSdk, 'getGroupInfoWithProofInfo').resolves('ok');
    getGroupInfosStub = this.sinon.stub(wasmSdk, 'getGroupInfos').resolves('ok');
    getGroupInfosWithProofInfoStub = this.sinon.stub(wasmSdk, 'getGroupInfosWithProofInfo').resolves('ok');
    getGroupMembersStub = this.sinon.stub(wasmSdk, 'getGroupMembers').resolves('ok');
    getGroupMembersWithProofInfoStub = this.sinon.stub(wasmSdk, 'getGroupMembersWithProofInfo').resolves('ok');
    getIdentityGroupsStub = this.sinon.stub(wasmSdk, 'getIdentityGroups').resolves('ok');
    getIdentityGroupsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getIdentityGroupsWithProofInfo').resolves('ok');
    getGroupActionsStub = this.sinon.stub(wasmSdk, 'getGroupActions').resolves('ok');
    getGroupActionsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getGroupActionsWithProofInfo').resolves('ok');
    getGroupActionSignersStub = this.sinon.stub(wasmSdk, 'getGroupActionSigners').resolves('ok');
    getGroupActionSignersWithProofInfoStub = this.sinon.stub(wasmSdk, 'getGroupActionSignersWithProofInfo').resolves('ok');
    getGroupsDataContractsStub = this.sinon.stub(wasmSdk, 'getGroupsDataContracts').resolves('ok');
    getGroupsDataContractsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getGroupsDataContractsWithProofInfo').resolves('ok');
    getContestedResourcesStub = this.sinon.stub(wasmSdk, 'getContestedResources').resolves('ok');
    getContestedResourcesWithProofInfoStub = this.sinon.stub(wasmSdk, 'getContestedResourcesWithProofInfo').resolves('ok');
    getContestedResourceVotersForIdentityStub = this.sinon.stub(wasmSdk, 'getContestedResourceVotersForIdentity').resolves('ok');
    getContestedResourceVotersForIdentityWithProofInfoStub = this.sinon.stub(wasmSdk, 'getContestedResourceVotersForIdentityWithProofInfo').resolves('ok');
  });

  describe('info()', () => {
    it('should forward contract ID and position to wasm', async () => {
      await client.group.info('contract', 1);
      expect(getGroupInfoStub).to.be.calledOnceWithExactly('contract', 1);
    });
  });

  describe('infoWithProof()', () => {
    it('should forward contract ID and position to wasm', async () => {
      await client.group.infoWithProof('contract', 2);
      expect(getGroupInfoWithProofInfoStub).to.be.calledOnceWithExactly('contract', 2);
    });
  });

  describe('infos()', () => {
    it('should forward query with optional args to wasm', async () => {
      const query = { dataContractId: 'contract', startAt: { position: 10, included: true }, limit: 5 };
      await client.group.infos(query);
      expect(getGroupInfosStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('infosWithProof()', () => {
    it('should forward query to wasm', async () => {
      const proofQuery = { dataContractId: 'contract' };
      await client.group.infosWithProof(proofQuery);
      expect(getGroupInfosWithProofInfoStub).to.be.calledOnceWithExactly(proofQuery);
    });
  });

  describe('members()', () => {
    it('should forward query with optional filters to wasm', async () => {
      const query = {
        dataContractId: 'contract',
        groupContractPosition: 1,
        memberIds: ['a'],
        startAtMemberId: 's',
        limit: 2,
      };
      await client.group.members(query);
      expect(getGroupMembersStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('membersWithProof()', () => {
    it('should forward query to wasm', async () => {
      const proofQuery = { dataContractId: 'contract', groupContractPosition: 1 };
      await client.group.membersWithProof(proofQuery);
      expect(getGroupMembersWithProofInfoStub).to.be.calledOnceWithExactly(proofQuery);
    });
  });

  describe('identityGroups()', () => {
    it('should forward query with optional contract filters to wasm', async () => {
      const query = {
        identityId: 'identity',
        memberDataContracts: ['m'],
        ownerDataContracts: ['o'],
        moderatorDataContracts: ['d'],
      };
      await client.group.identityGroups(query);
      expect(getIdentityGroupsStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('identityGroupsWithProof()', () => {
    it('should forward query to wasm', async () => {
      const proofQuery = { identityId: 'identity' };
      await client.group.identityGroupsWithProof(proofQuery);
      expect(getIdentityGroupsWithProofInfoStub).to.be.calledOnceWithExactly(proofQuery);
    });
  });

  describe('actions()', () => {
    it('should forward query to wasm', async () => {
      const query = {
        dataContractId: 'contract',
        groupContractPosition: 1,
        status: 'ACTIVE',
        startAt: { actionId: 'cursor', included: true },
        limit: 3,
      };
      await client.group.actions(query);
      expect(getGroupActionsStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('actionsWithProof()', () => {
    it('should forward query to wasm', async () => {
      const proofQuery = {
        dataContractId: 'contract',
        groupContractPosition: 1,
        status: 'CLOSED',
      };
      await client.group.actionsWithProof(proofQuery);
      expect(getGroupActionsWithProofInfoStub).to.be.calledOnceWithExactly(proofQuery);
    });
  });

  describe('actionSigners()', () => {
    it('should forward query to wasm', async () => {
      const signersQuery = {
        dataContractId: 'contract',
        groupContractPosition: 1,
        status: 'ACTIVE',
        actionId: 'action',
      };
      await client.group.actionSigners(signersQuery);
      expect(getGroupActionSignersStub).to.be.calledOnceWithExactly(signersQuery);
    });
  });

  describe('actionSignersWithProof()', () => {
    it('should forward query to wasm', async () => {
      const signersQuery = {
        dataContractId: 'contract',
        groupContractPosition: 1,
        status: 'ACTIVE',
        actionId: 'action',
      };
      await client.group.actionSignersWithProof(signersQuery);
      expect(getGroupActionSignersWithProofInfoStub).to.be.calledOnceWithExactly(signersQuery);
    });
  });

  describe('groupsDataContracts()', () => {
    it('should forward contract IDs to wasm', async () => {
      await client.group.groupsDataContracts(['a', 'b']);
      expect(getGroupsDataContractsStub).to.be.calledOnceWithExactly(['a', 'b']);
    });
  });

  describe('groupsDataContractsWithProof()', () => {
    it('should forward contract IDs to wasm', async () => {
      await client.group.groupsDataContractsWithProof(['a']);
      expect(getGroupsDataContractsWithProofInfoStub).to.be.calledOnceWithExactly(['a']);
    });
  });

  describe('contestedResources()', () => {
    it('should forward query to wasm', async () => {
      const contestedQuery = {
        dataContractId: 'c',
        documentTypeName: 'dt',
        indexName: 'i',
        startAtValue: new Uint8Array([1]),
        limit: 2,
        orderAscending: false,
      };
      await client.group.contestedResources(contestedQuery);
      expect(getContestedResourcesStub).to.be.calledOnceWithExactly(contestedQuery);
    });
  });

  describe('contestedResourcesWithProof()', () => {
    it('should forward query to wasm', async () => {
      const contestedProofQuery = {
        dataContractId: 'c',
        documentTypeName: 'dt',
        indexName: 'i',
      };
      await client.group.contestedResourcesWithProof(contestedProofQuery);
      expect(getContestedResourcesWithProofInfoStub).to.be
        .calledOnceWithExactly(contestedProofQuery);
    });
  });

  describe('contestedResourceVotersForIdentity()', () => {
    it('should forward query to wasm', async () => {
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
      expect(getContestedResourceVotersForIdentityStub).to.be.calledOnceWithExactly(votersQuery);
    });
  });

  describe('contestedResourceVotersForIdentityWithProof()', () => {
    it('should forward query to wasm', async () => {
      const votersProofQuery = {
        dataContractId: 'c',
        documentTypeName: 'dt',
        indexName: 'i',
        indexValues: ['v2'],
        contestantId: 'id',
      };
      await client.group.contestedResourceVotersForIdentityWithProof(votersProofQuery);
      expect(getContestedResourceVotersForIdentityWithProofInfoStub)
        .to.be.calledOnceWithExactly(votersProofQuery);
    });
  });
});
