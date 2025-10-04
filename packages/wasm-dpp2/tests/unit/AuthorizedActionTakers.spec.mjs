import getWasm from './helpers/wasm.js';
import { identifier } from './mocks/Identity/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('AuthorizedActionTakers', () => {
  describe('serialization / deserialization', () => {
    it('should allows to create AuthorizedActionTakers with NoOne', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.NoOne();

      expect(actionTaker.__wbg_ptr).to.not.equal(0);
      expect(actionTaker.getTakerType()).to.deep.equal('NoOne');
    });

    it('should allows to create AuthorizedActionTakers with ContractOwner', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.ContractOwner();

      expect(actionTaker.__wbg_ptr).to.not.equal(0);
      expect(actionTaker.getTakerType()).to.deep.equal('ContractOwner');
    });

    it('should allows to create AuthorizedActionTakers with Identity', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.Identity(identifier);

      expect(actionTaker.__wbg_ptr).to.not.equal(0);
      expect(actionTaker.getTakerType()).to.deep.equal(`Identity(${identifier})`);
    });

    it('should allows to create AuthorizedActionTakers with MainGroup', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.MainGroup();

      expect(actionTaker.__wbg_ptr).to.not.equal(0);
      expect(actionTaker.getTakerType()).to.deep.equal('MainGroup');
    });

    it('should allows to create AuthorizedActionTakers with Group', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.Group(12);

      expect(actionTaker.__wbg_ptr).to.not.equal(0);
      expect(actionTaker.getTakerType()).to.deep.equal('Group(12)');
    });
  });

  describe('getters', () => {
    it('should allows to get value with NoOne', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.NoOne();

      expect(actionTaker.getValue()).to.deep.equal(undefined);
    });

    it('should allows to get value with ContractOwner', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.ContractOwner();

      expect(actionTaker.getValue()).to.deep.equal(undefined);
    });

    it('should allows to get value with Identity', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.Identity(identifier);

      expect(actionTaker.getValue().base58()).to.deep.equal(identifier);
    });

    it('should allows to get value with MainGroup', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.MainGroup();

      expect(actionTaker.getValue()).to.deep.equal(undefined);
    });

    it('should allows to get value with Group', () => {
      const actionTaker = wasm.AuthorizedActionTakersWASM.Group(12);

      expect(actionTaker.getValue()).to.deep.equal(12);
    });
  });
});
