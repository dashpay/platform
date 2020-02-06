const utils = {
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
