import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenBurnResult', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  // Hardcoded expected JSON fixture (camelCase, numbers for u64 balances)
  const expectedJSON = {
    ownerId: testIdentifier,
    remainingBalance: 500000,
    groupPower: 100,
    groupActionStatus: 'ActionNeeded',
  };

  // Hardcoded expected Object fixture (camelCase, BigInt for balances)
  const expectedObject = {
    ownerId: testIdentifier,
    remainingBalance: 500000n,
    groupPower: 100,
    groupActionStatus: 'ActionNeeded',
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
        ownerId: testIdentifier,
        remainingBalance: 500000n,
        groupPower: 100,
        groupActionStatus: 'ActionNeeded',
      };

      const result = sdk.TokenBurnResult.fromObject(data);

      expect(result.ownerId.toBase58()).to.equal(testIdentifier);
      expect(result.remainingBalance).to.equal(500000n);
      expect(result.groupPower).to.equal(100);
      expect(result.groupActionStatus).to.equal('ActionNeeded');
    });

    it('should handle null optional fields', () => {
      const data = {
        ownerId: testIdentifier,
        remainingBalance: 100n,
      };

      const result = sdk.TokenBurnResult.fromObject(data);
      expect(result.ownerId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.be.undefined();
      expect(result.groupActionStatus).to.be.undefined();
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        ownerId: testIdentifier,
        remainingBalance: 500000n,
        groupPower: 100,
        groupActionStatus: 'ActionNeeded',
      };

      const result = sdk.TokenBurnResult.fromObject(data);
      const obj = result.toObject();
      const roundtrip = sdk.TokenBurnResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(100);
    });

    it('should produce output matching expected Object fixture', () => {
      const result = sdk.TokenBurnResult.fromObject(expectedObject);
      const obj = result.toObject();

      expect(obj.remainingBalance).to.equal(expectedObject.remainingBalance);
      expect(obj.groupPower).to.equal(expectedObject.groupPower);
      expect(obj.groupActionStatus).to.equal(expectedObject.groupActionStatus);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        ownerId: testIdentifier,
        remainingBalance: 500000,
        groupPower: 80,
        groupActionStatus: 'Completed',
      };

      const result = sdk.TokenBurnResult.fromJSON(data);
      expect(result.ownerId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(80);
    });

    it('should create from JSON fixture and verify all fields via getters', () => {
      const result = sdk.TokenBurnResult.fromJSON(expectedJSON);

      expect(result.ownerId.toBase58()).to.equal(testIdentifier);
      expect(result.remainingBalance).to.equal(500000n);
      expect(result.groupPower).to.equal(100);
      expect(result.groupActionStatus).to.equal('ActionNeeded');
      expect(result.document).to.be.undefined();
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        ownerId: testIdentifier,
        remainingBalance: 500000,
        groupPower: 80,
        groupActionStatus: 'Completed',
      };

      const result = sdk.TokenBurnResult.fromJSON(data);
      const json = result.toJSON();

      expect(json.ownerId).to.equal(testIdentifier);
      expect(json.groupPower).to.equal(80);

      const roundtrip = sdk.TokenBurnResult.fromJSON(json);
      expect(roundtrip.ownerId.toBase58()).to.equal(testIdentifier);
      expect(roundtrip.groupPower).to.equal(80);
    });

    it('should produce output matching expected JSON fixture', () => {
      const result = sdk.TokenBurnResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.ownerId).to.equal(expectedJSON.ownerId);
      expect(json.remainingBalance).to.equal(expectedJSON.remainingBalance);
      expect(json.groupPower).to.equal(expectedJSON.groupPower);
      expect(json.groupActionStatus).to.equal(expectedJSON.groupActionStatus);
    });
  });

  describe('document serialization', () => {
    it('should include document in toJSON when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenBurnResult.fromJSON(data);

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
      const result = sdk.TokenBurnResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenBurnResult.fromJSON(json);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
      expect(restored.groupPower).to.equal(expectedJSON.groupPower);
    });

    it('should not include document in toJSON when absent', () => {
      const result = sdk.TokenBurnResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.document).to.be.undefined();
    });

    it('should include document in toObject when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenBurnResult.fromJSON(data);

      expect(result.document).to.exist();

      const obj = result.toObject();
      expect(obj.document).to.exist();
    });

    it('should round-trip document through toObject/fromObject', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenBurnResult.fromJSON(data);
      const obj = result.toObject();
      const restored = sdk.TokenBurnResult.fromObject(obj);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
    });
  });
});
