import getWasm from './helpers/wasm.js';
import { instantLockBytes, transactionBytes } from './mocks/Locks/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('AssetLockProof', () => {
  describe('serialization / deserialization', () => {
    it('should allow to get instant lock proof via constructor', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const chainlock = new wasm.ChainAssetLockProof(11, outpoint);
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const instantAssetLock = new wasm.AssetLockProof(instantLockProof);
      const chainAssetLock = new wasm.AssetLockProof(chainlock);

      expect(instantAssetLock.constructor.name).to.equal('AssetLockProof');
      expect(instantAssetLock.__wbg_ptr).to.not.equal(0);
      expect(chainAssetLock.constructor.name).to.equal('AssetLockProof');
      expect(chainAssetLock.__wbg_ptr).to.not.equal(0);
    });

    it('shouldn\'t allow to get chain lock proof via constructor', () => {
      try {
        // eslint-disable-next-line
        new wasm.AssetLockProof('chain')
      } catch (e) {
        expect(true).to.be.ok();
        return;
      }
      expect.fail('Expected an error to be thrown');
    });

    it('should allow to create instant lock proof from values', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.constructor.name).to.equal('AssetLockProof');
    });

    it('should allow to create chain lock proof from values', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      expect(chainLockProof.constructor.name).to.equal('AssetLockProof');
    });

    it('should allow to serialize and deserialize asset lock in hex', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const newInstantLockProof = wasm.AssetLockProof.fromHex(instantLockProof.toHex());

      expect(instantLockProof.constructor.name).to.equal('AssetLockProof');
      expect(newInstantLockProof.constructor.name).to.equal('AssetLockProof');

      expect(newInstantLockProof.toObject()).to.deep.equal(instantLockProof.toObject());
    });

    it('should allow to serialize and deserialize asset lock in hex for chain proofs', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      const newChainLockProof = wasm.AssetLockProof.fromHex(chainLockProof.toHex());

      expect(chainLockProof.constructor.name).to.equal('AssetLockProof');
      expect(newChainLockProof.constructor.name).to.equal('AssetLockProof');

      expect(newChainLockProof.toObject()).to.deep.equal(chainLockProof.toObject());
    });

    it('should round-trip asset lock via bytes for instant proofs', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const bytes = instantLockProof.toBytes();
      const restored = wasm.AssetLockProof.fromBytes(bytes);

      expect(restored.toObject()).to.deep.equal(instantLockProof.toObject());
    });

    it('should round-trip asset lock via bytes for chain proofs', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      const bytes = chainLockProof.toBytes();
      const restored = wasm.AssetLockProof.fromBytes(bytes);

      expect(restored.toObject()).to.deep.equal(chainLockProof.toObject());
    });

    it('should recreate asset lock proof from object', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(instantLockBytes, transactionBytes, 0);
      const objectRepresentation = instantLockProof.toObject();

      const restoredProof = wasm.AssetLockProof.fromObject(objectRepresentation);

      expect(restoredProof.toObject()).to.deep.equal(objectRepresentation);
    });

    it('should recreate asset lock proof from JSON', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(instantLockBytes, transactionBytes, 0);
      const jsonRepresentation = instantLockProof.toJSON();

      // Should have type field (0 = Instant)
      expect(jsonRepresentation.type).to.equal(0);
      expect(jsonRepresentation.instantLock).to.be.a('string');
      expect(jsonRepresentation.transaction).to.be.a('string');
      expect(Buffer.from(jsonRepresentation.instantLock, 'base64')).to.deep.equal(Buffer.from(instantLockBytes));
      expect(Buffer.from(jsonRepresentation.transaction, 'base64')).to.deep.equal(Buffer.from(transactionBytes));

      const restoredProof = wasm.AssetLockProof.fromJSON(jsonRepresentation);

      expect(restoredProof.toObject()).to.deep.equal(instantLockProof.toObject());
    });

    it('should recreate chain asset lock proof from JSON', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);
      const jsonRepresentation = chainLockProof.toJSON();

      // Should have type field (1 = Chain)
      expect(jsonRepresentation.type).to.equal(1);

      const restoredProof = wasm.AssetLockProof.fromJSON(jsonRepresentation);

      expect(restoredProof.toObject()).to.deep.equal(chainLockProof.toObject());
    });

    it('should export binary fields as Uint8Array in object form with type field', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const objectRepresentation = instantLockProof.toObject();

      // Should have type field (0 = Instant)
      expect(objectRepresentation.type).to.equal(0);
      expect(objectRepresentation.instantLock).to.be.instanceOf(Uint8Array);
      expect(objectRepresentation.transaction).to.be.instanceOf(Uint8Array);
      expect(objectRepresentation.instantLock).to.deep.equal(instantLockBytes);
      expect(objectRepresentation.transaction).to.deep.equal(transactionBytes);
    });

    it('should export chain lock proof object with type field', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      const objectRepresentation = chainLockProof.toObject();

      // Should have type field (1 = Chain)
      expect(objectRepresentation.type).to.equal(1);
      expect(objectRepresentation.coreChainLockedHeight).to.equal(1);
      // outPoint is {txid, vout} object
      expect(objectRepresentation.outPoint).to.be.an('object');
      expect(objectRepresentation.outPoint.txid).to.exist;
      expect(objectRepresentation.outPoint.vout).to.equal(1);
    });
  });

  describe('getters', () => {
    it('should allow to get lock type', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      expect(instantAssetLockProof.lockTypeName).to.equal('Instant');
      expect(chainLockProof.lockTypeName).to.equal('Chain');
    });

    it('should allow to get lock instances', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);
      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);

      expect(chainLockProof.chainLockProof.constructor.name).to.equal('ChainAssetLockProof');
      expect(instantAssetLockProof.instantLockProof.constructor.name).to.equal('InstantAssetLockProof');
    });

    it('should allow to return object of lock', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);

      // InstantAssetLockProof.toObject() does not include type field
      const expectedInner = {
        instantLock: instantLockBytes,
        transaction: transactionBytes,
        outputIndex: 0,
      };

      // AssetLockProof.toObject() includes type field (0 = Instant)
      const expectedOuter = {
        ...expectedInner,
        type: 0,
      };

      expect(instantLockProof.toObject()).to.deep.equal(expectedInner);
      expect(instantAssetLockProof.toObject()).to.deep.equal(expectedOuter);
    });
  });
});
