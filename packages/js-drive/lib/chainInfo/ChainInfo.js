const Long = require('long');

class ChainInfo {
  /**
   *
   * @param {Long} [lastBlockHeight]
   * @param {number} [creditsDistributionPool]
   */
  constructor(
    lastBlockHeight = Long.fromInt(0),
    creditsDistributionPool = 0,
  ) {
    this.lastBlockHeight = lastBlockHeight;
    this.creditsDistributionPool = creditsDistributionPool;
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
   * Set credits distribution pool
   *
   * @param {number} credits
   * @return {ChainInfo}
   */
  setCreditsDistributionPool(credits) {
    this.creditsDistributionPool = credits;

    return this;
  }

  /**
   * Get credits distribution pool
   *
   * @return {number}
   */
  getCreditsDistributionPool() {
    return this.creditsDistributionPool;
  }

  /**
   * Get plain JS object
   *
   * @return {{
   *    lastBlockHeight: string,
   *    creditsDistributionPool: number,
   * }}
   */
  toJSON() {
    return {
      lastBlockHeight: this.getLastBlockHeight().toString(),
      creditsDistributionPool: this.getCreditsDistributionPool(),
    };
  }
}

module.exports = ChainInfo;
