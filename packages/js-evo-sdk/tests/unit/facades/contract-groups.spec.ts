import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('ContractGroupsFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  let getContractGroupInfoStub: SinonStub;
  let getContractGroupInfoWithProofInfoStub: SinonStub;
  let getContractGroupMembersStub: SinonStub;
  let getContractGroupMembersWithProofInfoStub: SinonStub;
  let getContractGroupsForContractStub: SinonStub;
  let getContractGroupsForContractWithProofInfoStub: SinonStub;

  const contractGroupId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
  const contractId = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getContractGroupInfoStub = this.sinon.stub(wasmSdk, 'getContractGroupInfo').resolves(undefined);
    getContractGroupInfoWithProofInfoStub = this.sinon.stub(wasmSdk, 'getContractGroupInfoWithProofInfo').resolves({
      data: undefined,
      proof: {},
      metadata: {},
    });
    getContractGroupMembersStub = this.sinon.stub(wasmSdk, 'getContractGroupMembers').resolves({ kind: 'contracts', contracts: [] });
    getContractGroupMembersWithProofInfoStub = this.sinon.stub(wasmSdk, 'getContractGroupMembersWithProofInfo').resolves({
      data: { kind: 'contracts', contracts: [] },
      proof: {},
      metadata: {},
    });
    getContractGroupsForContractStub = this.sinon.stub(wasmSdk, 'getContractGroupsForContract').resolves({
      contract: [],
      documentTypes: {},
      tokens: {},
    });
    getContractGroupsForContractWithProofInfoStub = this.sinon.stub(wasmSdk, 'getContractGroupsForContractWithProofInfo').resolves({
      data: { contract: [], documentTypes: {}, tokens: {} },
      proof: {},
      metadata: {},
    });
  });

  describe('info()', () => {
    it('should forward the contract group id to wasm', async () => {
      await client.contractGroups.info(contractGroupId);
      expect(getContractGroupInfoStub).to.be.calledOnceWithExactly(contractGroupId);
    });
  });

  describe('infoWithProof()', () => {
    it('should forward the contract group id to wasm', async () => {
      await client.contractGroups.infoWithProof(contractGroupId);
      expect(getContractGroupInfoWithProofInfoStub).to.be.calledOnceWithExactly(contractGroupId);
    });
  });

  describe('members()', () => {
    it('should forward the members query to wasm', async () => {
      const query = {
        contractGroupId,
        kind: 'documentTypes' as const,
        startAfter: { contractId, documentTypeName: 'note' },
        limit: 10,
      };

      await client.contractGroups.members(query);

      expect(getContractGroupMembersStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('membersWithProof()', () => {
    it('should forward the members query to wasm', async () => {
      const query = { contractGroupId, kind: 'tokens' as const };

      await client.contractGroups.membersWithProof(query);

      expect(getContractGroupMembersWithProofInfoStub).to.be.calledOnceWithExactly(query);
    });
  });

  describe('forContract()', () => {
    it('should forward the contract id to wasm', async () => {
      await client.contractGroups.forContract(contractId);
      expect(getContractGroupsForContractStub).to.be.calledOnceWithExactly(contractId);
    });
  });

  describe('forContractWithProof()', () => {
    it('should forward the contract id to wasm', async () => {
      await client.contractGroups.forContractWithProof(contractId);
      expect(getContractGroupsForContractWithProofInfoStub).to.be.calledOnceWithExactly(contractId);
    });
  });
});
