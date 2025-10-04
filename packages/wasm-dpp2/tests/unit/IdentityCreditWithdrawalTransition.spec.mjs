import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('IdentityCreditWithdrawalTransition', () => {
  describe('serialization / deserialization', () => {
    it('Should allow to create IdentityCreditWithdrawalTransition', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(identifier.__wbg_ptr).to.not.equal(0);
      expect(script.__wbg_ptr).to.not.equal(0);
      expect(transition.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('Should allow to get outputScript', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(transition.outputScript.toString()).to.deep.equal('dqkUAQEBAQEBAQEBAQEBAQEBAQEBAQGIrA==');
    });

    it('Should allow to get pooling', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(transition.pooling).to.deep.equal('Never');
    });

    it('Should allow to get identityId', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(transition.identityId.base58()).to.deep.equal(identifier.base58());
    });

    it('Should allow to get userFeeIncrease', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(transition.userFeeIncrease).to.deep.equal(1);
    });

    it('Should allow to get nonce', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(transition.nonce).to.deep.equal(BigInt(1));
    });

    it('Should allow to get amount', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(transition.amount).to.deep.equal(BigInt(111));
    });

    it('Should allow to get signature', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('Should allow to get signaturePublicKeyId', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      expect(transition.signaturePublicKeyId).to.deep.equal(0);
    });
  });

  describe('setters', () => {
    it('Should allow to set outputScript', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      const script2 = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      transition.outputScript = script2;

      expect(transition.outputScript.toString()).to.deep.equal(script2.toString());
    });

    it('Should allow to set pooling', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      transition.pooling = 'Standard';

      expect(transition.pooling).to.deep.equal('Standard');
    });

    it('Should allow to set identityId', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      const identifier2 = new wasm.IdentifierWASM('11SAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      transition.identityId = identifier2;

      expect(transition.identityId.base58()).to.deep.equal(identifier2.base58());
    });

    it('Should allow to set userFeeIncrease', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      transition.userFeeIncrease = 999;

      expect(transition.userFeeIncrease).to.deep.equal(999);
    });

    it('Should allow to set nonce', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      transition.nonce = BigInt(1111);

      expect(transition.nonce).to.deep.equal(BigInt(1111));
    });

    it('Should allow to get amount', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      transition.amount = BigInt(2222);

      expect(transition.amount).to.deep.equal(BigInt(2222));
    });

    it('Should allow to get signature', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      transition.signature = Uint8Array.from([1, 2, 3]);

      expect(transition.signature).to.deep.equal(Uint8Array.from([1, 2, 3]));
    });

    it('Should allow to get signaturePublicKeyId', () => {
      const identifier = new wasm.IdentifierWASM('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      const script = wasm.CoreScriptWASM.newP2PKH([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);

      const transition = new wasm.IdentityCreditWithdrawalTransitionWASM(identifier, BigInt(111), 1, 'never', script, BigInt(1), 1);

      transition.signaturePublicKeyId = 11;

      expect(transition.signaturePublicKeyId).to.deep.equal(11);
    });
  });
});
