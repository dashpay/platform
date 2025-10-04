import getWasm from './helpers/wasm.js';
import { instantLockBytes, transactionBytes } from './mocks/Locks/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('InstantLock', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create InstantLock from values', () => {
      const instantLockProof = new wasm.InstantAssetLockProofWASM(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to convert to object', () => {
      const instantLockProof = new wasm.InstantAssetLockProofWASM(instantLockBytes, transactionBytes, 0);

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

      const instantLockProof = wasm.InstantAssetLockProofWASM.fromObject(lockObject);

      expect(instantLockProof.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get output', () => {
      const instantLockProof = new wasm.InstantAssetLockProofWASM(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.getOutput().constructor.name).to.deep.equal('TxOutWASM');
    });

    it('should allow to convert to get OutPoint', () => {
      const instantLockProof = new wasm.InstantAssetLockProofWASM(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.getOutPoint().constructor.name).to.deep.equal('OutPointWASM');
    });

    it('should allow to get output index', () => {
      const instantLockProof = new wasm.InstantAssetLockProofWASM(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.outputIndex).to.deep.equal(0);
    });

    it('should allow to get instant lock', () => {
      const instantLockProof = new wasm.InstantAssetLockProofWASM(instantLockBytes, transactionBytes, 0);

      expect(instantLockProof.instantLock.constructor.name).to.deep.equal('InstantLockWASM');
    });
  });

  describe('setters', () => {
    it('should allow to set output index', () => {
      const instantLockProof = new wasm.InstantAssetLockProofWASM(instantLockBytes, transactionBytes, 0);

      instantLockProof.outputIndex = 12;

      expect(instantLockProof.outputIndex).to.deep.equal(12);
    });

    it('should allow to set instant lock', () => {
      const instantLockProof = new wasm.InstantAssetLockProofWASM(instantLockBytes, transactionBytes, 0);

      const newInstantLockProof = new wasm.InstantLockWASM(
        0,
        [],
        'dbdb604952d08184b55d48c915ed78aadc81dbc5cc98e8b4821abe5b4bbcbecb',
        '00000000000000151e0fe3ab9a12c57402153c9f476236148364ec4337213101',
        'a9f131626c49a2f183b7a2f563ad1dc50ac8220190dbedb805209b608eb864e01d62f18bc9faa60a8b8a27f5a0c7c8b914fa3a14360a2f25558ee0e0a693b18faccbb59ec39b9b3cae430e0b76eb080752ce103df76537a1a583680a5914529d',
      );

      instantLockProof.instantLock = newInstantLockProof;

      expect(instantLockProof.instantLock.version).to.deep.equal(0);
      expect(newInstantLockProof.__wbg_ptr).to.not.equal(0);
    });
  });
});
