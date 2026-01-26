import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { identifier } from './mocks/Identity/index.js';


before(async () => {
  await initWasm();
});

describe('AuthorizedActionTakers', () => {
  describe('serialization / deserialization', () => {
    it('should allows to create AuthorizedActionTakers with NoOne', () => {
      const actionTaker = wasm.AuthorizedActionTakers.NoOne();

      expect(actionTaker).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(actionTaker.takerType).to.deep.equal('NoOne');
    });

    it('should allows to create AuthorizedActionTakers with ContractOwner', () => {
      const actionTaker = wasm.AuthorizedActionTakers.ContractOwner();

      expect(actionTaker).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(actionTaker.takerType).to.deep.equal('ContractOwner');
    });

    it('should allows to create AuthorizedActionTakers with Identity', () => {
      const actionTaker = wasm.AuthorizedActionTakers.Identity(identifier);

      expect(actionTaker).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(actionTaker.takerType).to.deep.equal(`Identity(${identifier})`);
    });

    it('should allows to create AuthorizedActionTakers with MainGroup', () => {
      const actionTaker = wasm.AuthorizedActionTakers.MainGroup();

      expect(actionTaker).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(actionTaker.takerType).to.deep.equal('MainGroup');
    });

    it('should allows to create AuthorizedActionTakers with Group', () => {
      const actionTaker = wasm.AuthorizedActionTakers.Group(12);

      expect(actionTaker).to.be.an.instanceof(wasm.AuthorizedActionTakers);
      expect(actionTaker.takerType).to.deep.equal('Group(12)');
    });
  });

  describe('getters', () => {
    it('should allows to get value with NoOne', () => {
      const actionTaker = wasm.AuthorizedActionTakers.NoOne();

      expect(actionTaker.value).to.deep.equal(undefined);
    });

    it('should allows to get value with ContractOwner', () => {
      const actionTaker = wasm.AuthorizedActionTakers.ContractOwner();

      expect(actionTaker.value).to.deep.equal(undefined);
    });

    it('should allows to get value with Identity', () => {
      const actionTaker = wasm.AuthorizedActionTakers.Identity(identifier);

      expect(actionTaker.value.toBase58()).to.deep.equal(identifier);
    });

    it('should allows to get value with MainGroup', () => {
      const actionTaker = wasm.AuthorizedActionTakers.MainGroup();

      expect(actionTaker.value).to.deep.equal(undefined);
    });

    it('should allows to get value with Group', () => {
      const actionTaker = wasm.AuthorizedActionTakers.Group(12);

      expect(actionTaker.value).to.deep.equal(12);
    });
  });
});
