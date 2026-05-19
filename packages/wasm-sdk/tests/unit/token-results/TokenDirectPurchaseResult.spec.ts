import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenDirectPurchaseResult', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  // Hardcoded expected JSON fixture (camelCase, numbers for u64 balances)
  const expectedJSON = {
    buyerId: testIdentifier,
    newBalance: 2000000,
    groupPower: 55,
  };

  // Hardcoded expected Object fixture (camelCase, BigInt for balances)
  const expectedObject = {
    buyerId: testIdentifier,
    newBalance: 2000000n,
    groupPower: 55,
  };

  const documentJSON = {
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
        buyerId: testIdentifier,
        newBalance: 2000000n,
        groupPower: 55,
      };

      const result = sdk.TokenDirectPurchaseResult.fromObject(data);

      expect(result.buyerId.toBase58()).to.equal(testIdentifier);
      expect(result.newBalance).to.equal(2000000n);
      expect(result.groupPower).to.equal(55);
    });

    it('should handle group action without balance', () => {
      const data = {
        groupPower: 30,
      };

      const result = sdk.TokenDirectPurchaseResult.fromObject(data);
      expect(result.buyerId).to.be.undefined();
      expect(result.newBalance).to.be.undefined();
      expect(result.groupPower).to.equal(30);
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        buyerId: testIdentifier,
        newBalance: 2000000n,
        groupPower: 55,
      };

      const result = sdk.TokenDirectPurchaseResult.fromObject(data);
      const obj = result.toObject();
      const roundtrip = sdk.TokenDirectPurchaseResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(55);
    });

    it('should produce output matching expected Object fixture', () => {
      const result = sdk.TokenDirectPurchaseResult.fromObject(expectedObject);
      const obj = result.toObject();

      expect(obj.newBalance).to.equal(expectedObject.newBalance);
      expect(obj.groupPower).to.equal(expectedObject.groupPower);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        buyerId: testIdentifier,
        newBalance: 2000000,
        groupPower: 40,
      };

      const result = sdk.TokenDirectPurchaseResult.fromJSON(data);

      expect(result.buyerId.toBase58()).to.equal(testIdentifier);
      expect(result.newBalance).to.equal(2000000n);
      expect(result.groupPower).to.equal(40);
    });

    it('should create from JSON fixture and verify all fields via getters', () => {
      const result = sdk.TokenDirectPurchaseResult.fromJSON(expectedJSON);

      expect(result.buyerId.toBase58()).to.equal(testIdentifier);
      expect(result.newBalance).to.equal(2000000n);
      expect(result.groupPower).to.equal(55);
      expect(result.document).to.be.undefined();
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        buyerId: testIdentifier,
        newBalance: 2000000,
        groupPower: 40,
      };

      const result = sdk.TokenDirectPurchaseResult.fromJSON(data);
      const json = result.toJSON();
      const roundtrip = sdk.TokenDirectPurchaseResult.fromJSON(json);
      expect(roundtrip.buyerId.toBase58()).to.equal(testIdentifier);
      expect(roundtrip.groupPower).to.equal(40);
    });

    it('should produce output matching expected JSON fixture', () => {
      const result = sdk.TokenDirectPurchaseResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.buyerId).to.equal(expectedJSON.buyerId);
      expect(json.newBalance).to.equal(expectedJSON.newBalance);
      expect(json.groupPower).to.equal(expectedJSON.groupPower);
    });
  });

  describe('document serialization', () => {
    it('should include document in toJSON when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenDirectPurchaseResult.fromJSON(data);

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
      const result = sdk.TokenDirectPurchaseResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenDirectPurchaseResult.fromJSON(json);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
      expect(restored.groupPower).to.equal(expectedJSON.groupPower);
    });

    it('should not include document in toJSON when absent', () => {
      const result = sdk.TokenDirectPurchaseResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.document).to.be.undefined();
    });

    it('should include document in toObject when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenDirectPurchaseResult.fromJSON(data);

      expect(result.document).to.exist();

      const obj = result.toObject();
      expect(obj.document).to.exist();
    });

    it('should round-trip document through toObject/fromObject', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenDirectPurchaseResult.fromJSON(data);
      const obj = result.toObject();
      const restored = sdk.TokenDirectPurchaseResult.fromObject(obj);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
    });
  });
});
