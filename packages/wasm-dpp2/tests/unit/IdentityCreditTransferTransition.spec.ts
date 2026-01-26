import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';


before(async () => {
  await initWasm();
});

describe('IdentityCreditTransferTransition', () => {
  describe('serialization / deserialization', () => {
    it('Should create IdentityCreditTransferTransition with empty platform version', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition).to.be.an.instanceof(wasm.IdentityCreditTransfer);
    });

    it('Should create IdentityCreditTransferTransition with non empty platform version', async () => {
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

    it('Should convert IdentityCreditTransferTransition to base64 and back', () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.IdentityCreditTransfer.fromBase64(base64);

      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('getters', () => {
    it('Should return recipientId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.recipientId.toBase58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
    });

    it('Should return senderId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.senderId.toBase58()).to.deep.equal('11111111111111111111111111111111');
    });

    it('Should return amount', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.amount).to.deep.equal(BigInt(100));
    });

    it('Should return nonce', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.nonce).to.deep.equal(BigInt(199));
    });

    it('Should return signature', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('Should return signaturePublicKeyId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });

    it('Should return userFeeIncrease', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      expect(transition.userFeeIncrease).to.deep.equal(0);
    });
  });

  describe('setters', () => {
    it('Should allow to set recipientId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      const recipient = new wasm.Identifier('11111111111111111111111111111111');

      transition.recipientId = recipient;

      expect(transition.recipientId.toBase58()).to.deep.equal('11111111111111111111111111111111');

      transition.recipientId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      expect(transition.recipientId.toBase58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      expect(recipient).to.be.an.instanceof(wasm.Identifier);
    });

    it('Should return senderId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      const sender = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      transition.senderId = sender;

      expect(transition.senderId.toBase58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      transition.senderId = '11111111111111111111111111111111';

      expect(sender).to.be.an.instanceof(wasm.Identifier);
      expect(transition.senderId.toBase58()).to.deep.equal('11111111111111111111111111111111');
    });

    it('Should return amount', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.amount = BigInt(199);

      expect(transition.amount).to.deep.equal(BigInt(199));
    });

    it('Should return nonce', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.nonce = BigInt(1);

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });

    it('Should return signature', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.signature = [1, 1];

      expect(transition.signature).to.deep.equal(Uint8Array.from([1, 1]));
    });

    it('Should return signaturePublicKeyId', async () => {
      const transition = new wasm.IdentityCreditTransfer({
        amount: BigInt(100),
        senderId: '11111111111111111111111111111111',
        recipientId: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        nonce: BigInt(199),
      });

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });

    it('Should return userFeeIncrease', async () => {
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
});
