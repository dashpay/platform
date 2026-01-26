import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('OutPoint', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint).to.be.an.instanceof(wasm.OutPoint);
    });

    it('should allow to create from bytes', () => {
      const txIdBytes = Buffer.from('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 'hex');

      // 32 bytes for txId and 4 bytes for vout
      const bytes = [...txIdBytes.reverse(), ...[0, 0, 0, 1].reverse()];

      const outpoint = wasm.OutPoint.fromBytes(bytes);

      expect(outpoint).to.be.an.instanceof(wasm.OutPoint);
    });
  });

  describe('getters', () => {
    it('should allow to get txid', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint.txid).to.equal('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d');
    });

    it('should allow to get VOUT', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint.vout).to.equal(1);
    });

    it('should allow to get bytes', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      const txIdBytes = Buffer.from('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 'hex');

      // 32 bytes for txId and 4 bytes for vout
      const bytes = [...txIdBytes.reverse(), ...[0, 0, 0, 1].reverse()];

      expect(outpoint.toBytes()).to.deep.equal(Uint8Array.from(bytes));
    });

    it('should allow to get base64 representation', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      const base64 = outpoint.toBase64();
      const bytes = outpoint.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));
      expect(Buffer.from(wasm.OutPoint.fromBase64(base64).toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });
});
