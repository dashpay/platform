import { Buffer } from 'buffer';
import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('TokenContractInfo', () => {
  const contractIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  // TokenContractInfo is a versioned enum tagged with `$formatVersion`.
  // V0 -> "0". Inner V0 fields (contractId, tokenContractPosition) flatten
  // at the top level via internal tagging.
  function createJsonFixture() {
    const contractId = wasm.Identifier.fromHex(contractIdHex);
    return {
      $formatVersion: '0',
      contractId: contractId.toBase58(),
      tokenContractPosition: 3,
    };
  }

  function createObjectFixture() {
    return {
      $formatVersion: '0',
      contractId: new Uint8Array(Buffer.from(contractIdHex, 'hex')),
      tokenContractPosition: 3,
    };
  }

  describe('fromJSON()', () => {
    it('should create from JSON and verify getters', () => {
      const json = createJsonFixture();
      const info = wasm.TokenContractInfo.fromJSON(json);

      expect(info.contractId.toHex()).to.equal(contractIdHex);
      expect(info.tokenContractPosition).to.equal(3);
    });
  });

  describe('toJSON()', () => {
    it('should round-trip via fromJSON/toJSON', () => {
      const json = createJsonFixture();
      const info = wasm.TokenContractInfo.fromJSON(json);
      const result = info.toJSON();

      expect(result.contractId).to.equal(json.contractId);
      expect(result.tokenContractPosition).to.equal(3);
    });
  });

  describe('fromObject()', () => {
    it('should create from Object and verify getters', () => {
      const obj = createObjectFixture();
      const info = wasm.TokenContractInfo.fromObject(obj);

      expect(info.contractId.toHex()).to.equal(contractIdHex);
      expect(info.tokenContractPosition).to.equal(3);
    });
  });

  describe('toObject()', () => {
    it('should round-trip via fromObject/toObject', () => {
      const obj = createObjectFixture();
      const info = wasm.TokenContractInfo.fromObject(obj);
      const result = info.toObject();

      expect(result.contractId).to.be.instanceOf(Uint8Array);
      expect(Buffer.from(result.contractId).toString('hex')).to.equal(contractIdHex);
      expect(result.tokenContractPosition).to.equal(3);
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const info = wasm.TokenContractInfo.fromJSON(createJsonFixture());
      expect(info.__type).to.equal('TokenContractInfo');
    });
  });
});
