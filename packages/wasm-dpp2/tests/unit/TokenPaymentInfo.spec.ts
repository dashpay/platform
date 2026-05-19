import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('TokenPaymentInfo', () => {
  const paymentContractIdHex = '1111111111111111111111111111111111111111111111111111111111111111';

  describe('constructor()', () => {
    it('should create with minimal options', () => {
      const info = new wasm.TokenPaymentInfo({
        tokenContractPosition: 2,
      });

      expect(info.tokenContractPosition).to.equal(2);
      expect(info.paymentTokenContractId).to.be.undefined();
      expect(info.minimumTokenCost).to.be.undefined();
      expect(info.maximumTokenCost).to.be.undefined();
    });

    it('should create with all options', () => {
      const paymentId = wasm.Identifier.fromHex(paymentContractIdHex);
      const info = new wasm.TokenPaymentInfo({
        paymentTokenContractId: paymentId,
        tokenContractPosition: 5,
        minimumTokenCost: 100n,
        maximumTokenCost: 1000n,
        gasFeesPaidBy: 'documentOwner',
      });

      expect(info.tokenContractPosition).to.equal(5);
      expect(info.paymentTokenContractId).to.not.be.undefined();
      expect(info.paymentTokenContractId.toHex()).to.equal(paymentContractIdHex);
      expect(info.minimumTokenCost).to.equal(100n);
      expect(info.maximumTokenCost).to.equal(1000n);
    });
  });

  describe('setters', () => {
    it('should set tokenContractPosition', () => {
      const info = new wasm.TokenPaymentInfo({ tokenContractPosition: 1 });
      info.tokenContractPosition = 10;
      expect(info.tokenContractPosition).to.equal(10);
    });

    it('should set paymentTokenContractId', () => {
      const info = new wasm.TokenPaymentInfo({ tokenContractPosition: 1 });
      const paymentId = wasm.Identifier.fromHex(paymentContractIdHex);
      info.paymentTokenContractId = paymentId;
      expect(info.paymentTokenContractId.toHex()).to.equal(paymentContractIdHex);
    });

    it('should clear paymentTokenContractId with undefined', () => {
      const paymentId = wasm.Identifier.fromHex(paymentContractIdHex);
      const info = new wasm.TokenPaymentInfo({
        paymentTokenContractId: paymentId,
        tokenContractPosition: 1,
      });
      info.paymentTokenContractId = undefined;
      expect(info.paymentTokenContractId).to.be.undefined();
    });

    it('should set minimumTokenCost', () => {
      const info = new wasm.TokenPaymentInfo({ tokenContractPosition: 1 });
      info.minimumTokenCost = 500n;
      expect(info.minimumTokenCost).to.equal(500n);
    });

    it('should set maximumTokenCost', () => {
      const info = new wasm.TokenPaymentInfo({ tokenContractPosition: 1 });
      info.maximumTokenCost = 2000n;
      expect(info.maximumTokenCost).to.equal(2000n);
    });
  });

  describe('toJSON()', () => {
    it('should serialize with $formatVersion and camelCase fields', () => {
      const paymentId = wasm.Identifier.fromHex(paymentContractIdHex);
      const info = new wasm.TokenPaymentInfo({
        paymentTokenContractId: paymentId,
        tokenContractPosition: 3,
        minimumTokenCost: 50n,
        maximumTokenCost: 1000n,
        gasFeesPaidBy: 'contractOwner',
      });

      const json = info.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.paymentTokenContractId).to.be.a('string');
      expect(json.tokenContractPosition).to.equal(3);
      expect(json.minimumTokenCost).to.equal(50);
      expect(json.maximumTokenCost).to.equal(1000);
      expect(json.gasFeesPaidBy).to.equal('ContractOwner');

      info.free();
    });

    it('should serialize null optional fields', () => {
      const info = new wasm.TokenPaymentInfo({
        tokenContractPosition: 0,
      });

      const json = info.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.paymentTokenContractId).to.be.null();
      expect(json.tokenContractPosition).to.equal(0);
      expect(json.minimumTokenCost).to.be.null();
      expect(json.maximumTokenCost).to.be.null();
      expect(json.gasFeesPaidBy).to.equal('DocumentOwner');

      info.free();
    });
  });

  describe('fromJSON()', () => {
    it('should deserialize from JSON', () => {
      const paymentId = wasm.Identifier.fromHex(paymentContractIdHex);
      const info = new wasm.TokenPaymentInfo({
        paymentTokenContractId: paymentId,
        tokenContractPosition: 3,
        minimumTokenCost: 50n,
        maximumTokenCost: 1000n,
        gasFeesPaidBy: 'contractOwner',
      });

      const json = info.toJSON();
      const restored = wasm.TokenPaymentInfo.fromJSON(json);

      expect(restored.tokenContractPosition).to.equal(3);
      expect(restored.paymentTokenContractId.toHex()).to.equal(paymentContractIdHex);
      expect(restored.minimumTokenCost).to.equal(50n);
      expect(restored.maximumTokenCost).to.equal(1000n);

      info.free();
      restored.free();
    });

    it('should round-trip through JSON', () => {
      const info = new wasm.TokenPaymentInfo({
        tokenContractPosition: 7,
        maximumTokenCost: 500n,
      });

      const json = info.toJSON();
      const restored = wasm.TokenPaymentInfo.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      info.free();
      restored.free();
    });
  });

  describe('toObject()', () => {
    it('should serialize with $formatVersion and Uint8Array identifiers', () => {
      const paymentId = wasm.Identifier.fromHex(paymentContractIdHex);
      const info = new wasm.TokenPaymentInfo({
        paymentTokenContractId: paymentId,
        tokenContractPosition: 3,
        maximumTokenCost: 1000n,
        gasFeesPaidBy: 'contractOwner',
      });

      const obj = info.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.paymentTokenContractId).to.be.instanceOf(Uint8Array);
      expect(obj.tokenContractPosition).to.equal(3);
      expect(obj.maximumTokenCost).to.equal(1000n);
      expect(obj.gasFeesPaidBy).to.equal('ContractOwner');

      info.free();
    });
  });

  describe('fromObject()', () => {
    it('should round-trip through Object', () => {
      const paymentId = wasm.Identifier.fromHex(paymentContractIdHex);
      const info = new wasm.TokenPaymentInfo({
        paymentTokenContractId: paymentId,
        tokenContractPosition: 2,
        minimumTokenCost: 10n,
        maximumTokenCost: 500n,
      });

      const obj = info.toObject();
      const restored = wasm.TokenPaymentInfo.fromObject(obj);

      expect(restored.tokenContractPosition).to.equal(2);
      expect(restored.paymentTokenContractId.toHex()).to.equal(paymentContractIdHex);
      expect(restored.minimumTokenCost).to.equal(10n);
      expect(restored.maximumTokenCost).to.equal(500n);

      info.free();
      restored.free();
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const info = new wasm.TokenPaymentInfo({ tokenContractPosition: 0 });
      expect(info.__type).to.equal('TokenPaymentInfo');
    });
  });
});
