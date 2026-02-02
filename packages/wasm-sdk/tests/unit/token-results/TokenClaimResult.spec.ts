import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenClaimResult', () => {
  before(async () => {
    await init();
  });

  describe('fromObject()', () => {
    it('should create result from object with groupPower', () => {
      const data = {
        groupPower: 33,
      };

      const result = sdk.TokenClaimResult.fromObject(data);
      expect(result.groupPower).to.equal(33);
      expect(result.document).to.be.undefined();
    });

    it('should handle empty data', () => {
      const data = {};

      const result = sdk.TokenClaimResult.fromObject(data);
      expect(result.groupPower).to.be.undefined();
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        groupPower: 33,
      };

      const result = sdk.TokenClaimResult.fromObject(data);
      const obj = result.toObject();
      const roundtrip = sdk.TokenClaimResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(33);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        groupPower: 42,
      };

      const result = sdk.TokenClaimResult.fromJSON(data);
      expect(result.groupPower).to.equal(42);
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        groupPower: 42,
      };

      const result = sdk.TokenClaimResult.fromJSON(data);
      const json = result.toJSON();
      const roundtrip = sdk.TokenClaimResult.fromJSON(json);
      expect(roundtrip.groupPower).to.equal(42);
    });
  });
});
