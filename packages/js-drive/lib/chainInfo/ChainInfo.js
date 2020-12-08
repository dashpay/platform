const Long = require('long');

class ChainInfo {
  /**
   *
   * @param {Long} [lastBlockHeight]
   */
  constructor(
    lastBlockHeight = Long.fromInt(0),
  ) {
    this.lastBlockHeight = lastBlockHeight;
  }

  /**
   * Get last block height
   *
   * @return {Long}
   */
  getLastBlockHeight() {
    return this.lastBlockHeight;
  }

  /**
   * Set last block height
   *
   * @param {Long} blockHeight
   * @return {ChainInfo}
   */
  setLastBlockHeight(blockHeight) {
    this.lastBlockHeight = blockHeight;

    return this;
  }

  /**
   * Get plain JS object
   *
   * @return {{
   *    lastBlockHeight: string,
   * }}
   */
  toJSON() {
    return {
      lastBlockHeight: this.getLastBlockHeight().toString(),
    };
  }
}

module.exports = ChainInfo;
