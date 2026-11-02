import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { prefetchLocalReady } from './helpers/trustedContext.ts';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

// No contract group exists on the local network yet: registering one takes a
// DataContractCreateTransitionV1, which the SDK does not build until the creation surfaces
// land. Every query still answers and proves an absent id, which is what these tests read.
describe('Contract Group Queries', function describeContractGroupQueries() {
  this.timeout(60000);

  let client: sdk.WasmSdk;
  let dpnsContractId: string;
  const absentGroupId = new Uint8Array(32).fill(7);

  before(async () => {
    await init();
    const context = await prefetchLocalReady();
    client = await sdk.WasmSdkBuilder.local().withTrustedContext(context).build();
    ({ dpnsContractId } = wasmFunctionalTestRequirements());
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('getContractGroupInfo()', () => {
    it('should return undefined for an absent group', async () => {
      const info = await client.getContractGroupInfo(absentGroupId);
      expect(info).to.be.undefined();
    });
  });

  describe('getContractGroupInfoWithProofInfo()', () => {
    it('should prove the absence of a group', async () => {
      const res = await client.getContractGroupInfoWithProofInfo(absentGroupId);
      expect(res.data).to.be.undefined();
      expect(res.metadata).to.be.ok();
      expect(res.proof).to.be.ok();
    });
  });

  describe('getContractGroupMembers()', () => {
    it('should return an empty page of the requested kind for an absent group', async () => {
      const page = await client.getContractGroupMembers({
        contractGroupId: absentGroupId,
        kind: 'documentTypes',
        limit: 10,
      });
      expect(page.kind).to.equal('documentTypes');
      expect(page.documentTypes).to.deep.equal([]);
      expect(page.nextStartAfter).to.be.undefined();
    });

    it('should reject a limit above the page cap', async () => {
      let error: Error | undefined;
      try {
        await client.getContractGroupMembers({ contractGroupId: absentGroupId, kind: 'contracts', limit: 101 });
      } catch (e) {
        error = e as Error;
      }
      expect(error).to.be.ok();
    });
  });

  describe('getContractGroupMembersWithProofInfo()', () => {
    it('should prove an empty page', async () => {
      const res = await client.getContractGroupMembersWithProofInfo({
        contractGroupId: absentGroupId,
        kind: 'tokens',
      });
      expect(res.data.kind).to.equal('tokens');
      expect(res.data.tokens).to.deep.equal([]);
      expect(res.metadata).to.be.ok();
      expect(res.proof).to.be.ok();
    });
  });

  describe('getContractGroupsForContract()', () => {
    it('should return empty memberships for a contract in no group', async () => {
      const memberships = await client.getContractGroupsForContract(dpnsContractId);
      expect(memberships.contract).to.deep.equal([]);
      expect(memberships.documentTypes).to.deep.equal({});
      expect(memberships.tokens).to.deep.equal({});
    });
  });

  describe('getContractGroupsForContractWithProofInfo()', () => {
    it('should prove empty memberships', async () => {
      const res = await client.getContractGroupsForContractWithProofInfo(dpnsContractId);
      expect(res.data.contract).to.deep.equal([]);
      expect(res.metadata).to.be.ok();
      expect(res.proof).to.be.ok();
    });
  });
});
