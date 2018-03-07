/* eslint no-underscore-dangle: ["error", { "allow": ["_getHash"] }] */
const DashUtil = require('dash-util');
const bitcore = require('bitcore-lib-dash');

module.exports = {

  getCorrectedHash(reversedHashObj) {
    const clone = Buffer.alloc(32);
    reversedHashObj.copy(clone);
    return clone.reverse().toString('hex');
  },
  getDifficulty(target) {
    // TODO
    return 1.0 / target;
  },
  normalizeHeader(header) {
    const el = JSON.parse(JSON.stringify(header));

    return new bitcore.BlockHeader({
      version: el.version,
      prevHash: DashUtil.toHash(el.previousblockhash),
      merkleRoot: el.merkleroot,
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
