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

      expect(restored.toBytes()).to.deep.equal(bytes);
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

  describe('coreFeePerByte', () => {
    it('should return coreFeePerByte', () => {
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

      expect(transition.coreFeePerByte).to.equal(1);
    });
  });

  describe('toJSON()', () => {
    it('should produce expected JSON structure', () => {
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

      const json = transition.toJSON();

      expect(json.$formatVersion).to.equal('1');
      expect(json.identityId).to.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      expect(json.amount).to.equal(111);
      expect(json.coreFeePerByte).to.equal(1);
      expect(json.pooling).to.equal('never');
      expect(json.outputScript).to.equal('dqkUAQEBAQEBAQEBAQEBAQEBAQEBAQGIrA==');
      expect(json.nonce).to.equal(1);
      expect(json.userFeeIncrease).to.equal(1);
      expect(json.signature).to.equal('');
      expect(json.signaturePublicKeyId).to.equal(0);
    });
  });

  describe('fromJSON()', () => {
    it('should restore transition from JSON and verify getters', () => {
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

      const json = transition.toJSON();
      const restored = wasm.IdentityCreditWithdrawalTransition.fromJSON(json);

      expect(restored.identityId.toBase58()).to.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      expect(restored.amount).to.deep.equal(BigInt(111));
      expect(restored.coreFeePerByte).to.equal(1);
      expect(restored.pooling).to.equal('Never');
      expect(restored.outputScript.toString()).to.equal('dqkUAQEBAQEBAQEBAQEBAQEBAQEBAQGIrA==');
      expect(restored.nonce).to.deep.equal(BigInt(1));
      expect(restored.userFeeIncrease).to.equal(1);
      expect(restored.signaturePublicKeyId).to.equal(0);
      expect(restored.signature).to.deep.equal(Uint8Array.from([]));
    });
  });

  describe('toObject()', () => {
    it('should produce expected object structure', () => {
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

      const obj = transition.toObject();

      expect(obj.$formatVersion).to.equal('1');
      expect(obj.identityId).to.be.instanceOf(Uint8Array);
      expect(obj.identityId.length).to.equal(32);
      expect(obj.amount).to.deep.equal(BigInt(111));
      expect(obj.coreFeePerByte).to.equal(1);
      expect(obj.pooling).to.equal(0);
      expect(obj.outputScript).to.be.instanceOf(Uint8Array);
      expect(obj.outputScript.length).to.equal(25);
      expect(obj.nonce).to.deep.equal(BigInt(1));
      expect(obj.userFeeIncrease).to.equal(1);
      expect(obj.signature).to.be.instanceOf(Uint8Array);
      expect(obj.signaturePublicKeyId).to.equal(0);
    });
  });
});
