import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('CoreScript', () => {
  // Common test data
  const pubKeyHash = [
    195, 219, 253, 64, 231, 248, 164, 132,
    92, 47, 142, 134, 138, 22, 124, 152, 64, 73, 118, 73,
  ];

  describe('fromBytes()', () => {
    it('should allow to create from bytes', () => {
      const scriptHex = '76a914c3dbfd40e7f8a4845c2f8e868a167c984049764988ac';
      const script = wasm.CoreScript.fromBytes(Buffer.from(scriptHex, 'hex'));

      expect(script._wbg_ptr).to.not.equal(0);
    });
  });

  describe('fromP2PKH()', () => {
    it('should allow to create P2PKH', () => {
      const script = wasm.CoreScript.fromP2PKH(pubKeyHash);

      expect(script._wbg_ptr).to.not.equal(0);
    });
  });

  describe('fromP2SH()', () => {
    it('should allow to create P2SH', () => {
      const script = wasm.CoreScript.fromP2SH(pubKeyHash);

      expect(script._wbg_ptr).to.not.equal(0);
    });
  });

  describe('toASMString()', () => {
    it('should allow to convert to asm P2PKH', () => {
      const script = wasm.CoreScript.fromP2PKH(pubKeyHash);

      expect(script.toASMString()).to.equal('OP_DUP OP_HASH160 OP_PUSHBYTES_20 c3dbfd40e7f8a4845c2f8e868a167c9840497649 OP_EQUALVERIFY OP_CHECKSIG');
    });
  });

  describe('toAddress()', () => {
    it('should allow to convert to address', () => {
      const script = wasm.CoreScript.fromBytes(Buffer.from('76a9142de40f87177f6e167fb9fcda9a3b3c64fc42468f88ac', 'hex'));

      expect(script.toAddress(wasm.Network.Testnet)).to.equal('yQW6TmUFef5CDyhEYwjoN8aUTMmKLYYNDm');
    });
  });

  describe('toBase64()', () => {
    it('should allow to get base64 representation', () => {
      const script = wasm.CoreScript.fromBytes(Buffer.from('76a9142de40f87177f6e167fb9fcda9a3b3c64fc42468f88ac', 'hex'));

      const base64 = script.toBase64();
      const bytes = script.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));
    });
  });
});
