import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TxOut', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values with pubkey hex', () => {
      const out = new wasm.TxOutWASM(BigInt(100), '76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');
      expect(out.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create from values with pubkey array', () => {
      const out = new wasm.TxOutWASM(BigInt(100), Array.from(Buffer.from('76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac', 'hex')));

      expect(out.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get value', () => {
      const out = new wasm.TxOutWASM(BigInt(100), '76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');

      expect(out.value).to.equal(BigInt(100));
    });

    it('should allow to get script hex', () => {
      const out = new wasm.TxOutWASM(BigInt(100), '76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');

      expect(out.scriptPubKeyHex).to.equal('76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');
    });

    it('should allow to get script bytes', () => {
      const out = new wasm.TxOutWASM(BigInt(100), '76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');

      expect(out.scriptPubKeyBytes).to.deep.equal(Uint8Array.from(Buffer.from('76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac', 'hex')));
    });

    it('should allow to get script asm', () => {
      const out = new wasm.TxOutWASM(BigInt(100), '76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');

      expect(out.getScriptPubKeyASM()).to.deep.equal('OP_DUP OP_HASH160 OP_PUSHBYTES_20 1a486a3855e6dc6dd02874424f53a6f2197b3d45 OP_EQUALVERIFY OP_CHECKSIG');
    });
  });

  describe('setters', () => {
    it('should allow to set value', () => {
      const out = new wasm.TxOutWASM(BigInt(100), '76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');

      out.value = BigInt(101);

      expect(out.value).to.equal(BigInt(101));
    });

    it('should allow to set script hex', () => {
      const out = new wasm.TxOutWASM(BigInt(100), '76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');

      out.scriptPubKeyHex = '16a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac';

      expect(out.scriptPubKeyHex).to.equal('16a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');
    });

    it('should allow to set script bytes', () => {
      const out = new wasm.TxOutWASM(BigInt(100), '76a9141a486a3855e6dc6dd02874424f53a6f2197b3d4588ac');

      out.scriptPubKeyBytes = Array.from(Buffer.from('76a914f995e42d1aa7a31b0106b63e1b896fe9aeeccc9988ac', 'hex'));

      expect(out.scriptPubKeyBytes).to.deep.equal(Uint8Array.from(Buffer.from('76a914f995e42d1aa7a31b0106b63e1b896fe9aeeccc9988ac', 'hex')));
    });
  });
});
