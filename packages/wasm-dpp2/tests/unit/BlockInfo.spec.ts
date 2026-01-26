import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('BlockInfo', () => {
  describe('constructor()', () => {
    it('should create BlockInfo with all parameters', () => {
      const blockInfo = new wasm.BlockInfo(1000n, 100n, 50, 5);

      expect(blockInfo.timeMs).to.equal(1000n);
      expect(blockInfo.height).to.equal(100n);
      expect(blockInfo.coreHeight).to.equal(50);
      expect(blockInfo.epochIndex).to.equal(5);
    });

    it('should create BlockInfo with zero values', () => {
      const blockInfo = new wasm.BlockInfo(0n, 0n, 0, 0);

      expect(blockInfo.timeMs).to.equal(0n);
      expect(blockInfo.height).to.equal(0n);
      expect(blockInfo.coreHeight).to.equal(0);
      expect(blockInfo.epochIndex).to.equal(0);
    });
  });

  describe('toJSON()', () => {
    it('should round-trip via toJSON/fromJSON', () => {
      const blockInfo = new wasm.BlockInfo(1234567890n, 999n, 500, 10);

      const json = blockInfo.toJSON();
      // serde uses snake_case by default
      expect(json.time_ms).to.equal(1234567890);
      expect(json.height).to.equal(999);
      expect(json.core_height).to.equal(500);
      expect(json.epoch.index).to.equal(10);

      const restored = wasm.BlockInfo.fromJSON(json);
      expect(restored.timeMs).to.equal(blockInfo.timeMs);
      expect(restored.height).to.equal(blockInfo.height);
      expect(restored.coreHeight).to.equal(blockInfo.coreHeight);
      expect(restored.epochIndex).to.equal(blockInfo.epochIndex);
    });
  });

  describe('toObject()', () => {
    it('should round-trip via toObject/fromObject', () => {
      const blockInfo = new wasm.BlockInfo(1234567890n, 999n, 500, 10);

      const obj = blockInfo.toObject();
      // serde uses snake_case by default
      // serde_wasm_bindgen serializes u64 as Number when it fits
      expect(Number(obj.time_ms)).to.equal(1234567890);
      expect(Number(obj.height)).to.equal(999);
      expect(obj.core_height).to.equal(500);
      expect(obj.epoch.index).to.equal(10);

      const restored = wasm.BlockInfo.fromObject(obj);
      expect(restored.timeMs).to.equal(blockInfo.timeMs);
      expect(restored.height).to.equal(blockInfo.height);
      expect(restored.coreHeight).to.equal(blockInfo.coreHeight);
      expect(restored.epochIndex).to.equal(blockInfo.epochIndex);
    });

    it('should handle large values correctly', () => {
      // Test with BigInt-sized values
      const largeTimeMs = 1702500000000n; // realistic timestamp in ms
      const largeHeight = 1000000n;
      const blockInfo = new wasm.BlockInfo(largeTimeMs, largeHeight, 800000, 100);

      // JSON round-trip
      const json = blockInfo.toJSON();
      const restoredFromJson = wasm.BlockInfo.fromJSON(json);
      expect(restoredFromJson.timeMs).to.equal(largeTimeMs);
      expect(restoredFromJson.height).to.equal(largeHeight);

      // Object round-trip
      const obj = blockInfo.toObject();
      const restoredFromObj = wasm.BlockInfo.fromObject(obj);
      expect(restoredFromObj.timeMs).to.equal(largeTimeMs);
      expect(restoredFromObj.height).to.equal(largeHeight);
    });
  });

  describe('timeMs', () => {
    it('should return timeMs', () => {
      const blockInfo = new wasm.BlockInfo(5000n, 100n, 50, 5);
      expect(blockInfo.timeMs).to.equal(5000n);
    });
  });

  describe('height', () => {
    it('should return height', () => {
      const blockInfo = new wasm.BlockInfo(5000n, 100n, 50, 5);
      expect(blockInfo.height).to.equal(100n);
    });
  });

  describe('coreHeight', () => {
    it('should return coreHeight', () => {
      const blockInfo = new wasm.BlockInfo(5000n, 100n, 50, 5);
      expect(blockInfo.coreHeight).to.equal(50);
    });
  });

  describe('epochIndex', () => {
    it('should return epochIndex', () => {
      const blockInfo = new wasm.BlockInfo(5000n, 100n, 50, 5);
      expect(blockInfo.epochIndex).to.equal(5);
    });
  });
});
