/* eslint no-underscore-dangle: ["error", { "allow": ["_getHash"] }] */
/* eslint-disable no-bitwise */
const DashUtil = require('dash-util');
const bitcore = require('bitcore-lib-dash');
const BN = require('bn.js');

const temp = {
  double256(target) {
    const B192 = 0x1000000000000000000000000000000000000000000000000;
    const B128 = 0x100000000000000000000000000000000;
    const B64 = 0x10000000000000000;
    const B0 = 0x1;

    let n = 0;
    let hi = null;
    let lo = null;

    hi = target.readUInt32LE(28, true);
    lo = target.readUInt32LE(24, true);
    n += ((hi * 0x100000000) + lo) * B192;

    hi = target.readUInt32LE(20, true);
    lo = target.readUInt32LE(16, true);
    n += ((hi * 0x100000000) + lo) * B128;

    hi = target.readUInt32LE(12, true);
    lo = target.readUInt32LE(8, true);
    n += ((hi * 0x100000000) + lo) * B64;

    hi = target.readUInt32LE(4, true);
    lo = target.readUInt32LE(0, true);
    n += ((hi * 0x100000000) + lo) * B0;

    return n;
  },
  fromCompact(compact) {
    if (compact === 0) { return new BN(0); }

    const exponent = compact >>> 24;
    const negative = (compact >>> 23) & 1;

    let mantissa = compact & 0x7fffff;
    let num;

    if (exponent <= 3) {
      mantissa >>>= 8 * (3 - exponent);
      num = new BN(mantissa);
    } else {
      num = new BN(mantissa);
      num.iushln(8 * (exponent - 3));
    }

    if (negative) { num.ineg(); }

    return num;
  },
  getTarget(bits) {
    const target = this.fromCompact(bits);

    if (target.isNeg()) { throw new Error('Target is negative.'); }

    if (target.isZero()) {
      throw new Error('Target is zero.');
    }

    return this.double256(target.toArrayLike(Buffer, 'le', 32));
  },
};

module.exports = {

  getCorrectedHash(reversedHashObj) {
    const clone = Buffer.alloc(32);
    reversedHashObj.copy(clone);
    return clone.reverse().toString('hex');
  },
  getDifficulty(bitsInDecimal) {
    const maxTargetBits = 0x1e0ffff0;
    return temp.getTarget(maxTargetBits) / temp.getTarget(bitsInDecimal);
  },
  normalizeHeader(header) {
    if (header instanceof bitcore.BlockHeader) {
      return header;
    }

    const el = JSON.parse(JSON.stringify(header));

    return new bitcore.BlockHeader({
      version: el.version,
      prevHash: DashUtil.toHash(el.previousblockhash || el.prevHash),
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
  validProofOfWork(header) {
    const target = DashUtil.expandTarget(header.bits);
    const hash = header._getHash().reverse();
    return hash.compare(target) !== 1;
  },
  createBlock(prev, bits) {
    let i = 0;
    let header = null;
    do {
      header = new bitcore.BlockHeader({
        version: 2,
        prevHash: prev ? prev._getHash() : DashUtil.nullHash,
        merkleRoot: DashUtil.nullHash,
        time: prev ? (prev.time + 1) : Math.floor(Date.now() / 1000),
        bits,
        nonce: i += 1,
      });
    } while (!this.validProofOfWork(header));
    return header;
  },
};
