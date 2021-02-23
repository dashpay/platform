const Long = require('long');

class ChainInfo {
  /**
   *
   * @param {Long} [lastBlockHeight]
   * @param {number} [lastCoreChainLockedHeight]
   */
  constructor(
    lastBlockHeight = Long.fromInt(0),
    lastCoreChainLockedHeight = 0,
  ) {
    this.lastBlockHeight = lastBlockHeight;
    this.lastCoreChainLockedHeight = lastCoreChainLockedHeight;
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
   * Get last core chain locked height
   *
   * @return {number}
   */
  getLastCoreChainLockedHeight() {
    return this.lastCoreChainLockedHeight;
  }

  /**
   * Set last core chain locked height
   *
   * @param {number} height
   * @return {ChainInfo}
   */
  setLastCoreChainLockedHeight(height) {
    this.lastCoreChainLockedHeight = height;

    return this;
  }

  /**
   * Populate with data
   *
   * @param {{
   *    lastBlockHeight: string,
   *    lastCoreChainLockedHeight: number,
   * }} object
   * @return {ChainInfo}
   */
  populate(object) {
    this.lastBlockHeight = Long.fromString(object.lastBlockHeight);
    this.lastCoreChainLockedHeight = object.lastCoreChainLockedHeight;

    return this;
  }

  /**
   * Get plain JS object
   *
   * @return {{
   *    lastBlockHeight: string,
   *    lastCoreChainLockedHeight: number,
   * }}
   */
  toJSON() {
    return {
      lastBlockHeight: this.getLastBlockHeight().toString(),
      lastCoreChainLockedHeight: this.getLastCoreChainLockedHeight(),
    };
  }
}

module.exports = ChainInfo;
