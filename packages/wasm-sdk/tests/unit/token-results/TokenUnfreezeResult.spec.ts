import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenUnfreezeResult', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  before(async () => {
    await init();
  });

  // Hardcoded expected JSON fixture (camelCase)
  const expectedJSON = {
    unfrozenIdentityId: testIdentifier,
    groupPower: 70,
  };

  // Hardcoded expected Object fixture (camelCase)
  const expectedObject = {
    unfrozenIdentityId: testIdentifier,
    groupPower: 70,
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

    it('should produce output matching expected Object fixture', () => {
      const result = sdk.TokenUnfreezeResult.fromObject(expectedObject);
      const obj = result.toObject();

      expect(obj.groupPower).to.equal(expectedObject.groupPower);
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

    it('should create from JSON fixture and verify all fields via getters', () => {
      const result = sdk.TokenUnfreezeResult.fromJSON(expectedJSON);

      expect(result.unfrozenIdentityId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(70);
      expect(result.document).to.be.undefined();
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

    it('should produce output matching expected JSON fixture', () => {
      const result = sdk.TokenUnfreezeResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.unfrozenIdentityId).to.equal(expectedJSON.unfrozenIdentityId);
      expect(json.groupPower).to.equal(expectedJSON.groupPower);
    });
  });

  describe('document serialization', () => {
    it('should include document in toJSON when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenUnfreezeResult.fromJSON(data);

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
      const result = sdk.TokenUnfreezeResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenUnfreezeResult.fromJSON(json);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
      expect(restored.groupPower).to.equal(expectedJSON.groupPower);
    });

    it('should not include document in toJSON when absent', () => {
      const result = sdk.TokenUnfreezeResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.document).to.be.undefined();
    });

    it('should include document in toObject when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenUnfreezeResult.fromJSON(data);

      expect(result.document).to.exist();

      const obj = result.toObject();
      expect(obj.document).to.exist();
    });

    it('should round-trip document through toObject/fromObject', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenUnfreezeResult.fromJSON(data);
      const obj = result.toObject();
      const restored = sdk.TokenUnfreezeResult.fromObject(obj);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
    });
  });
});
