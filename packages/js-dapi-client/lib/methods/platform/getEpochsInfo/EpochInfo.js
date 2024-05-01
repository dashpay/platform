class EpochInfo {
  /**
   *
   * @param {number} number
   * @param {number} firstBlockHeight
   * @param {number} firstCoreBlockHeight
   * @param {number} startTime
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
   * @returns {number}
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
   * @returns {number}
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
