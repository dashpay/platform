import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { instantLockBytes, transactionBytes } from './mocks/Locks/index.js';

before(async () => {
  await initWasm();
});

describe('AssetLockProof', () => {
  describe('constructor()', () => {
    it('should allow to get instant lock proof via constructor', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainlock = new wasm.ChainAssetLockProof(11, outpoint);
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const instantAssetLock = new wasm.AssetLockProof(instantLockProof);
      const chainAssetLock = new wasm.AssetLockProof(chainlock);

      expect(instantAssetLock.constructor.name).to.equal('AssetLockProof');
      expect(instantAssetLock).to.be.an.instanceof(wasm.AssetLockProof);
      expect(chainAssetLock.constructor.name).to.equal('AssetLockProof');
      expect(chainAssetLock).to.be.an.instanceof(wasm.AssetLockProof);
    });

    it('should not allow to get chain lock proof via constructor with invalid argument', () => {
      try {
        new (wasm.AssetLockProof as any)('chain');
      } catch {
        expect(true).to.be.ok();
        return;
      }
      expect.fail('Expected an error to be thrown');
    });
  });

  describe('createInstantAssetLockProof()', () => {
    it('should allow to create instant lock proof from values', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      expect(instantLockProof.constructor.name).to.equal('AssetLockProof');
    });
  });

  describe('createChainAssetLockProof()', () => {
    it('should allow to create chain lock proof from values', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );

      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      expect(chainLockProof.constructor.name).to.equal('AssetLockProof');
    });
  });

  describe('toHex()', () => {
    it('should allow to serialize and deserialize asset lock in hex', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const newInstantLockProof = wasm.AssetLockProof.fromHex(instantLockProof.toHex());

      expect(instantLockProof.constructor.name).to.equal('AssetLockProof');
      expect(newInstantLockProof.constructor.name).to.equal('AssetLockProof');

      expect(newInstantLockProof.toObject()).to.deep.equal(instantLockProof.toObject());
    });

    it('should allow to serialize and deserialize asset lock in hex for chain proofs', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      const newChainLockProof = wasm.AssetLockProof.fromHex(chainLockProof.toHex());

      expect(chainLockProof.constructor.name).to.equal('AssetLockProof');
      expect(newChainLockProof.constructor.name).to.equal('AssetLockProof');

      expect(newChainLockProof.toObject()).to.deep.equal(chainLockProof.toObject());
    });
  });

  describe('toBytes()', () => {
    it('should round-trip asset lock via bytes for instant proofs', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const bytes = instantLockProof.toBytes();
      const restored = wasm.AssetLockProof.fromBytes(bytes);

      expect(restored.toObject()).to.deep.equal(instantLockProof.toObject());
    });

    it('should round-trip asset lock via bytes for chain proofs', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      const bytes = chainLockProof.toBytes();
      const restored = wasm.AssetLockProof.fromBytes(bytes);

      expect(restored.toObject()).to.deep.equal(chainLockProof.toObject());
    });
  });

  describe('toObject()', () => {
    it('should recreate asset lock proof from object', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );
      const objectRepresentation = instantLockProof.toObject();

      const restoredProof = wasm.AssetLockProof.fromObject(objectRepresentation);

      expect(restoredProof.toObject()).to.deep.equal(objectRepresentation);
    });

    it('should export internally-tagged {$type:"instant", ...fields} for Instant', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const objectRepresentation = instantLockProof.toObject();

      expect(objectRepresentation.$type).to.equal('instant');
      expect(objectRepresentation.instantLock).to.be.instanceOf(Uint8Array);
      expect(objectRepresentation.transaction).to.be.instanceOf(Uint8Array);
      expect(objectRepresentation.instantLock).to.deep.equal(instantLockBytes);
      expect(objectRepresentation.transaction).to.deep.equal(transactionBytes);
    });

    it('should export internally-tagged {$type:"chain", ...fields} for Chain', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      const objectRepresentation = chainLockProof.toObject();

      expect(objectRepresentation.$type).to.equal('chain');
      expect(objectRepresentation.coreChainLockedHeight).to.equal(1);
      expect(objectRepresentation.outPoint).to.be.an('object');
      expect(objectRepresentation.outPoint.txid).to.exist();
      expect(objectRepresentation.outPoint.vout).to.equal(1);
    });

    it('should flatten the inner proof shape next to the type tag', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);

      const innerExpected = {
        instantLock: instantLockBytes,
        transaction: transactionBytes,
        outputIndex: 0,
      };

      // AssetLockProof.toObject() flattens the inner fields next to `type`
      const outerExpected = {
        $type: 'instant',
        ...innerExpected,
      };

      expect(instantLockProof.toObject()).to.deep.equal(innerExpected);
      expect(instantAssetLockProof.toObject()).to.deep.equal(outerExpected);
    });
  });

  describe('toJSON()', () => {
    it('should recreate asset lock proof from JSON', () => {
      const instantLockProof = wasm.AssetLockProof.createInstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );
      const jsonRepresentation = instantLockProof.toJSON();

      expect(jsonRepresentation.$type).to.equal('instant');
      expect(jsonRepresentation.instantLock).to.be.a('string');
      expect(jsonRepresentation.transaction).to.be.a('string');
      expect(Buffer.from(jsonRepresentation.instantLock, 'base64')).to.deep.equal(
        Buffer.from(instantLockBytes),
      );
      expect(Buffer.from(jsonRepresentation.transaction, 'base64')).to.deep.equal(
        Buffer.from(transactionBytes),
      );

      const restoredProof = wasm.AssetLockProof.fromJSON(jsonRepresentation);

      expect(restoredProof.toObject()).to.deep.equal(instantLockProof.toObject());
    });

    it('should recreate chain asset lock proof from JSON', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);
      const jsonRepresentation = chainLockProof.toJSON();

      expect(jsonRepresentation.$type).to.equal('chain');

      const restoredProof = wasm.AssetLockProof.fromJSON(jsonRepresentation);

      expect(restoredProof.toObject()).to.deep.equal(chainLockProof.toObject());
    });
  });

  describe('lockType', () => {
    it('should allow to get lock type', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      // lockType returns the lowercase wire-shape string, matching
      // `AssetLockProof.toObject().$type` for round-trip consistency.
      expect(instantAssetLockProof.lockType).to.equal('instant');
      expect(chainLockProof.lockType).to.equal('chain');
    });
  });

  describe('chainLockProof', () => {
    it('should allow to get chain lock proof instance', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );

      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      expect(chainLockProof.chainLockProof.constructor.name).to.equal('ChainAssetLockProof');
    });
  });

  describe('instantLockProof', () => {
    it('should allow to get instant lock proof instance', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(
        instantLockBytes,
        transactionBytes,
        0,
      );

      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);

      expect(instantAssetLockProof.instantLockProof.constructor.name).to.equal(
        'InstantAssetLockProof',
      );
    });
  });
});
