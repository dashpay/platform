class ChainStatus {
  /**
   * @param {boolean} catchingUp - is node syncing?
   * @param {string} latestBlockHash - latest block hash
   * @param {string} latestAppHash - latest app hash
   * @param {bigint} latestBlockHeight - latest block height
   * @param {string} earliestBlockHash - earliest block hash
   * @param {string} earliestAppHash - earliest app hash
   * @param {bigint} earliestBlockHeight - earliest block height
   * @param {bigint} maxPeerBlockHeight - max peer block height
   * @param {number=} coreChainLockedHeight - core chain locked height
   */
  constructor(
    catchingUp,
    latestBlockHash,
    latestAppHash,
    latestBlockHeight,
    earliestBlockHash,
    earliestAppHash,
    earliestBlockHeight,
    maxPeerBlockHeight,
    coreChainLockedHeight,
  ) {
    this.catchingUp = catchingUp;
    this.latestBlockHash = latestBlockHash;
    this.latestAppHash = latestAppHash;
    this.latestBlockHeight = latestBlockHeight;
    this.earliestBlockHash = earliestBlockHash;
    this.earliestAppHash = earliestAppHash;
    this.earliestBlockHeight = earliestBlockHeight;
    this.maxPeerBlockHeight = maxPeerBlockHeight;
    this.coreChainLockedHeight = coreChainLockedHeight || null;
  }

  /**
   * @returns {boolean} returns true if node is currently syncing
   */
  isCatchingUp() {
    return this.catchingUp;
  }

  /**
   * @returns {string} latest block hash
   */
  getLatestBlockHash() {
    return this.latestBlockHash;
  }

  /**
   * @returns {string} latest app hash
   */
  getLatestAppHash() {
    return this.latestAppHash;
  }

  /**
   * @returns {bigint} latest block height
   */
  getLatestBlockHeight() {
    return this.latestBlockHeight;
  }

  /**
   * @returns {string} earliest block hash
   */
  getEarliestBlockHash() {
    return this.earliestBlockHash;
  }

  /**
   * @returns {string} earliest app hash
   */
  getEarliestAppHash() {
    return this.earliestAppHash;
  }

  /**
   * @returns {bigint} earliest block height
   */
  getEarliestBlockHeight() {
    return this.earliestBlockHeight;
  }

  /**
   * @returns {bigint} max peer block height
   */
  getMaxPeerBlockHeight() {
    return this.maxPeerBlockHeight;
  }

  /**
   * @returns {number|null} core chain locked height
   */
  getCoreChainLockedHeight() {
    return this.coreChainLockedHeight;
  }
}

module.exports = ChainStatus;
