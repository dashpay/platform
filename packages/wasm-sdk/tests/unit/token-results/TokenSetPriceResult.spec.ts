import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenSetPriceResult', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  describe('fromObject()', () => {
    it('should create result from object with all fields', () => {
      const data = {
        ownerId: testIdentifier,
        groupPower: 70,
        groupActionStatus: 'Approved',
      };

      const result = sdk.TokenSetPriceResult.fromObject(data);

      expect(result.ownerId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(70);
      expect(result.groupActionStatus).to.equal('Approved');
      // pricingSchedule is skipped in serde, so it won't be present
      expect(result.pricingSchedule).to.be.undefined();
    });

    it('should handle group action without owner', () => {
      const data = {
        groupPower: 50,
        groupActionStatus: 'Pending',
      };

      const result = sdk.TokenSetPriceResult.fromObject(data);
      expect(result.ownerId).to.be.undefined();
      expect(result.groupPower).to.equal(50);
      expect(result.groupActionStatus).to.equal('Pending');
      expect(result.pricingSchedule).to.be.undefined();
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        ownerId: testIdentifier,
        groupPower: 70,
        groupActionStatus: 'Approved',
      };

      const result = sdk.TokenSetPriceResult.fromObject(data);
      const obj = result.toObject();
      const roundtrip = sdk.TokenSetPriceResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(70);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        ownerId: testIdentifier,
        groupPower: 70,
        groupActionStatus: 'Approved',
      };

      const result = sdk.TokenSetPriceResult.fromJSON(data);

      expect(result.ownerId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(70);
      expect(result.groupActionStatus).to.equal('Approved');
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        ownerId: testIdentifier,
        groupPower: 70,
        groupActionStatus: 'Approved',
      };

      const result = sdk.TokenSetPriceResult.fromJSON(data);
      const json = result.toJSON();
      const roundtrip = sdk.TokenSetPriceResult.fromJSON(json);
      expect(roundtrip.ownerId.toBase58()).to.equal(testIdentifier);
      expect(roundtrip.groupPower).to.equal(70);
    });
  });
});
