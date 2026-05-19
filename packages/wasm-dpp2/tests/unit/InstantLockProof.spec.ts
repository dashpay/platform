import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { instantLockBytes, transactionBytes } from './mocks/Locks/index.js';

before(async () => {
  await initWasm();
});

describe('InstantAssetLockProof', () => {
  describe('constructor()', () => {
    it('should create InstantAssetLockProof from values', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      expect(instantLockProof).to.be.an.instanceof(wasm.InstantAssetLockProof);
    });
  });

  describe('toObject()', () => {
    it('should convert to object with Uint8Array values', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const expected = {
        instantLock: instantLockBytes,
        transaction: transactionBytes,
        outputIndex: 0,
      };

      expect(instantLockProof.toObject()).to.deep.equal(expected);
    });

    it('should return object with Uint8Array for bytes fields', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const obj = instantLockProof.toObject();

      // Object format should have Uint8Array
      expect(obj.instantLock).to.be.instanceOf(Uint8Array);
      expect(obj.transaction).to.be.instanceOf(Uint8Array);
      expect(obj.outputIndex).to.equal(0);
    });
  });

  describe('fromObject()', () => {
    it('should create from object', () => {
      const lockObject = {
        instantLock: instantLockBytes,
        transaction: transactionBytes,
        outputIndex: 0,
      };

      const instantLockProof = wasm.InstantAssetLockProof.fromObject(lockObject);

      expect(instantLockProof).to.be.an.instanceof(wasm.InstantAssetLockProof);
    });

    it('should round-trip via toObject/fromObject', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const obj = instantLockProof.toObject();
      const restored = wasm.InstantAssetLockProof.fromObject(obj);

      expect(restored.toObject()).to.deep.equal(obj);
    });
  });

  describe('toJSON()', () => {
    it('should convert to JSON with base64 strings', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const json = instantLockProof.toJSON();

      // JSON format should have base64 strings
      expect(json.instantLock).to.be.a('string');
      expect(json.transaction).to.be.a('string');
      expect(json.outputIndex).to.equal(0);

      // Verify base64 decodes to original bytes
      expect(Buffer.from(json.instantLock, 'base64')).to.deep.equal(Buffer.from(instantLockBytes));
      expect(Buffer.from(json.transaction, 'base64')).to.deep.equal(Buffer.from(transactionBytes));
    });

    it('should match hardcoded expected JSON', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const expectedJSON = {
        instantLock: Buffer.from(instantLockBytes).toString('base64'),
        transaction: Buffer.from(transactionBytes).toString('base64'),
        outputIndex: 0,
      };

      expect(instantLockProof.toJSON()).to.deep.equal(expectedJSON);
    });
  });

  describe('fromJSON()', () => {
    it('should round-trip via toJSON/fromJSON', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const json = instantLockProof.toJSON();
      const restored = wasm.InstantAssetLockProof.fromJSON(json);

      expect(restored.toObject()).to.deep.equal(instantLockProof.toObject());
    });

    it('should create from hardcoded JSON fixture and verify via getters', () => {
      const expectedInstantLockBase64 = Buffer.from(instantLockBytes).toString('base64');
      const expectedTransactionBase64 = Buffer.from(transactionBytes).toString('base64');

      const jsonFixture = {
        instantLock: expectedInstantLockBase64,
        transaction: expectedTransactionBase64,
        outputIndex: 0,
      };

      const proof = wasm.InstantAssetLockProof.fromJSON(jsonFixture);

      expect(proof).to.be.an.instanceof(wasm.InstantAssetLockProof);
      expect(proof.outputIndex).to.equal(0);
      expect(proof.instantLock).to.deep.equal(instantLockBytes);
      expect(proof.transaction).to.deep.equal(transactionBytes);
    });
  });

  describe('output', () => {
    it('should return output as bytes', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const { output } = instantLockProof;
      expect(output).to.be.instanceOf(Uint8Array);
      expect(output.length).to.be.greaterThan(0);
    });
  });

  describe('outPoint', () => {
    it('should return OutPoint instance', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      expect(instantLockProof.outPoint.constructor.name).to.deep.equal('OutPoint');
    });
  });

  describe('outputIndex', () => {
    it('should return output index', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      expect(instantLockProof.outputIndex).to.deep.equal(0);
    });

    it('should set output index', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      instantLockProof.outputIndex = 12;

      expect(instantLockProof.outputIndex).to.deep.equal(12);
    });
  });

  describe('instantLock', () => {
    it('should return instant lock as bytes', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      expect(instantLockProof.instantLock).to.be.instanceOf(Uint8Array);
      expect(instantLockProof.instantLock).to.deep.equal(instantLockBytes);
    });

    it('should set instant lock from bytes', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      // Set and verify the instant lock bytes round-trip
      instantLockProof.instantLock = instantLockBytes;

      expect(instantLockProof.instantLock).to.deep.equal(instantLockBytes);
    });
  });
});
