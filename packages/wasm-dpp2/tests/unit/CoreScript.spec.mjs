import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('CoreScript', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from bytes', () => {
      const script = wasm.CoreScript.fromBytes(Buffer.from('76a914c3dbfd40e7f8a4845c2f8e868a167c984049764988ac', 'hex'));

      expect(script._wbg_ptr).to.not.equal(0);
    });

    it('should allow to create P2PKH', () => {
      const script = wasm.CoreScript.newP2PKH([195, 219, 253, 64, 231, 248, 164, 132, 92, 47, 142, 134, 138, 22, 124, 152, 64, 73, 118, 73]);

      expect(script._wbg_ptr).to.not.equal(0);
    });

    it('should allow to create P2SH', () => {
      const script = wasm.CoreScript.newP2SH([195, 219, 253, 64, 231, 248, 164, 132, 92, 47, 142, 134, 138, 22, 124, 152, 64, 73, 118, 73]);

      expect(script._wbg_ptr).to.not.equal(0);
    });

    it('should allow to convert to asm P2PKH', () => {
      const script = wasm.CoreScript.newP2PKH([195, 219, 253, 64, 231, 248, 164, 132, 92, 47, 142, 134, 138, 22, 124, 152, 64, 73, 118, 73]);

      expect(script.ASMString()).to.equal('OP_DUP OP_HASH160 OP_PUSHBYTES_20 c3dbfd40e7f8a4845c2f8e868a167c9840497649 OP_EQUALVERIFY OP_CHECKSIG');
    });

    it('should allow to convert to adddress', () => {
      const script = wasm.CoreScript.fromBytes(Buffer.from('76a9142de40f87177f6e167fb9fcda9a3b3c64fc42468f88ac', 'hex'));

      expect(script.toAddress(wasm.Network.Testnet)).to.equal('yQW6TmUFef5CDyhEYwjoN8aUTMmKLYYNDm');
    });
  });
});
