class Metadata {
  /**
   * @param {object} properties
   * @param {bigint|string} properties.height - block height
   * @param {number} properties.coreChainLockedHeight - core chain locked height
   * @param {bigint|string} properties.timeMs - block time
   * @param {number} properties.protocolVersion - protocol version
   */
  constructor(properties) {
    this.height = BigInt(properties.height);
    this.coreChainLockedHeight = properties.coreChainLockedHeight;
    this.timeMs = BigInt(properties.timeMs);
    this.protocolVersion = properties.protocolVersion;
  }

  /**
   * Get height
   * @returns {bigint} - block height
   */
  getHeight() {
    return this.height;
  }

  /**
   * Get core chain locked height
   * @returns {number} - core chain locked height
   */
  getCoreChainLockedHeight() {
    return this.coreChainLockedHeight;
  }

  /**
   * Get block time
   * @returns {bigint}
   */
  getTimeMs() {
    return this.timeMs;
  }

  /**
   * Get protocol version
   * @returns {number}
   */
  getProtocolVersion() {
    return this.protocolVersion;
  }
}

module.exports = Metadata;
