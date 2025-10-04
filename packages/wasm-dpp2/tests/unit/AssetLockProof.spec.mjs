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
        expect(true).to.be.ok;
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

      const newInstantLockProof = wasm.AssetLockProof.fromHex(instantLockProof.hex());

      expect(instantLockProof.constructor.name).to.equal('AssetLockProof');
      expect(newInstantLockProof.constructor.name).to.equal('AssetLockProof');

      expect(newInstantLockProof.toObject()).to.deep.equal(instantLockProof.toObject());
    });
  });

  describe('getters', () => {
    it('should allow to get lock type', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);
      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);

      expect(instantAssetLockProof.getLockType()).to.equal('Instant');
      expect(chainLockProof.getLockType()).to.equal('Chain');
    });

    it('should allow to get lock instances', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const chainLockProof = wasm.AssetLockProof.createChainAssetLockProof(1, outpoint);
      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);

      expect(chainLockProof.getChainLockProof().constructor.name).to.equal('ChainAssetLockProof');
      expect(instantAssetLockProof.getInstantLockProof().constructor.name).to.equal('InstantAssetLockProof');
    });

    it('should allow to return object of lock', () => {
      const instantLockProof = new wasm.InstantAssetLockProof(instantLockBytes, transactionBytes, 0);

      const instantAssetLockProof = new wasm.AssetLockProof(instantLockProof);

      const expected = {
        instantLock: instantLockBytes,
        transaction: transactionBytes,
        outputIndex: 0,
      };

      expect(instantLockProof.toObject()).to.deep.equal(expected);
      expect(instantAssetLockProof.toObject()).to.deep.equal(expected);
    });
  });
});
