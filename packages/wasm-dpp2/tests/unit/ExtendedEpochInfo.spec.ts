import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ExtendedEpochInfo', () => {
  const epochOptions = {
    index: 42,
    firstBlockTime: 1708900000000n,
    firstBlockHeight: 100000n,
    firstCoreBlockHeight: 50000,
    feeMultiplierPermille: 1000n,
    protocolVersion: 7,
  };

  describe('toJSON()', () => {
    it('should serialize with $formatVersion tag and u64 fields as numbers', () => {
      const epoch = new wasm.ExtendedEpochInfo(epochOptions);
      const json = epoch.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.index).to.equal(42);
      // u64 values within JS safe integer range are serialized as numbers
      expect(json.firstBlockTime).to.equal(1708900000000);
      expect(json.firstBlockHeight).to.equal(100000);
      expect(json.firstCoreBlockHeight).to.equal(50000);
      expect(json.feeMultiplierPermille).to.equal(1000);
      expect(json.protocolVersion).to.equal(7);

      epoch.free();
    });
  });

  describe('fromJSON()', () => {
    it('should deserialize from JSON fixture', () => {
      const fixture = {
        $formatVersion: '0',
        index: 42,
        firstBlockTime: 1708900000000,
        firstBlockHeight: 100000,
        firstCoreBlockHeight: 50000,
        feeMultiplierPermille: 1000,
        protocolVersion: 7,
      };

      const epoch = wasm.ExtendedEpochInfo.fromJSON(fixture);

      expect(epoch.index).to.equal(42);
      expect(epoch.firstBlockTime).to.equal(1708900000000n);
      expect(epoch.firstBlockHeight).to.equal(100000n);
      expect(epoch.firstCoreBlockHeight).to.equal(50000);
      expect(epoch.protocolVersion).to.equal(7);

      epoch.free();
    });

    it('should round-trip through JSON', () => {
      const epoch = new wasm.ExtendedEpochInfo(epochOptions);

      const json = epoch.toJSON();

      // toJSON() outputs numbers for safe u64 values, fromJSON() accepts both
      // numbers and strings via StringNumberDeserializer - direct round-trip works
      const restored = wasm.ExtendedEpochInfo.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      epoch.free();
      restored.free();
    });
  });

  describe('toObject()', () => {
    it('should serialize with $formatVersion tag and BigInt for u64 fields', () => {
      const epoch = new wasm.ExtendedEpochInfo(epochOptions);
      const obj = epoch.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.index).to.equal(42);
      expect(obj.firstBlockTime).to.equal(1708900000000n);
      expect(obj.firstBlockHeight).to.equal(100000n);
      expect(obj.firstCoreBlockHeight).to.equal(50000);
      expect(obj.feeMultiplierPermille).to.equal(1000n);
      expect(obj.protocolVersion).to.equal(7);

      epoch.free();
    });
  });

  describe('fromObject()', () => {
    it('should deserialize from Object fixture', () => {
      const fixture = {
        $formatVersion: '0',
        index: 42,
        firstBlockTime: 1708900000000n,
        firstBlockHeight: 100000n,
        firstCoreBlockHeight: 50000,
        feeMultiplierPermille: 1000n,
        protocolVersion: 7,
      };

      const restored = wasm.ExtendedEpochInfo.fromObject(fixture);

      expect(restored.index).to.equal(42);
      expect(restored.firstBlockTime).to.equal(1708900000000n);
      expect(restored.firstBlockHeight).to.equal(100000n);
      expect(restored.firstCoreBlockHeight).to.equal(50000);
      expect(restored.protocolVersion).to.equal(7);

      restored.free();
    });
  });

  describe('getters', () => {
    it('should expose feeMultiplier as computed f64', () => {
      const epoch = new wasm.ExtendedEpochInfo(epochOptions);

      // feeMultiplierPermille = 1000 => feeMultiplier = 1000/1000 = 1.0
      expect(epoch.feeMultiplier).to.equal(1);

      epoch.free();
    });
  });
});
