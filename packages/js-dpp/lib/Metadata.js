class Metadata {
  /**
   * @param {Object} rawMetadata
   * @param {number} rawMetadata.blockHeight
   * @param {number} rawMetadata.coreChainLockedHeight
   * @param {number} rawMetadata.timeMs - block time
   * @param {number} rawMetadata.protocolVersion - protocol version
   */
  constructor(rawMetadata) {
    this.blockHeight = rawMetadata.blockHeight;
    this.coreChainLockedHeight = rawMetadata.coreChainLockedHeight;
    this.timeMs = rawMetadata.timeMs;
    this.protocolVersion = rawMetadata.protocolVersion;
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

  /**
   * Get block time
   * @return {number}
   */
  getTimeMs() {
    return this.timeMs;
  }

  /**
   * Get protocol version
   * @return {number}
   */
  getProtocolVersion() {
    return this.protocolVersion;
  }
}

module.exports = Metadata;
