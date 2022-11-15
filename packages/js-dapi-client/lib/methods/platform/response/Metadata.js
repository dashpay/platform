class Metadata {
  /**
   * @param {Object} properties
   * @param {number} properties.height - block height
   * @param {number} properties.coreChainLockedHeight - core chain locked height
   * @param {ITimestamp} properties.blockTime - block time
   * @param {Long} properties.protocolVersion - protocol version
   */
  constructor(properties) {
    this.height = properties.height;
    this.coreChainLockedHeight = properties.coreChainLockedHeight;
    this.blockTime = properties.blockTime;
    this.protocolVersion = properties.protocolVersion;
  }

  /**
   * Get height
   *
   * @returns {number} - block height
   */
  getHeight() {
    return this.height;
  }

  /**
   * Get core chain locked height
   *
   * @returns {number} - core chain locked height
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
