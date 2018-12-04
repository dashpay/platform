const utils = {
  getCorrectedHash: (reversedHashObj) => {
    const clone = Buffer.alloc(32);
    reversedHashObj.copy(clone);
    return clone.reverse().toString('hex');
  },
  /**
   * @param {string} network
   * @return {boolean}
   */
  isRegtest(network) {
    return network === 'regtest';
  },
  /**
   * @param {string} network
   * @return {boolean}
   */
  isDevnet(network) {
    return /^devnet/.test(network);
  },
};

module.exports = utils;
