import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('ActionTaker', () => {
  const id1Hex = '1111111111111111111111111111111111111111111111111111111111111111';
  const id2Hex = '2222222222222222222222222222222222222222222222222222222222222222';

  describe('constructor()', () => {
    it('should create SingleIdentity from a single Identifier', () => {
      const id = wasm.Identifier.fromHex(id1Hex);
      const taker = new wasm.ActionTaker(id);

      expect(taker.takerType).to.equal('SingleIdentity');
    });

    it('should create SpecifiedIdentities from an array of Identifiers', () => {
      const id1 = wasm.Identifier.fromHex(id1Hex);
      const id2 = wasm.Identifier.fromHex(id2Hex);
      const taker = new wasm.ActionTaker([id1, id2]);

      expect(taker.takerType).to.equal('SpecifiedIdentities');
    });
  });

  describe('value', () => {
    it('should return Identifier for SingleIdentity', () => {
      const id = wasm.Identifier.fromHex(id1Hex);
      const taker = new wasm.ActionTaker(id);

      const value = taker.value;
      expect(value).to.be.instanceOf(wasm.Identifier);
      expect(value.toHex()).to.equal(id1Hex);
    });

    it('should return array for SpecifiedIdentities', () => {
      const id1 = wasm.Identifier.fromHex(id1Hex);
      const id2 = wasm.Identifier.fromHex(id2Hex);
      const taker = new wasm.ActionTaker([id1, id2]);

      const value = taker.value;
      expect(Array.isArray(value)).to.be.true();
      expect(value).to.have.length(2);
    });
  });

  describe('setter', () => {
    it('should change value', () => {
      const id1 = wasm.Identifier.fromHex(id1Hex);
      const taker = new wasm.ActionTaker(id1);
      expect(taker.takerType).to.equal('SingleIdentity');

      const id2 = wasm.Identifier.fromHex(id2Hex);
      taker.value = [id1, id2];
      expect(taker.takerType).to.equal('SpecifiedIdentities');
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const id = wasm.Identifier.fromHex(id1Hex);
      const taker = new wasm.ActionTaker(id);
      expect(taker.__type).to.equal('ActionTaker');
    });
  });
});
