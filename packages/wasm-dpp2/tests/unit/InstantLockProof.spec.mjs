import getWasm from './helpers/wasm.js';
import { instantLockBytes, transactionBytes } from './mocks/Locks/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('InstantLock', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create InstantLock from values', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to convert to object', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const expected = {
        instantLock: instantLockBytes,
        transaction: transactionBytes,
        outputIndex: 0,
      };

      expect(instantLockProof.toObject()).to.deep.equal(expected);
    });

    it('should allow to create from object', () => {
      const lockObject = {
        instantLock: instantLockBytes,
        transaction: transactionBytes,
        outputIndex: 0,
      };

      const instantLockProof = wasm.InstantAssetLockProof.fromObject(lockObject);

      expect(instantLockProof.__wbg_ptr).to.not.equal(0);
    });

    it('should round-trip via toJSON/fromJSON', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const json = instantLockProof.toJSON();

      // JSON format should have base64 strings
      expect(json.instantLock).to.be.a('string');
      expect(json.transaction).to.be.a('string');
      expect(json.outputIndex).to.equal(0);

      // Verify base64 decodes to original bytes
      expect(Buffer.from(json.instantLock, 'base64')).to.deep.equal(Buffer.from(instantLockBytes));
      expect(Buffer.from(json.transaction, 'base64')).to.deep.equal(Buffer.from(transactionBytes));

      // Round-trip via fromJSON
      const restored = wasm.InstantAssetLockProof.fromJSON(json);

      expect(restored.toObject()).to.deep.equal(instantLockProof.toObject());
    });

    it('should round-trip via toObject/fromObject', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const obj = instantLockProof.toObject();

      // Object format should have Uint8Array
      expect(obj.instantLock).to.be.instanceOf(Uint8Array);
      expect(obj.transaction).to.be.instanceOf(Uint8Array);
      expect(obj.outputIndex).to.equal(0);

      // Round-trip via fromObject
      const restored = wasm.InstantAssetLockProof.fromObject(obj);

      expect(restored.toObject()).to.deep.equal(obj);
    });
  });

  describe('getters', () => {
    it('should allow to get output as bytes', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const output = instantLockProof.output;
      expect(output).to.be.instanceOf(Uint8Array);
      expect(output.length).to.be.greaterThan(0);
    });

    it('should allow to convert to get OutPoint', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.outPoint.constructor.name).to.deep.equal('OutPoint');
    });

    it('should allow to get output index', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.outputIndex).to.deep.equal(0);
    });

    it('should allow to get instant lock as bytes', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.instantLock).to.be.instanceOf(Uint8Array);
      expect(instantLockProof.instantLock).to.deep.equal(instantLockBytes);
    });
  });

  describe('setters', () => {
    it('should allow to set output index', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      instantLockProof.outputIndex = 12;

      expect(instantLockProof.outputIndex).to.deep.equal(12);
    });

    it('should allow to set instant lock from bytes', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      // Set and verify the instant lock bytes round-trip
      instantLockProof.instantLock = instantLockBytes;

      expect(instantLockProof.instantLock).to.deep.equal(instantLockBytes);
    });
  });
});
