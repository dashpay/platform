import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentityCreditTransfer', () => {
  describe('constructor()', () => {
    it('should create IdentityCreditTransfer with string identifiers', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition).to.be.an.instanceof(wasm.IdentityCreditTransfer);
    });

    it('should create IdentityCreditTransfer with Identifier objects', async () => {
      const sender = new wasm.Identifier('11111111111111111111111111111111');
      const recipient = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: sender,
        recipientId: recipient,
        nonce: BigInt(199),
      });

      expect(transition).to.be.an.instanceof(wasm.IdentityCreditTransfer);
      expect(sender).to.be.an.instanceof(wasm.Identifier);
      expect(recipient).to.be.an.instanceof(wasm.Identifier);
    });
  });

  describe('toBase64()', () => {
    it('should convert IdentityCreditTransfer to base64', () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('fromBase64()', () => {
    it('should create IdentityCreditTransfer from base64', () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      const restored = wasm.IdentityCreditTransfer.fromBase64(base64);

      expect(restored.toBytes()).to.deep.equal(bytes);
    });
  });

  describe('recipientId', () => {
    it('should return recipientId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.recipientId.toBase58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
    });

    it('should set recipientId with Identifier', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      const recipient = new wasm.Identifier('11111111111111111111111111111111');

      transition.recipientId = recipient;

      expect(transition.recipientId.toBase58()).to.deep.equal('11111111111111111111111111111111');
      expect(recipient).to.be.an.instanceof(wasm.Identifier);
    });

    it('should set recipientId with string', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: '11111111111111111111111111111111',
        nonce: BigInt(199),
      });

      transition.recipientId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      expect(transition.recipientId.toBase58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
    });
  });

  describe('senderId', () => {
    it('should return senderId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.senderId.toBase58()).to.deep.equal('11111111111111111111111111111111');
    });

    it('should set senderId with Identifier', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      const sender = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      transition.senderId = sender;

      expect(transition.senderId.toBase58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      expect(sender).to.be.an.instanceof(wasm.Identifier);
    });

    it('should set senderId with string', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.senderId = '11111111111111111111111111111111';

      expect(transition.senderId.toBase58()).to.deep.equal('11111111111111111111111111111111');
    });
  });

  describe('amount', () => {
    it('should return amount', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.amount).to.deep.equal(BigInt(100));
    });

    it('should set amount', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.amount = BigInt(199);

      expect(transition.amount).to.deep.equal(BigInt(199));
    });
  });

  describe('nonce', () => {
    it('should return nonce', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.nonce).to.deep.equal(BigInt(199));
    });

    it('should set nonce', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.nonce = BigInt(1);

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });
  });

  describe('signature', () => {
    it('should return signature', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should set signature', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.signature = [1, 1];

      expect(transition.signature).to.deep.equal(Uint8Array.from([1, 1]));
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should return signaturePublicKeyId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });

    it('should set signaturePublicKeyId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });
  });

  describe('userFeeIncrease', () => {
    it('should return userFeeIncrease', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.userFeeIncrease).to.deep.equal(0);
    });

    it('should set userFeeIncrease', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.userFeeIncrease = 11;

      expect(transition.userFeeIncrease).to.deep.equal(11);
    });
  });

  // Note: toJSON/fromJSON/toObject/fromObject are not available in the current WASM build.
  // The impl_wasm_conversions_inner! macro was added with a js_class name mismatch
  // (IdentityCreditTransferTransition vs IdentityCreditTransfer).
  // The Rust source has been fixed. After the next WASM rebuild, these tests should be enabled.
  //
  // TODO: Enable after WASM rebuild:
  // describe('toJSON()', () => { ... });
  // describe('fromJSON()', () => { ... });
  // describe('toObject()', () => { ... });
  // describe('fromObject()', () => { ... });

  describe('bytes round-trip', () => {
    it('should preserve all field values through bytes serialization', () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      const bytes = transition.toBytes();
      const restored = wasm.IdentityCreditTransfer.fromBytes(bytes);

      expect(restored.amount).to.deep.equal(BigInt(100));
      expect(restored.senderId.toBase58()).to.equal('11111111111111111111111111111111');
      expect(restored.recipientId.toBase58()).to.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      expect(restored.nonce).to.deep.equal(BigInt(199));
      expect(restored.userFeeIncrease).to.equal(0);
      expect(restored.signaturePublicKeyId).to.equal(0);
      expect(restored.signature).to.deep.equal(Uint8Array.from([]));
    });
  });
});
