import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('OutPoint', () => {
  describe('constructor()', () => {
    it('should create OutPoint from txid and vout', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint).to.be.an.instanceof(wasm.OutPoint);
    });
  });

  describe('fromBytes()', () => {
    it('should create OutPoint from bytes', () => {
      const txIdBytes = Buffer.from('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 'hex');

      // 32 bytes for txId and 4 bytes for vout
      const bytes = [...txIdBytes.reverse(), ...[0, 0, 0, 1].reverse()];

      const outpoint = wasm.OutPoint.fromBytes(bytes);

      expect(outpoint).to.be.an.instanceof(wasm.OutPoint);
    });
  });

  describe('txid', () => {
    it('should return transaction id', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint.txid).to.equal('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d');
    });
  });

  describe('vout', () => {
    it('should return output index', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint.vout).to.equal(1);
    });
  });

  describe('toBytes()', () => {
    it('should return bytes representation', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      const txIdBytes = Buffer.from('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 'hex');

      // 32 bytes for txId and 4 bytes for vout
      const bytes = [...txIdBytes.reverse(), ...[0, 0, 0, 1].reverse()];

      expect(outpoint.toBytes()).to.deep.equal(Uint8Array.from(bytes));
    });
  });

  describe('toBase64()', () => {
    it('should return base64 representation', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      const base64 = outpoint.toBase64();
      const bytes = outpoint.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('fromBase64()', () => {
    it('should create OutPoint from base64 string', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      const base64 = outpoint.toBase64();
      const bytes = outpoint.toBytes();

      expect(wasm.OutPoint.fromBase64(base64).toBytes()).to.deep.equal(bytes);
    });
  });
});
