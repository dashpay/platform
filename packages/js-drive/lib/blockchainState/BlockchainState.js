const Long = require('long');

class BlockchainState {
  /**
   *
   * @param {Long} [lastBlockHeight]
   * @param {Buffer} [lastBlockAppHash]
   * @param {number} [creditsDistributionPool]
   */
  constructor(
    lastBlockHeight = Long.fromInt(0),
    lastBlockAppHash = Buffer.alloc(0),
    creditsDistributionPool = 0,
  ) {
    this.lastBlockHeight = lastBlockHeight;
    this.lastBlockAppHash = lastBlockAppHash;
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
   * @return {BlockchainState}
   */
  setLastBlockHeight(blockHeight) {
    this.lastBlockHeight = blockHeight;

    return this;
  }

  /**
   * Get last block app hash
   *
   * @return {Buffer}
   */
  getLastBlockAppHash() {
    return this.lastBlockAppHash;
  }

  /**
   * Set last block app hash
   *
   * @param {Buffer} appHash
   * @return {BlockchainState}
   */
  setLastBlockAppHash(appHash) {
    this.lastBlockAppHash = appHash;

    return this;
  }

  /**
   * Set credits distribution pool
   *
   * @param {number} credits
   * @return {BlockchainState}
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
   *    lastBlockAppHash: Buffer,
   *    creditsDistributionPool: number,
   * }}
   */
  toJSON() {
    return {
      lastBlockHeight: this.getLastBlockHeight().toString(),
      lastBlockAppHash: this.getLastBlockAppHash(),
      creditsDistributionPool: this.getCreditsDistributionPool(),
    };
  }
}

module.exports = BlockchainState;
