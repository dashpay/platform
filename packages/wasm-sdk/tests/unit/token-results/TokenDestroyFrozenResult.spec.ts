import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenDestroyFrozenResult', () => {
  before(async () => {
    await init();
  });

  describe('fromObject()', () => {
    it('should create result from object with groupPower', () => {
      const data = {
        groupPower: 90,
      };

      const result = sdk.TokenDestroyFrozenResult.fromObject(data);
      expect(result.groupPower).to.equal(90);
      expect(result.document).to.be.undefined();
    });

    it('should handle empty data', () => {
      const data = {};

      const result = sdk.TokenDestroyFrozenResult.fromObject(data);
      expect(result.groupPower).to.be.undefined();
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        groupPower: 90,
      };

      const result = sdk.TokenDestroyFrozenResult.fromObject(data);
      const obj = result.toObject();
      const roundtrip = sdk.TokenDestroyFrozenResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(90);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        groupPower: 85,
      };

      const result = sdk.TokenDestroyFrozenResult.fromJSON(data);
      expect(result.groupPower).to.equal(85);
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        groupPower: 85,
      };

      const result = sdk.TokenDestroyFrozenResult.fromJSON(data);
      const json = result.toJSON();
      const roundtrip = sdk.TokenDestroyFrozenResult.fromJSON(json);
      expect(roundtrip.groupPower).to.equal(85);
    });
  });
});
