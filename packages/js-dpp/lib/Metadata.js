class Metadata {
  /**
   * @param {Object} rawMetadata
   * @param {number} rawMetadata.blockHeight
   * @param {number} rawMetadata.coreChainLockedHeight
   * @param {ITimestamp} rawMetadata.blockTime - block time
   * @param {Long} rawMetadata.protocolVersion - protocol version
   */
  constructor(rawMetadata) {
    this.blockHeight = rawMetadata.blockHeight;
    this.coreChainLockedHeight = rawMetadata.coreChainLockedHeight;
    this.blockTime = rawMetadata.blockTime;
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
   * @return {ITimestamp}
   */
  getBlockTime() {
    return this.blockTime;
  }

  /**
   * Get protocol version
   * @return {Long}
   */
  getProtocolVersion() {
    return this.protocolVersion;
  }
}

module.exports = Metadata;
