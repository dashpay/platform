class LatestCoreChainLock {
  /**
   *
   * @param {ChainLock} [chainLock]
   * @param {LatestCoreChainLock}
   */
  constructor(chainLock) {
    this.chainLock = chainLock;
  }

  /**
   * Update latest chainlock
   *
   * @param {ChainLock} chainLock
   * @return {LatestCoreChainLock}
   */
  update(chainLock) {
    this.chainLock = chainLock;
    return this;
  }
}

module.exports = LatestCoreChainLock;
