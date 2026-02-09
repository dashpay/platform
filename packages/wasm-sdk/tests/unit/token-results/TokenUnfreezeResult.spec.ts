import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenUnfreezeResult', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  describe('fromObject()', () => {
    it('should create result from object with all fields', () => {
      const data = {
        unfrozenIdentityId: testIdentifier,
        groupPower: 70,
      };

      const result = sdk.TokenUnfreezeResult.fromObject(data);

      expect(result.unfrozenIdentityId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(70);
    });

    it('should handle group action without identity', () => {
      const data = {
        groupPower: 35,
      };

      const result = sdk.TokenUnfreezeResult.fromObject(data);
      expect(result.unfrozenIdentityId).to.be.undefined();
      expect(result.groupPower).to.equal(35);
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        unfrozenIdentityId: testIdentifier,
        groupPower: 70,
      };

      const result = sdk.TokenUnfreezeResult.fromObject(data);
      const obj = result.toObject();
      const roundtrip = sdk.TokenUnfreezeResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(70);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        unfrozenIdentityId: testIdentifier,
        groupPower: 60,
      };

      const result = sdk.TokenUnfreezeResult.fromJSON(data);

      expect(result.unfrozenIdentityId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(60);
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        unfrozenIdentityId: testIdentifier,
        groupPower: 60,
      };

      const result = sdk.TokenUnfreezeResult.fromJSON(data);
      const json = result.toJSON();
      const roundtrip = sdk.TokenUnfreezeResult.fromJSON(json);
      expect(roundtrip.unfrozenIdentityId.toBase58()).to.equal(testIdentifier);
      expect(roundtrip.groupPower).to.equal(60);
    });
  });
});
