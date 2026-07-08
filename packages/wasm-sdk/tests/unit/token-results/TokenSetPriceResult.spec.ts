import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenSetPriceResult', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  // Hardcoded expected JSON fixture (camelCase)
  const expectedJSON = {
    ownerId: testIdentifier,
    groupPower: 70,
    groupActionStatus: 'Approved',
  };

  // Hardcoded expected Object fixture (camelCase)
  const expectedObject = {
    ownerId: testIdentifier,
    groupPower: 70,
    groupActionStatus: 'Approved',
  };

  const documentJSON = {
    $formatVersion: "0",
    $id: '9tSsCqKHTZ8ro16MydChSxgHBukFW36eMLJKKRtebJEn',
    $ownerId: 'CXH2kZCATjvDTnQAPVg28EgPg9WySUvwvnR5ZkmNqY5i',
    $dataContractId: 'GnXgMaiqAwTxh44ccQe8AoCgFvcseHK5CncH3sUorW4X',
    $type: 'note',
    $revision: 1,
    message: 'hello',
  };

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

    it('should produce output matching expected Object fixture', () => {
      const result = sdk.TokenSetPriceResult.fromObject(expectedObject);
      const obj = result.toObject();

      expect(obj.groupPower).to.equal(expectedObject.groupPower);
      expect(obj.groupActionStatus).to.equal(expectedObject.groupActionStatus);
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

    it('should create from JSON fixture and verify all fields via getters', () => {
      const result = sdk.TokenSetPriceResult.fromJSON(expectedJSON);

      expect(result.ownerId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(70);
      expect(result.groupActionStatus).to.equal('Approved');
      expect(result.pricingSchedule).to.be.undefined();
      expect(result.document).to.be.undefined();
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

    it('should produce output matching expected JSON fixture', () => {
      const result = sdk.TokenSetPriceResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.ownerId).to.equal(expectedJSON.ownerId);
      expect(json.groupPower).to.equal(expectedJSON.groupPower);
      expect(json.groupActionStatus).to.equal(expectedJSON.groupActionStatus);
    });
  });

  describe('pricingSchedule serialization', () => {
    it('should include pricingSchedule in toJSON when present', () => {
      const data = {
        ownerId: testIdentifier,
        // TokenPricingSchedule is internally `$type`-tagged (camelCase variant,
        // JS-safe `price`); externally-tagged `{ SinglePrice: N }` is no longer valid.
        pricingSchedule: { $type: 'singlePrice', price: 5000 },
        groupPower: 70,
      };

      const result = sdk.TokenSetPriceResult.fromJSON(data);
      expect(result.pricingSchedule).to.exist();

      const json = result.toJSON();
      expect(json.pricingSchedule).to.exist();
      expect(json.pricingSchedule.$type).to.equal('singlePrice');
      expect(json.pricingSchedule.price).to.equal(5000);
    });

    it('should round-trip pricingSchedule through toJSON/fromJSON', () => {
      const data = {
        ownerId: testIdentifier,
        pricingSchedule: { $type: 'singlePrice', price: 5000 },
        groupPower: 70,
      };

      const result = sdk.TokenSetPriceResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenSetPriceResult.fromJSON(json);

      expect(restored.pricingSchedule).to.exist();
      expect(restored.groupPower).to.equal(70);
    });

    it('should round-trip pricingSchedule through toObject/fromObject', () => {
      const data = {
        ownerId: testIdentifier,
        pricingSchedule: { $type: 'singlePrice', price: 5000n },
        groupPower: 70,
      };

      const result = sdk.TokenSetPriceResult.fromObject(data);
      const obj = result.toObject();
      expect(obj.pricingSchedule).to.exist();

      const restored = sdk.TokenSetPriceResult.fromObject(obj);
      expect(restored.pricingSchedule).to.exist();
      expect(restored.groupPower).to.equal(70);
    });
  });

  describe('document serialization', () => {
    it('should include document in toJSON when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenSetPriceResult.fromJSON(data);

      expect(result.document).to.exist();
      expect(result.document.id.toBase58()).to.equal(documentJSON.$id);

      const json = result.toJSON();
      expect(json.document).to.exist();
      expect(json.document.$id).to.equal(documentJSON.$id);
      expect(json.document.$ownerId).to.equal(documentJSON.$ownerId);
      expect(json.document.$type).to.equal(documentJSON.$type);
    });

    it('should round-trip document through toJSON/fromJSON', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenSetPriceResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenSetPriceResult.fromJSON(json);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
      expect(restored.groupPower).to.equal(expectedJSON.groupPower);
    });

    it('should not include document in toJSON when absent', () => {
      const result = sdk.TokenSetPriceResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.document).to.be.undefined();
    });

    it('should include document in toObject when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenSetPriceResult.fromJSON(data);

      expect(result.document).to.exist();

      const obj = result.toObject();
      expect(obj.document).to.exist();
    });

    it('should round-trip document through toObject/fromObject', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenSetPriceResult.fromJSON(data);
      const obj = result.toObject();
      const restored = sdk.TokenSetPriceResult.fromObject(obj);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
    });
  });
});
