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

  describe('__type', () => {
    it('should return correct __type', () => {
      const info = new wasm.TokenPaymentInfo({ tokenContractPosition: 0 });
      expect(info.__type).to.equal('TokenPaymentInfo');
    });
  });
});
