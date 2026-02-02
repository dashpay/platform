import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Data Contract Queries', function describeDataContractQueries() {
  this.timeout(60000);

  const { dpnsContractId, tokenContracts } = wasmFunctionalTestRequirements();

  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
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
