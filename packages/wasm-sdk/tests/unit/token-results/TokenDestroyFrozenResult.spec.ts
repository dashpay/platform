import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenDestroyFrozenResult', () => {
  before(async () => {
    await init();
  });

  // Hardcoded expected JSON fixture (camelCase)
  const expectedJSON = {
    groupPower: 90,
  };

  // Hardcoded expected Object fixture (camelCase)
  const expectedObject = {
    groupPower: 90,
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

    it('should produce output matching expected Object fixture', () => {
      const result = sdk.TokenDestroyFrozenResult.fromObject(expectedObject);
      const obj = result.toObject();

      expect(obj.groupPower).to.equal(expectedObject.groupPower);
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

    it('should create from JSON fixture and verify all fields via getters', () => {
      const result = sdk.TokenDestroyFrozenResult.fromJSON(expectedJSON);

      expect(result.groupPower).to.equal(90);
      expect(result.document).to.be.undefined();
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

    it('should produce output matching expected JSON fixture', () => {
      const result = sdk.TokenDestroyFrozenResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.groupPower).to.equal(expectedJSON.groupPower);
    });
  });

  describe('document serialization', () => {
    it('should include document in toJSON when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenDestroyFrozenResult.fromJSON(data);

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
      const result = sdk.TokenDestroyFrozenResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenDestroyFrozenResult.fromJSON(json);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
      expect(restored.groupPower).to.equal(expectedJSON.groupPower);
    });

    it('should not include document in toJSON when absent', () => {
      const result = sdk.TokenDestroyFrozenResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.document).to.be.undefined();
    });

    it('should include document in toObject when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenDestroyFrozenResult.fromJSON(data);

      expect(result.document).to.exist();

      const obj = result.toObject();
      expect(obj.document).to.exist();
    });

    it('should round-trip document through toObject/fromObject', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenDestroyFrozenResult.fromJSON(data);
      const obj = result.toObject();
      const restored = sdk.TokenDestroyFrozenResult.fromObject(obj);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
    });
  });
});
