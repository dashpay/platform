class ConsensusParamsBlock {
  /**
   *
   * @param {string} maxBytes
   * @param {string} maxGas
   * @param {string} timeIotaMs
   */
  constructor(maxBytes, maxGas, timeIotaMs) {
    this.maxBytes = maxBytes;
    this.maxGas = maxGas;
    this.timeIotaMs = timeIotaMs;
  }

  /**
   * @returns {string}
   */
  getMaxBytes() {
    return this.maxBytes;
  }

  /**
   * @returns {string}
   */
  getMaxGas() {
    return this.maxGas;
  }

  /**
   * @returns {string}
   */
  getTimeIotaMs() {
    return this.timeIotaMs;
  }
}

module.exports = ConsensusParamsBlock;
