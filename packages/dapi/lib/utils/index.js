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
  /**
   * Converts either number or {Buffer} to string in hex
   * @param bufferOrNumber {number}
   * @returns {string}
   */
  strHexOrNum(bufferOrNumber) {
    if (typeof bufferOrNumber === 'number') {
      return bufferOrNumber.toString()
    }

    return bufferOrNumber.toString('hex')
  }
};

module.exports = utils;
