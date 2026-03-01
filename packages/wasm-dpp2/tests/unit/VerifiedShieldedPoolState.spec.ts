import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('VerifiedShieldedPoolState', () => {
  const jsonFixture = {
    poolBalance: 1000000,
  };

  const objectFixture = {
    poolBalance: 1000000n,
  };

  describe('toJSON()', () => {
    it('should produce expected JSON', () => {
      const result = wasm.VerifiedShieldedPoolState.fromJSON(jsonFixture);
      expect(result.toJSON()).to.deep.equal(jsonFixture);
    });
  });

  describe('toObject()', () => {
    it('should produce expected Object', () => {
      const result = wasm.VerifiedShieldedPoolState.fromObject(objectFixture);
      expect(result.toObject()).to.deep.equal(objectFixture);
    });
  });

  describe('fromJSON()', () => {
    it('should deserialize and expose getters', () => {
      const result = wasm.VerifiedShieldedPoolState.fromJSON(jsonFixture);
      expect(result.poolBalance).to.equal(1000000n);
    });

    it('should handle null balance', () => {
      const result = wasm.VerifiedShieldedPoolState.fromJSON({ poolBalance: null });
      expect(result.poolBalance).to.be.undefined();
    });
  });

  describe('fromObject()', () => {
    it('should deserialize and expose getters', () => {
      const result = wasm.VerifiedShieldedPoolState.fromObject(objectFixture);
      expect(result.poolBalance).to.equal(1000000n);
    });

    it('should handle null balance', () => {
      const result = wasm.VerifiedShieldedPoolState.fromObject({ poolBalance: null });
      expect(result.poolBalance).to.be.undefined();
    });
  });
});
