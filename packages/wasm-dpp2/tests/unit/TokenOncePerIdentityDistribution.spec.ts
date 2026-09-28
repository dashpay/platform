import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('TokenOncePerIdentityDistribution', () => {
  describe('constructor', () => {
    it('should create instance from a bigint amount', () => {
      const distribution = new wasm.TokenOncePerIdentityDistribution(BigInt(1000));

      expect(distribution).to.be.an.instanceof(wasm.TokenOncePerIdentityDistribution);
      expect(distribution.amount).to.equal(BigInt(1000));
    });

    it('should create instance from a number amount', () => {
      const distribution = new wasm.TokenOncePerIdentityDistribution(250);

      expect(distribution.amount).to.equal(BigInt(250));
    });

    it('should reject a negative amount', () => {
      expect(() => new wasm.TokenOncePerIdentityDistribution(-1)).to.throw();
    });
  });

  describe('amount', () => {
    it('should set amount', () => {
      const distribution = new wasm.TokenOncePerIdentityDistribution(BigInt(1000));

      distribution.amount = BigInt(5000);

      expect(distribution.amount).to.equal(BigInt(5000));
    });
  });
});
