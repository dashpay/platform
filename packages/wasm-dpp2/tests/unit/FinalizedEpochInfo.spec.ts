import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('FinalizedEpochInfo', () => {
  const testId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  // Use empty blockProposers for toObject tests since non-empty Map
  // with Identifier keys fails in toObject (known limitation)
  const finalizedOptions = {
    firstBlockTime: 1708900000000n,
    firstBlockHeight: 100000n,
    totalBlocksInEpoch: 720n,
    firstCoreBlockHeight: 50000,
    nextEpochStartCoreBlockHeight: 50720,
    totalProcessingFees: 5000000n,
    totalDistributedStorageFees: 1000000n,
    totalCreatedStorageFees: 2000000n,
    coreBlockRewards: 3000000n,
    blockProposers: new Map([[testId, 100n]]),
    feeMultiplierPermille: 1000n,
    protocolVersion: 7,
  };

  describe('toJSON()', () => {
    it('should serialize with $formatVersion tag and u64 fields as numbers', () => {
      const info = new wasm.FinalizedEpochInfo(finalizedOptions);
      const json = info.toJSON();

      expect(json.$formatVersion).to.equal('0');
      // u64 values within JS safe integer range are serialized as numbers
      expect(json.firstBlockTime).to.equal(1708900000000);
      expect(json.firstBlockHeight).to.equal(100000);
      expect(json.totalBlocksInEpoch).to.equal(720);
      expect(json.firstCoreBlockHeight).to.equal(50000);
      expect(json.nextEpochStartCoreBlockHeight).to.equal(50720);
      expect(json.totalProcessingFees).to.equal(5000000);
      expect(json.totalDistributedStorageFees).to.equal(1000000);
      expect(json.totalCreatedStorageFees).to.equal(2000000);
      expect(json.coreBlockRewards).to.equal(3000000);
      expect(json.feeMultiplierPermille).to.equal(1000);
      expect(json.protocolVersion).to.equal(7);

      // blockProposers should be a Record<string, number> in JSON
      expect(json.blockProposers).to.have.property(testId);
      expect(json.blockProposers[testId]).to.equal(100);

      info.free();
    });
  });

  describe('fromJSON()', () => {
    it('should deserialize from JSON fixture', () => {
      const fixture = {
        $formatVersion: '0',
        firstBlockTime: 1708900000000,
        firstBlockHeight: 100000,
        totalBlocksInEpoch: 720,
        firstCoreBlockHeight: 50000,
        nextEpochStartCoreBlockHeight: 50720,
        totalProcessingFees: 5000000,
        totalDistributedStorageFees: 1000000,
        totalCreatedStorageFees: 2000000,
        coreBlockRewards: 3000000,
        blockProposers: { [testId]: 100 },
        feeMultiplierPermille: 1000,
        protocolVersion: 7,
      };

      const info = wasm.FinalizedEpochInfo.fromJSON(fixture);

      expect(info.firstBlockTime).to.equal(1708900000000n);
      expect(info.firstBlockHeight).to.equal(100000n);
      expect(info.totalBlocksInEpoch).to.equal(720n);
      expect(info.firstCoreBlockHeight).to.equal(50000);
      expect(info.nextEpochStartCoreBlockHeight).to.equal(50720);
      expect(info.totalProcessingFees).to.equal(5000000n);
      expect(info.protocolVersion).to.equal(7);

      info.free();
    });

    it('should round-trip through JSON', () => {
      const info = new wasm.FinalizedEpochInfo(finalizedOptions);

      const json = info.toJSON();

      // toJSON() outputs numbers for safe u64 values, fromJSON() accepts both
      // numbers and strings via StringNumberDeserializer - direct round-trip works
      const restored = wasm.FinalizedEpochInfo.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      info.free();
      restored.free();
    });
  });

  describe('toObject()', () => {
    it('should serialize with $formatVersion tag and BigInt for u64 fields', () => {
      // Use empty blockProposers to avoid Map key conversion issue
      const options = { ...finalizedOptions, blockProposers: new Map() };
      const info = new wasm.FinalizedEpochInfo(options);
      const obj = info.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.firstBlockTime).to.equal(1708900000000n);
      expect(obj.firstBlockHeight).to.equal(100000n);
      expect(obj.totalBlocksInEpoch).to.equal(720n);
      expect(obj.firstCoreBlockHeight).to.equal(50000);
      expect(obj.totalProcessingFees).to.equal(5000000n);
      expect(obj.feeMultiplierPermille).to.equal(1000n);
      expect(obj.protocolVersion).to.equal(7);

      info.free();
    });
  });

  describe('fromObject()', () => {
    it('should deserialize from Object fixture', () => {
      // fromObject uses serde_wasm_bindgen::from_value which cannot handle
      // JS Map for BTreeMap - use a plain object instead.
      const fixture = {
        $formatVersion: '0',
        firstBlockTime: 1708900000000n,
        firstBlockHeight: 100000n,
        totalBlocksInEpoch: 720n,
        firstCoreBlockHeight: 50000,
        nextEpochStartCoreBlockHeight: 50720,
        totalProcessingFees: 5000000n,
        totalDistributedStorageFees: 1000000n,
        totalCreatedStorageFees: 2000000n,
        coreBlockRewards: 3000000n,
        blockProposers: {},
        feeMultiplierPermille: 1000n,
        protocolVersion: 7,
      };

      const restored = wasm.FinalizedEpochInfo.fromObject(fixture);

      expect(restored.firstBlockTime).to.equal(1708900000000n);
      expect(restored.firstBlockHeight).to.equal(100000n);
      expect(restored.totalBlocksInEpoch).to.equal(720n);
      expect(restored.firstCoreBlockHeight).to.equal(50000);
      expect(restored.protocolVersion).to.equal(7);

      restored.free();
    });
  });

  describe('getters', () => {
    it('should expose feeMultiplier as computed f64', () => {
      const info = new wasm.FinalizedEpochInfo(finalizedOptions);

      // feeMultiplierPermille = 1000 => feeMultiplier = 1.0
      expect(info.feeMultiplier).to.equal(1);

      info.free();
    });
  });
});
