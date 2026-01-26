import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentityCreditWithdrawalTransition', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create IdentityCreditWithdrawalTransition', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(identifier).to.be.an.instanceof(wasm.Identifier);
      expect(script).to.be.an.instanceof(wasm.CoreScript);
      expect(transition).to.be.an.instanceof(wasm.IdentityCreditWithdrawalTransition);
    });

    it('should convert IdentityCreditWithdrawalTransition to base64 and back', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.IdentityCreditWithdrawalTransition.fromBase64(base64);

      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('getters', () => {
    it('should allow to get outputScript', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(transition.outputScript.toString()).to.deep.equal('dqkUAQEBAQEBAQEBAQEBAQEBAQEBAQGIrA==');
    });

    it('should allow to get pooling', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(transition.pooling).to.deep.equal('Never');
    });

    it('should allow to get identityId', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(transition.identityId.toBase58()).to.deep.equal(identifier.toBase58());
    });

    it('should allow to get userFeeIncrease', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(transition.userFeeIncrease).to.deep.equal(1);
    });

    it('should allow to get nonce', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });

    it('should allow to get amount', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(transition.amount).to.deep.equal(BigInt(111));
    });

    it('should allow to get signature', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should allow to get signaturePublicKeyId', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });
  });

  describe('setters', () => {
    it('should allow to set outputScript', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      const script2 = wasm.CoreScript.fromP2PKH([2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]);

      expect(transition.outputScript.toString()).to.deep.equal(script.toString());

      transition.outputScript = script2;

      expect(transition.outputScript.toString()).to.deep.equal(script2.toString());
      expect(transition.outputScript.toString()).to.not.deep.equal(script.toString());
    });

    it('should allow to set pooling', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      transition.pooling = 'Standard';

      expect(transition.pooling).to.deep.equal('Standard');
    });

    it('should allow to set identityId', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      const identifier2 = new wasm.Identifier('11SAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      transition.identityId = identifier2;

      expect(transition.identityId.toBase58()).to.deep.equal(identifier2.toBase58());
    });

    it('should allow to set userFeeIncrease', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      transition.userFeeIncrease = 999;

      expect(transition.userFeeIncrease).to.deep.equal(999);
    });

    it('should allow to set nonce', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      transition.nonce = BigInt(1111);

      expect(transition.nonce).to.deep.equal(BigInt(1111));
    });

    it('should allow to get amount', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      transition.amount = BigInt(2222);

      expect(transition.amount).to.deep.equal(BigInt(2222));
    });

    it('should allow to get signature', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      transition.signature = Uint8Array.from([1, 2, 3]);

      expect(transition.signature).to.deep.equal(Uint8Array.from([1, 2, 3]));
    });

    it('should allow to get signaturePublicKeyId', () => {
      const identifier = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScript.fromP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransition({
        identityId: identifier,
        amount: BigInt(111),
        coreFeePerByte: 1,
        pooling: 'never',
        outputScript: script,
        nonce: BigInt(1),
        userFeeIncrease: 1,
      });

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });
  });
});
