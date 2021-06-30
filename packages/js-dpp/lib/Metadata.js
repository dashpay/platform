class Metadata {
  /**
   * @param {Object} rawMetadata
   * @param {number} rawMetadata.blockHeight
   * @param {number} rawMetadata.coreChainLockedHeight
   */
  constructor(rawMetadata) {
    this.blockHeight = rawMetadata.blockHeight;
    this.coreChainLockedHeight = rawMetadata.coreChainLockedHeight;
  }

  /**
   * Get block height
   * @returns {number}
   */
  getBlockHeight() {
    return this.blockHeight;
  }

  /**
   * Get core chain-locked height
   * @returns {number}
   */
  getCoreChainLockedHeight() {
    return this.coreChainLockedHeight;
  }
}

module.exports = Metadata;
