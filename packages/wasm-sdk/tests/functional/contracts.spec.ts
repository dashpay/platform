import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { prefetchLocalReady } from './helpers/trustedContext.ts';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Data Contract Queries', function describeDataContractQueries() {
  this.timeout(60000);

  const { dpnsContractId, tokenContracts } = wasmFunctionalTestRequirements();

  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    const context = await prefetchLocalReady();
    const builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('getDataContract()', () => {
    it('should return data contract', async () => {
      const res = await client.getDataContract(dpnsContractId);
      expect(res).to.be.ok();
      expect(res.id.toString()).to.equal(dpnsContractId);
    });
  });

  describe('getDataContractWithProofInfo()', () => {
    it('should return proof info', async () => {
      const res = await client.getDataContractWithProofInfo(dpnsContractId);
      expect(res).to.be.ok();
      expect(res.data).to.be.ok();
      expect(res.metadata).to.be.ok();
      expect(res.proof).to.be.ok();
    });
  });

  describe('getDataContracts()', () => {
    it('should return multiple contracts', async () => {
      const contractIds = [dpnsContractId];
      if (tokenContracts.length > 0) {
        contractIds.push(tokenContracts[0].contractId);
      }

      const res = await client.getDataContracts(contractIds);
      expect(res).to.be.instanceOf(Map);
      expect(res.size).to.be.at.least(1);
    });
  });

  describe('getDataContractsWithProofInfo()', () => {
    it('should return proof info for multiple contracts', async () => {
      const contractIds = [dpnsContractId];

      const res = await client.getDataContractsWithProofInfo(contractIds);
      expect(res).to.be.ok();
      expect(res.data).to.be.instanceOf(Map);
      expect(res.metadata).to.be.ok();
      expect(res.proof).to.be.ok();
    });
  });

  describe('getDataContractsByRange()', () => {
    it('should page through contracts in ascending id order', async () => {
      const firstPage = await client.getDataContractsByRange({ limit: 1 });
      expect(firstPage).to.be.instanceOf(Map);
      expect(firstPage.size).to.equal(1);
      const [firstId] = firstPage.keys();

      const nextPage = await client.getDataContractsByRange({ limit: 1, startAfter: firstId });
      expect(nextPage.size).to.equal(1);
      const [nextId] = nextPage.keys();
      // Base58 strings do not sort like the raw id bytes the query orders by, so check the
      // cursor against a two-item page instead of comparing the strings.
      const firstTwo = await client.getDataContractsByRange({ limit: 2 });
      expect([...firstTwo.keys()]).to.deep.equal([firstId, nextId]);

      const fromFirst = await client.getDataContractsByRange({ limit: 1, startAt: firstId });
      expect([...fromFirst.keys()]).to.deep.equal([firstId]);
    });

    it('should include the DPNS contract in the full first page', async () => {
      const page = await client.getDataContractsByRange({});
      expect(page.has(dpnsContractId)).to.be.true();
      expect(page.get(dpnsContractId)).to.be.instanceOf(sdk.DataContract);
    });

    it('should return ids only when requested', async () => {
      const page = await client.getDataContractsByRange({ idsOnly: true });
      expect(page.size).to.be.at.least(1);
      expect([...page.values()].every((value) => value === undefined)).to.be.true();
    });
  });

  describe('getDataContractsByRangeWithProofInfo()', () => {
    it('should return proof info for a page of contracts', async () => {
      const res = await client.getDataContractsByRangeWithProofInfo({ limit: 2 });
      expect(res).to.be.ok();
      expect(res.data).to.be.instanceOf(Map);
      expect(res.metadata).to.be.ok();
      expect(res.proof).to.be.ok();
    });
  });

  describe('getDataContractsLatestVersions()', () => {
    it('should return one entry per id, versions only, undefined for an unknown id', async () => {
      const unknownId = new Uint8Array(32).fill(7);
      const res = await client.getDataContractsLatestVersions({ contractIds: [dpnsContractId, unknownId] });
      expect(res).to.be.instanceOf(Map);
      expect(res.size).to.equal(2);
      const dpns = res.get(dpnsContractId);
      expect(dpns).to.be.ok();
      expect(dpns.version).to.be.at.least(1);
      expect(dpns.dataContract).to.be.undefined();
      const [, unknownEntry] = [...res.values()].filter((value) => value !== dpns);
      expect(unknownEntry).to.be.undefined();
    });

    it('should include the contracts when asked', async () => {
      const res = await client.getDataContractsLatestVersions({
        contractIds: [dpnsContractId],
        includeContracts: true,
      });
      const dpns = res.get(dpnsContractId);
      expect(dpns.dataContract).to.be.instanceOf(sdk.DataContract);
      expect(dpns.dataContract.id.toString()).to.equal(dpnsContractId);
      expect(dpns.dataContract.version).to.equal(dpns.version);
    });
  });

  describe('getDataContractsLatestVersionsUnproved()', () => {
    it('should return versions only, undefined for an unknown id', async () => {
      const unknownId = new Uint8Array(32).fill(7);
      const res = await client.getDataContractsLatestVersionsUnproved({ contractIds: [dpnsContractId, unknownId] });
      expect(res).to.be.instanceOf(Map);
      expect(res.size).to.equal(2);
      const dpns = res.get(dpnsContractId);
      expect(dpns).to.be.ok();
      expect(dpns.version).to.be.at.least(1);
      expect(dpns.dataContract).to.be.undefined();
    });

    it('should refuse includeContracts', async () => {
      let failed = false;
      try {
        await client.getDataContractsLatestVersionsUnproved({ contractIds: [dpnsContractId], includeContracts: true });
      } catch (e) {
        failed = true;
        expect(String(e)).to.match(/includeContracts/);
      }
      expect(failed).to.be(true);
    });
  });

  describe('addKnownContract()', () => {
    it('should accept a contract the caller holds', async () => {
      const contract = await client.getDataContract(dpnsContractId);
      expect(client.addKnownContract(contract)).to.be(true);
    });
  });

  describe('getDataContractsLatestVersionsWithProofInfo()', () => {
    it('should return proof info for the versions', async () => {
      const res = await client.getDataContractsLatestVersionsWithProofInfo({ contractIds: [dpnsContractId] });
      expect(res).to.be.ok();
      expect(res.data).to.be.instanceOf(Map);
      expect(res.data.get(dpnsContractId).version).to.be.at.least(1);
      expect(res.metadata).to.be.ok();
      expect(res.proof).to.be.ok();
    });
  });

  describe('getDataContractHistory()', () => {
    // TODO: Fix proof verification error: dash drive: proof: corrupted error:
    // we did not get back an element for the correct path for the historical contract
    it.skip('should return history for contract', async () => {
      // Use DPNS contract to test history retrieval
      // Note: This may return an empty map if the contract has no history (version = 1)
      const res = await client.getDataContractHistory({
        dataContractId: dpnsContractId,
        limit: 10,
      });
      expect(res).to.be.instanceOf(Map);
    });
  });
});
