import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { prefetchLocalReady } from './helpers/trustedContext.ts';

describe('Epochs and Evonode Blocks', function describeEpochs() {
  this.timeout(60000);

  let client: sdk.WasmSdk;
  let evonodeProTxHash: string;

  before(async () => {
    await init();
    const context = await prefetchLocalReady();
    const builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
    client = await builder.build();

    // Get the proTxHash from the node status
    const status = await client.getStatus();
    evonodeProTxHash = status.node.proTxHash;
  });

  after(async () => {
    // Wait briefly to ensure any pending async operations complete
    await new Promise((resolve) => { setTimeout(resolve, 100); });
    if (client) { client.free(); }
  });

  describe('getEpochsInfo()', () => {
    it('should get epochs info and finalized epochs', async () => {
      // Get current epoch info
      const current = await client.getCurrentEpoch().catch(() => null);
      const currentIndex = current ? Number(current.index) : 0;
      const start = Math.max(0, currentIndex - 5);

      const infos = await client.getEpochsInfo({
        startEpoch: start,
        count: 5,
        ascending: true,
      });
      expect(infos).to.be.instanceOf(Map);
    });
  });

  describe('getFinalizedEpochInfos()', () => {
    it('should get finalized epoch infos', async () => {
      // Get current epoch info
      const current = await client.getCurrentEpoch().catch(() => null);
      const currentIndex = current ? Number(current.index) : 0;
      const start = Math.max(0, currentIndex - 5);

      const finalized = await client.getFinalizedEpochInfos({
        startEpoch: start,
        count: 5,
      });
      expect(finalized).to.be.instanceOf(Map);
    });
  });

  describe('getEvonodesProposedEpochBlocksByIds()', () => {
    it('should query evonode proposed blocks by ids', async () => {
      // Get current epoch
      const current = await client.getCurrentEpoch().catch(() => null);
      const epochIndex = current ? Number(current.index) : 0;

      // Query by specific IDs only if we have a proTxHash
      if (evonodeProTxHash) {
        const byIds = await client
          .getEvonodesProposedEpochBlocksByIds(epochIndex, [evonodeProTxHash]);
        expect(byIds).to.be.instanceOf(Map);
      }
    });
  });

  describe('getEvonodesProposedEpochBlocksByRange()', () => {
    it('should query evonode proposed blocks by range', async () => {
      // Get current epoch
      const current = await client.getCurrentEpoch().catch(() => null);
      const epochIndex = current ? Number(current.index) : 0;

      // Query by range (doesn't require a specific proTxHash)
      const byRange = await client.getEvonodesProposedEpochBlocksByRange({
        epoch: epochIndex,
        limit: 50,
      });
      expect(byRange).to.be.instanceOf(Map);
    });
  });

  describe('getEvonodesProposedEpochBlocksByIdsWithProofInfo()', () => {
    it('should query evonode proposed blocks by ids with proof', async () => {
      const current = await client.getCurrentEpoch().catch(() => null);
      const epochIndex = current ? Number(current.index) : 0;

      // Get at least one proTxHash from the range query results
      const byRange = await client.getEvonodesProposedEpochBlocksByRange({
        epoch: epochIndex,
        limit: 1,
      });
      expect(byRange).to.be.instanceOf(Map);

      // Get a proTxHash from the results (or use node's proTxHash if available)
      let testProTxHash = evonodeProTxHash;
      if (!testProTxHash && byRange.size > 0) {
        // The keys in the map are hex strings (ProTxHash.toHex())
        const firstKey = byRange.keys().next().value;
        if (firstKey) {
          testProTxHash = firstKey;
        }
      }

      // Only test by IDs if we have a valid proTxHash
      if (testProTxHash) {
        const res = await client
          .getEvonodesProposedEpochBlocksByIdsWithProofInfo(epochIndex, [testProTxHash]);
        expect(res).to.be.ok();
        expect(res.data).to.be.instanceOf(Map);
        expect(res.proof).to.be.ok();
        expect(res.metadata).to.be.ok();
      }
    });
  });

  describe('getEvonodesProposedEpochBlocksByRangeWithProofInfo()', () => {
    it('should query evonode proposed blocks by range with proof', async () => {
      const current = await client.getCurrentEpoch().catch(() => null);
      const epochIndex = current ? Number(current.index) : 0;

      const res = await client.getEvonodesProposedEpochBlocksByRangeWithProofInfo({
        epoch: epochIndex,
        limit: 50,
      });
      expect(res).to.be.ok();
      expect(res.data).to.be.instanceOf(Map);
      expect(res.proof).to.be.ok();
      expect(res.metadata).to.be.ok();
    });
  });
});
