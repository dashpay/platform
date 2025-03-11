class TimeStatus {
  /**
   * @param {bigint} local - Local system time
   * @param {bigint=} block - Block time
   * @param {bigint=} genesis - Genesis time
   * @param {number=} epoch - Epoch number
   */
  constructor(
    local,
    block,
    genesis,
    epoch,
  ) {
    this.local = local;
    this.block = typeof block === 'bigint' ? block : null;
    this.genesis = typeof genesis === 'bigint' ? genesis : null;
    this.epoch = epoch || null;
  }

  /**
   * @returns {bigint} Local system time
   */
  getLocalTime() {
    return this.local;
  }

  /**
   * @returns {bigint|null} Block time
   */
  getBlockTime() {
    return this.block;
  }

  /**
   * @returns {bigint|null} Genesis time
   */
  getGenesisTime() {
    return this.genesis;
  }

  /**
   * @returns {number|null} Epoch number
   */
  getEpochNumber() {
    return this.epoch;
  }
}

module.exports = TimeStatus;
