class EpochInfo {
  /**
   *
   * @param {number} number
   * @param {bigint} firstBlockHeight
   * @param {number} firstCoreBlockHeight
   * @param {bigint} startTime
   * @param {number} feeMultiplier
   */
  constructor(number, firstBlockHeight, firstCoreBlockHeight, startTime, feeMultiplier) {
    this.number = number;
    this.firstBlockHeight = firstBlockHeight;
    this.firstCoreBlockHeight = firstCoreBlockHeight;
    this.startTime = startTime;
    this.feeMultiplier = feeMultiplier;
  }

  /**
   * @returns {number}
   */
  getNumber() {
    return this.number;
  }

  /**
   * @returns {bigint}
   */
  getFirstBlockHeight() {
    return this.firstBlockHeight;
  }

  /**
   * @returns {number}
   */
  getFirstCoreBlockHeight() {
    return this.firstCoreBlockHeight;
  }

  /**
   * @returns {bigint}
   */
  getStartTime() {
    return this.startTime;
  }

  /**
   * @returns {number}
   */
  getFeeMultiplier() {
    return this.feeMultiplier;
  }
}

module.exports = EpochInfo;
