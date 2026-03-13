import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import dataContractJson from './mocks/DataContract/json.ts';

before(async () => {
  await initWasm();
});

describe('VerifiedDataContract', () => {
  function createFromJson() {
    return wasm.VerifiedDataContract.fromJSON({ dataContract: dataContractJson });
  }

  describe('fromJSON()', () => {
    it('should create from JSON with dataContract wrapper', () => {
      const verified = createFromJson();

      expect(verified.dataContract).to.be.instanceOf(wasm.DataContract);
      expect(verified.dataContract.id.toBase58()).to.equal(dataContractJson.id);
    });
  });

  describe('toJSON()', () => {
    it('should round-trip via fromJSON/toJSON', () => {
      const verified = createFromJson();
      const json = verified.toJSON();

      expect(json).to.have.property('dataContract');
      expect(json.dataContract.id).to.equal(dataContractJson.id);
    });
  });

  describe('fromObject()', () => {
    it('should create from Object and verify getter', () => {
      const verified = createFromJson();
      const obj = verified.toObject();
      const restored = wasm.VerifiedDataContract.fromObject(obj);

      expect(restored.dataContract).to.be.instanceOf(wasm.DataContract);
      expect(restored.dataContract.id.toBase58()).to.equal(dataContractJson.id);
    });
  });

  describe('toObject()', () => {
    it('should produce object with dataContract property', () => {
      const verified = createFromJson();
      const obj = verified.toObject();

      expect(obj).to.have.property('dataContract');
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const verified = createFromJson();
      expect(verified.__type).to.equal('VerifiedDataContract');
    });
  });
});
