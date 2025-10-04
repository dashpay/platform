import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('IdentityCreditTransferTransition', () => {
  describe('serialization / deserialization', () => {
    it('Should create IdentityCreditTransferTransition with empty platform version', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      expect(transition.__wbg_ptr).to.not.equal(0);
    });

    it('Should create IdentityCreditTransferTransition with non empty platform version', async () => {
      const sender = new wasm.IdentifierWASM('11111111111111111111111111111111');
      const recipient = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), sender, recipient, BigInt(199), 'platform_v1');

      expect(transition.__wbg_ptr).to.not.equal(0);
      expect(sender.__wbg_ptr).to.not.equal(0);
      expect(recipient.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('Should return recipientId', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      expect(transition.recipientId.base58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
    });

    it('Should return senderId', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      expect(transition.senderId.base58()).to.deep.equal('11111111111111111111111111111111');
    });

    it('Should return amount', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      expect(transition.amount).to.deep.equal(BigInt(100));
    });

    it('Should return nonce', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      expect(transition.nonce).to.deep.equal(BigInt(199));
    });

    it('Should return signature', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('Should return signaturePublicKeyId', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });

    it('Should return userFeeIncrease', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      expect(transition.userFeeIncrease).to.deep.equal(0);
    });
  });

  describe('setters', () => {
    it('Should allow to set recipientId', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      const recipient = new wasm.IdentifierWASM('11111111111111111111111111111111');

      transition.recipientId = recipient;

      expect(transition.recipientId.base58()).to.deep.equal('11111111111111111111111111111111');

      transition.recipientId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

      expect(transition.recipientId.base58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      expect(recipient.__wbg_ptr).to.not.equal(0);
    });

    it('Should return senderId', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      const sender = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      transition.senderId = sender;

      expect(transition.senderId.base58()).to.deep.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      transition.senderId = '11111111111111111111111111111111';

      expect(sender.__wbg_ptr).to.not.equal(0);
      expect(transition.senderId.base58()).to.deep.equal('11111111111111111111111111111111');
    });

    it('Should return amount', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      transition.amount = BigInt(199);

      expect(transition.amount).to.deep.equal(BigInt(199));
    });

    it('Should return nonce', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      transition.nonce = BigInt(1);

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });

    it('Should return signature', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      transition.signature = [1, 1];

      expect(transition.signature).to.deep.equal(Uint8Array.from([1, 1]));
    });

    it('Should return signaturePublicKeyId', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });

    it('Should return userFeeIncrease', async () => {
      const transition = new wasm.IdentityCreditTransferWASM(BigInt(100), '11111111111111111111111111111111', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', BigInt(199));

      transition.userFeeIncrease = 11;

      expect(transition.userFeeIncrease).to.deep.equal(11);
    });
  });
});
