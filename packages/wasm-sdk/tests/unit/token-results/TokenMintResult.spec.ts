import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenMintResult', () => {
  const testIdentifier = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  const documentJSON = {
    $formatVersion: "0",
    $id: '9tSsCqKHTZ8ro16MydChSxgHBukFW36eMLJKKRtebJEn',
    $ownerId: 'CXH2kZCATjvDTnQAPVg28EgPg9WySUvwvnR5ZkmNqY5i',
    $dataContractId: 'GnXgMaiqAwTxh44ccQe8AoCgFvcseHK5CncH3sUorW4X',
    $type: 'note',
    $revision: 1,
    message: 'hello',
  };

  before(async () => {
    await init();
  });

  // Hardcoded expected JSON fixture (camelCase, numbers for balances)
  const expectedJSON = {
    recipientId: testIdentifier,
    newBalance: 1000000,
    groupPower: 75,
    groupActionStatus: 'Completed',
  };

  // Hardcoded expected Object fixture (camelCase, BigInt for balances)
  const expectedObject = {
    recipientId: testIdentifier,
    newBalance: 1000000n,
    groupPower: 75,
    groupActionStatus: 'Completed',
  };

  describe('fromObject()', () => {
    it('should create result from object with all fields', () => {
      const data = {
        recipientId: testIdentifier,
        newBalance: 1000000n,
        groupPower: 75,
        groupActionStatus: 'Completed',
      };

      const result = sdk.TokenMintResult.fromObject(data);

      expect(result.recipientId.toBase58()).to.equal(testIdentifier);
      expect(result.newBalance).to.equal(1000000n);
      expect(result.groupPower).to.equal(75);
      expect(result.groupActionStatus).to.equal('Completed');
      expect(result.document).to.be.undefined();
    });

    it('should handle optional fields being undefined', () => {
      const data = {
        groupPower: 25,
      };

      const result = sdk.TokenMintResult.fromObject(data);
      expect(result.recipientId).to.be.undefined();
      expect(result.newBalance).to.be.undefined();
      expect(result.groupPower).to.equal(25);
      expect(result.groupActionStatus).to.be.undefined();
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        recipientId: testIdentifier,
        newBalance: 1000000n,
        groupPower: 75,
        groupActionStatus: 'Completed',
      };

      const result = sdk.TokenMintResult.fromObject(data);
      const obj = result.toObject();
      expect(obj.groupPower).to.equal(75);
      expect(obj.groupActionStatus).to.equal('Completed');

      const roundtrip = sdk.TokenMintResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(75);
      expect(roundtrip.groupActionStatus).to.equal('Completed');
    });

    it('should produce output matching expected Object fixture', () => {
      const result = sdk.TokenMintResult.fromObject(expectedObject);
      const obj = result.toObject();

      expect(obj.newBalance).to.equal(expectedObject.newBalance);
      expect(obj.groupPower).to.equal(expectedObject.groupPower);
      expect(obj.groupActionStatus).to.equal(expectedObject.groupActionStatus);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        recipientId: testIdentifier,
        newBalance: 1000000,
        groupPower: 50,
        groupActionStatus: 'Pending',
      };

      const result = sdk.TokenMintResult.fromJSON(data);
      expect(result.recipientId.toBase58()).to.equal(testIdentifier);
      expect(result.groupPower).to.equal(50);
      expect(result.groupActionStatus).to.equal('Pending');
    });

    it('should create from JSON fixture and verify all fields via getters', () => {
      const result = sdk.TokenMintResult.fromJSON(expectedJSON);

      expect(result.recipientId.toBase58()).to.equal(testIdentifier);
      expect(result.newBalance).to.equal(1000000n);
      expect(result.groupPower).to.equal(75);
      expect(result.groupActionStatus).to.equal('Completed');
      expect(result.document).to.be.undefined();
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        recipientId: testIdentifier,
        newBalance: 1000000,
        groupPower: 50,
        groupActionStatus: 'Pending',
      };

      const result = sdk.TokenMintResult.fromJSON(data);
      const json = result.toJSON();

      expect(json.recipientId).to.equal(testIdentifier);
      expect(json.groupPower).to.equal(50);
      expect(json.groupActionStatus).to.equal('Pending');

      const roundtrip = sdk.TokenMintResult.fromJSON(json);
      expect(roundtrip.recipientId.toBase58()).to.equal(testIdentifier);
      expect(roundtrip.groupPower).to.equal(50);
    });

    it('should produce output matching expected JSON fixture', () => {
      const result = sdk.TokenMintResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.recipientId).to.equal(expectedJSON.recipientId);
      expect(json.newBalance).to.equal(expectedJSON.newBalance);
      expect(json.groupPower).to.equal(expectedJSON.groupPower);
      expect(json.groupActionStatus).to.equal(expectedJSON.groupActionStatus);
    });
  });

  describe('document serialization', () => {
    it('should include document in toJSON when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenMintResult.fromJSON(data);

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
      const result = sdk.TokenMintResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenMintResult.fromJSON(json);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
      expect(restored.groupPower).to.equal(expectedJSON.groupPower);
    });

    it('should not include document in toJSON when absent', () => {
      const result = sdk.TokenMintResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.document).to.be.undefined();
    });

    it('should include document in toObject when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenMintResult.fromJSON(data);

      expect(result.document).to.exist();

      const obj = result.toObject();
      expect(obj.document).to.exist();
    });

    it('should round-trip document through toObject/fromObject', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenMintResult.fromJSON(data);
      const obj = result.toObject();
      const restored = sdk.TokenMintResult.fromObject(obj);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
    });
  });
});
