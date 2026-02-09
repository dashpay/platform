import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentityCreditWithdrawalTransition', () => {
  describe('constructor()', () => {
    it('should create IdentityCreditWithdrawalTransition', () => {
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
  });

  describe('toBase64()', () => {
    it('should convert IdentityCreditWithdrawalTransition to base64', () => {
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
    });
  });

  describe('fromBase64()', () => {
    it('should create IdentityCreditWithdrawalTransition from base64', () => {
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

      const restored = wasm.IdentityCreditWithdrawalTransition.fromBase64(base64);

      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('outputScript', () => {
    it('should return outputScript', () => {
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

    it('should set outputScript', () => {
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
  });

  describe('pooling', () => {
    it('should return pooling', () => {
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

    it('should set pooling', () => {
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
  });

  describe('identityId', () => {
    it('should return identityId', () => {
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

    it('should set identityId', () => {
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
  });

  describe('userFeeIncrease', () => {
    it('should return userFeeIncrease', () => {
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

    it('should set userFeeIncrease', () => {
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
  });

  describe('nonce', () => {
    it('should return nonce', () => {
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

    it('should set nonce', () => {
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
  });

  describe('amount', () => {
    it('should return amount', () => {
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

    it('should set amount', () => {
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
  });

  describe('signature', () => {
    it('should return signature', () => {
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

    it('should set signature', () => {
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
  });

  describe('signaturePublicKeyId', () => {
    it('should return signaturePublicKeyId', () => {
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

    it('should set signaturePublicKeyId', () => {
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
