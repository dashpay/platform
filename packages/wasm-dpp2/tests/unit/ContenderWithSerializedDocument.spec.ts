import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ContenderWithSerializedDocument', () => {
  const identityIdHex = '1111111111111111111111111111111111111111111111111111111111111111';
  const serializedDocBytes = new Uint8Array([1, 2, 3, 4, 5]);

  function createContender(withDoc = true, withTally = true) {
    const identityId = wasm.Identifier.fromHex(identityIdHex);
    return new wasm.ContenderWithSerializedDocument(
      identityId,
      withDoc ? serializedDocBytes : undefined,
      withTally ? 42 : undefined,
    );
  }

  describe('constructor()', () => {
    it('should create with all fields', () => {
      const contender = createContender();
      expect(contender).to.be.instanceOf(wasm.ContenderWithSerializedDocument);
    });

    it('should create with optional fields as undefined', () => {
      const contender = createContender(false, false);
      expect(contender.serializedDocument).to.be.undefined();
      expect(contender.voteTally).to.be.undefined();
    });
  });

  describe('getters', () => {
    it('should return identityId', () => {
      const contender = createContender();
      expect(contender.identityId.toHex()).to.equal(identityIdHex);
    });

    it('should return serializedDocument', () => {
      const contender = createContender();
      const doc = contender.serializedDocument;
      expect(doc).to.be.instanceOf(Uint8Array);
      expect(Array.from(doc)).to.deep.equal([1, 2, 3, 4, 5]);
    });

    it('should return voteTally', () => {
      const contender = createContender();
      expect(contender.voteTally).to.equal(42);
    });
  });

  describe('toJSON()', () => {
    it('should serialize to JSON with $formatVersion tag', () => {
      const contender = createContender();
      const json = contender.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.identityId).to.be.a('string');
      expect(json.voteTally).to.equal(42);
    });

    it('should handle null optional fields', () => {
      const contender = createContender(false, false);
      const json = contender.toJSON();

      expect(json.serializedDocument).to.be.null();
      expect(json.voteTally).to.be.null();
    });
  });

  describe('fromJSON()', () => {
    it('should round-trip via toJSON/fromJSON', () => {
      const contender = createContender();
      const json = contender.toJSON();
      const restored = wasm.ContenderWithSerializedDocument.fromJSON(json);

      expect(restored.identityId.toHex()).to.equal(identityIdHex);
      expect(restored.voteTally).to.equal(42);
    });
  });

  describe('toObject()', () => {
    it('should serialize to Object with $formatVersion tag', () => {
      const contender = createContender();
      const obj = contender.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.identityId).to.be.instanceOf(Uint8Array);
      expect(obj.voteTally).to.equal(42);
    });
  });

  describe('fromObject()', () => {
    it('should round-trip via toObject/fromObject', () => {
      const contender = createContender();
      const obj = contender.toObject();
      const restored = wasm.ContenderWithSerializedDocument.fromObject(obj);

      expect(restored.identityId.toHex()).to.equal(identityIdHex);
      expect(restored.voteTally).to.equal(42);
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const contender = createContender();
      expect(contender.__type).to.equal('ContenderWithSerializedDocument');
    });
  });
});
