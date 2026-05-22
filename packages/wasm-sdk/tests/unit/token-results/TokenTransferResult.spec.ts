import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';

describe('TokenTransferResult', () => {
  before(async () => {
    await init();
  });

  // Hardcoded expected JSON fixture (camelCase, numbers for u64 balances)
  const expectedJSON = {
    senderBalance: 900000,
    recipientBalance: 100000,
    groupPower: 25,
  };

  // Hardcoded expected Object fixture (camelCase, BigInt for balances)
  const expectedObject = {
    senderBalance: 900000n,
    recipientBalance: 100000n,
    groupPower: 25,
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
        senderBalance: 900000n,
        recipientBalance: 100000n,
        groupPower: 25,
      };

      const result = sdk.TokenTransferResult.fromObject(data);

      expect(result.senderBalance).to.equal(900000n);
      expect(result.recipientBalance).to.equal(100000n);
      expect(result.groupPower).to.equal(25);
    });

    it('should handle group action without balances', () => {
      const data = {
        groupPower: 50,
      };

      const result = sdk.TokenTransferResult.fromObject(data);
      expect(result.senderBalance).to.be.undefined();
      expect(result.recipientBalance).to.be.undefined();
      expect(result.groupPower).to.equal(50);
      expect(result.document).to.be.undefined();
    });
  });

  describe('toObject()', () => {
    it('should round-trip through toObject/fromObject', () => {
      const data = {
        senderBalance: 900000n,
        recipientBalance: 100000n,
        groupPower: 25,
      };

      const result = sdk.TokenTransferResult.fromObject(data);
      const obj = result.toObject();
      const roundtrip = sdk.TokenTransferResult.fromObject(obj);
      expect(roundtrip.groupPower).to.equal(25);
    });

    it('should produce output matching expected Object fixture', () => {
      const result = sdk.TokenTransferResult.fromObject(expectedObject);
      const obj = result.toObject();

      expect(obj.senderBalance).to.equal(expectedObject.senderBalance);
      expect(obj.recipientBalance).to.equal(expectedObject.recipientBalance);
      expect(obj.groupPower).to.equal(expectedObject.groupPower);
    });
  });

  describe('fromJSON()', () => {
    it('should create result from JSON', () => {
      const data = {
        senderBalance: 900000,
        recipientBalance: 100000,
        groupPower: 25,
      };

      const result = sdk.TokenTransferResult.fromJSON(data);

      expect(result.senderBalance).to.equal(900000n);
      expect(result.recipientBalance).to.equal(100000n);
      expect(result.groupPower).to.equal(25);
    });

    it('should create from JSON fixture and verify all fields via getters', () => {
      const result = sdk.TokenTransferResult.fromJSON(expectedJSON);

      expect(result.senderBalance).to.equal(900000n);
      expect(result.recipientBalance).to.equal(100000n);
      expect(result.groupPower).to.equal(25);
      expect(result.document).to.be.undefined();
    });
  });

  describe('toJSON()', () => {
    it('should round-trip through toJSON/fromJSON', () => {
      const data = {
        senderBalance: 900000,
        recipientBalance: 100000,
        groupPower: 25,
      };

      const result = sdk.TokenTransferResult.fromJSON(data);
      const json = result.toJSON();
      const roundtrip = sdk.TokenTransferResult.fromJSON(json);
      expect(roundtrip.groupPower).to.equal(25);
    });

    it('should produce output matching expected JSON fixture', () => {
      const result = sdk.TokenTransferResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.senderBalance).to.equal(expectedJSON.senderBalance);
      expect(json.recipientBalance).to.equal(expectedJSON.recipientBalance);
      expect(json.groupPower).to.equal(expectedJSON.groupPower);
    });
  });

  describe('document serialization', () => {
    it('should include document in toJSON when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenTransferResult.fromJSON(data);

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
      const result = sdk.TokenTransferResult.fromJSON(data);
      const json = result.toJSON();
      const restored = sdk.TokenTransferResult.fromJSON(json);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
      expect(restored.groupPower).to.equal(expectedJSON.groupPower);
    });

    it('should not include document in toJSON when absent', () => {
      const result = sdk.TokenTransferResult.fromJSON(expectedJSON);
      const json = result.toJSON();

      expect(json.document).to.be.undefined();
    });

    it('should include document in toObject when present', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenTransferResult.fromJSON(data);

      expect(result.document).to.exist();

      const obj = result.toObject();
      expect(obj.document).to.exist();
    });

    it('should round-trip document through toObject/fromObject', () => {
      const data = { ...expectedJSON, document: documentJSON };
      const result = sdk.TokenTransferResult.fromJSON(data);
      const obj = result.toObject();
      const restored = sdk.TokenTransferResult.fromObject(obj);

      expect(restored.document).to.exist();
      expect(restored.document.id.toBase58()).to.equal(documentJSON.$id);
    });
  });
});
