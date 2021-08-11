class ConsensusParamsEvidence {
  /**
   *
   * @param {string} maxAgeNumBlocks
   * @param {string} maxAgeDuration
   * @param {string} maxBytes
   */
  constructor(maxAgeNumBlocks, maxAgeDuration, maxBytes) {
    this.maxAgeNumBlocks = maxAgeNumBlocks;
    this.maxAgeDuration = maxAgeDuration;
    this.maxBytes = maxBytes;
  }

  /**
   * @returns {string}
   */
  getMaxAgeNumBlocks() {
    return this.maxAgeNumBlocks;
  }

  /**
   * @returns {string}
   */
  getMaxAgeDuration() {
    return this.maxAgeDuration;
  }

  /**
   * @returns {string}
   */
  getMaxBytes() {
    return this.maxBytes;
  }
}

module.exports = ConsensusParamsEvidence;
