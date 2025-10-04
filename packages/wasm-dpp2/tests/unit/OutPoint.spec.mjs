import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('OutPoint', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create from bytes', () => {
      const txIdBytes = Buffer.from('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 'hex');

      // 32 bytes for txId and 4 bytes for vout
      const bytes = [...txIdBytes.reverse(), ...[0, 0, 0, 1].reverse()];

      const outpoint = wasm.OutPoint.fromBytes(bytes);

      expect(outpoint.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get txid', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint.getTXID()).to.equal('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d');
    });

    it('should allow to get VOUT', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      expect(outpoint.getVOUT()).to.equal(1);
    });

    it('should allow to get bytes', () => {
      const outpoint = new wasm.OutPoint('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 1);

      const txIdBytes = Buffer.from('e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d', 'hex');

      // 32 bytes for txId and 4 bytes for vout
      const bytes = [...txIdBytes.reverse(), ...[0, 0, 0, 1].reverse()];

      expect(outpoint.bytes()).to.deep.equal(Uint8Array.from(bytes));
    });
  });
});
