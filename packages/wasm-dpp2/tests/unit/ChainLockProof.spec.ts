import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('InstantLock', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create chain lock proof from values', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainlock = new wasm.ChainAssetLockProof(11, outpoint);

      expect(chainlock).to.be.an.instanceof(wasm.ChainAssetLockProof);
    });

    it('should allow to create chain lock proof from object', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );

      // In non-human-readable serde, txid is serialized as Uint8Array
      const txidBytes = Buffer.from(outpoint.txid, 'hex');

      const chainlock = wasm.ChainAssetLockProof.fromObject({
        coreChainLockedHeight: 11,
        outPoint: {
          txid: txidBytes,
          vout: outpoint.vout,
        },
      });

      expect(chainlock).to.be.an.instanceof(wasm.ChainAssetLockProof);
      expect(chainlock.coreChainLockedHeight).to.equal(11);
      expect(chainlock.outPoint.vout).to.equal(1);
    });

    it('should round-trip via object/json/bytes', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainlock = new wasm.ChainAssetLockProof(11, outpoint);

      const object = chainlock.toObject();
      expect(object.coreChainLockedHeight).to.equal(11);
      expect(object.outPoint).to.be.an('object');
      expect(object.outPoint.txid).to.exist();
      expect(object.outPoint.vout).to.equal(1);

      const fromObject = wasm.ChainAssetLockProof.fromObject(object);
      expect(fromObject.coreChainLockedHeight).to.equal(11);
      expect(Array.from(fromObject.outPoint.toBytes())).to.deep.equal(
        Array.from(outpoint.toBytes()),
      );

      const json = chainlock.toJSON();
      expect(json.coreChainLockedHeight).to.equal(11);
      expect(json.outPoint).to.be.a('string');
      expect(json.outPoint).to.include(':'); // "txid:vout" format

      const fromJson = wasm.ChainAssetLockProof.fromJSON(json);
      expect(fromJson.coreChainLockedHeight).to.equal(11);
      expect(Array.from(fromJson.outPoint.toBytes())).to.deep.equal(Array.from(outpoint.toBytes()));

      const bytes = chainlock.toBytes();
      const fromBytes = wasm.ChainAssetLockProof.fromBytes(bytes);
      expect(fromBytes.coreChainLockedHeight).to.equal(11);
      expect(Array.from(fromBytes.outPoint.toBytes())).to.deep.equal(
        Array.from(outpoint.toBytes()),
      );
    });
  });

  describe('getters', () => {
    it('should allow to get coreChainLockedHeight', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainlock = new wasm.ChainAssetLockProof(11, outpoint);

      expect(chainlock.coreChainLockedHeight).to.equal(11);
    });

    it('should allow to get outPoint', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainlock = new wasm.ChainAssetLockProof(11, outpoint);

      expect(chainlock.outPoint.constructor.name).to.equal('OutPoint');
    });
  });

  describe('setters', () => {
    it('should allow to set coreChainLockedHeight', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainlock = new wasm.ChainAssetLockProof(11, outpoint);

      chainlock.coreChainLockedHeight = 33;

      expect(chainlock.coreChainLockedHeight).to.equal(33);
    });

    it('should allow to set outPoint', () => {
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainlock = new wasm.ChainAssetLockProof(11, outpoint);

      const newOutpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        222,
      );

      chainlock.outPoint = newOutpoint;

      expect(chainlock.outPoint.vout).to.equal(222);
      expect(newOutpoint).to.be.an.instanceof(wasm.OutPoint);
    });
  });
});
