import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenFreezeResult', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  // Hardcoded expected JSON fixture (camelCase)
  const expectedJSON = {
    frozenIdentityId: testIdentifier,
    groupPower: 80,
  };

  // Hardcoded expected Object fixture (camelCase)
  const expectedObject = {
    frozenIdentityId: testIdentifier,
    groupPower: 80,
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
        frozenIdentityId: testIdentifier,
        groupPower: 80,
      };

      const result = sdk.TokenFreezeResult.fromObject(data);

      expect(result.frozenIdentityId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(80);
    });

    it('should handle group action without identity', () => {
      const data = {
        groupPower: 45,
      };

      const result = sdk.TokenFreezeResult.fromObject(data);
      expect(result.frozenIdentityId).to.be.undefined();
      expect(result.groupPower).to.equal(45);
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        frozenIdentityId: testIdentifier,
        groupPower: 80,
      };

      const result = sdk.TokenFreezeResult.fromObject(data);
      const obj = result.toObject();
      const roundtrip = sdk.TokenFreezeResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(80);
    });

    it('should produce output matching expected Object fixture', () => {
      const result = sdk.TokenFreezeResult.fromObject(expectedObject);
      const obj = result.toObject();

      expect(obj.groupPower).to.equal(expectedObject.groupPower);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        frozenIdentityId: testIdentifier,
        groupPower: 60,
      };

      const result = sdk.TokenFreezeResult.fromJSON(data);

      expect(result.frozenIdentityId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(60);
    });

    it('should create from JSON fixture and verify all fields via getters', () => {
      const result = sdk.TokenFreezeResult.fromJSON(expectedJSON);

      expect(result.frozenIdentityId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(80);
      expect(result.document).to.be.undefined();
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        frozenIdentityId: testIdentifier,
        groupPower: 60,
      };

      const result = sdk.TokenFreezeResult.fromJSON(data);
      const json = result.toJSON();
      const roundtrip = sdk.TokenFreezeResult.fromJSON(json);
      expect(roundtrip.frozenIdentityId.toBase58()).to.equal(testIdentifier);
      expect(roundtrip.groupPower).to.equal(60);
    });

    it('should produce output matching expected JSON fixture', () => {
      const result = sdk.TokenFreezeResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.frozenIdentityId).to.equal(expectedJSON.frozenIdentityId);
      expect(json.groupPower).to.equal(expectedJSON.groupPower);
    });
  });

  describe('document serialization', () => {
    it('should include document in toJSON when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenFreezeResult.fromJSON(data);

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
      const result = sdk.TokenFreezeResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenFreezeResult.fromJSON(json);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
      expect(restored.groupPower).to.equal(expectedJSON.groupPower);
    });

    it('should not include document in toJSON when absent', () => {
      const result = sdk.TokenFreezeResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.document).to.be.undefined();
    });

    it('should include document in toObject when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenFreezeResult.fromJSON(data);

      expect(result.document).to.exist();

      const obj = result.toObject();
      expect(obj.document).to.exist();
    });

    it('should round-trip document through toObject/fromObject', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenFreezeResult.fromJSON(data);
      const obj = result.toObject();
      const restored = sdk.TokenFreezeResult.fromObject(obj);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
    });
  });
});
