/* eslint no-underscore-dangle: ["error", { "allow": ["_getHash"] }] */
/* eslint-disable no-bitwise */
const dashcore = require('@dashevo/dashcore-lib');

module.exports = {
  getCorrectedHash(reversedHashObj) {
    const clone = Buffer.alloc(32);
    reversedHashObj.copy(clone);
    return clone.reverse().toString('hex');
  },
  normalizeHeader(header) {
    if (header instanceof dashcore.BlockHeader) {
      return header;
    }
    if (Buffer.isBuffer(header) && header.length === 80) {
      return new dashcore.BlockHeader(header);
    }
    if (typeof header === 'string' && header.length === 160) {
      const buffer = Buffer.from(header, 'hex');
      return new dashcore.BlockHeader(buffer);
    }

    const el = JSON.parse(JSON.stringify(header));

    return new dashcore.BlockHeader({
      version: el.version,
      prevHash: Buffer.from(el.previousblockhash || el.prevHash, 'hex').reverse(),
      merkleRoot: (el.merkleroot || el.merkleRoot),
      time: el.time,
      bits: parseInt(el.bits, 16),
      nonce: el.nonce,
    });
  },
  getDgwBlock(header) {
    return {
      timestamp: header.timestamp,
      target: header.bits,
    };
  },
  expandTarget(bits) {
    if (bits > 0xffffffff) {
      throw new Error('"bits" may not be larger than 4 bytes');
    }

    const exponent = bits >>> 24;
    if (exponent <= 3) throw new Error('target exponent must be > 3');
    if (exponent > 32) throw new Error('target exponent must be < 32');

    const mantissa = bits & 0x007fffff;
    const target = Buffer.alloc(32).fill(0);
    target.writeUInt32BE(mantissa << 8, 32 - exponent);

    return target;
  },
  validProofOfWork(header) {
    const target = this.expandTarget(header.bits);
    const hash = header._getHash().reverse();
    return hash.compare(target) === -1;
  },
  createBlock(prev, bits) {
    let i = 0;
    let header = null;
    do {
      header = new dashcore.BlockHeader({
        version: 2,
        prevHash: prev ? prev._getHash() : Buffer.alloc(32),
        merkleRoot: Buffer.alloc(32),
        time: prev ? (prev.time + 1) : Math.floor(Date.now() / 1000),
        bits,
        nonce: i += 1,
      });
    } while (!this.validProofOfWork(header));
    return header;
  },
};
