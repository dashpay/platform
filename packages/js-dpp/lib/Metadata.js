class Metadata {
  /**
   * @param {Object} rawMetadata
   * @param {number} rawMetadata.blockHeight
   * @param {number} rawMetadata.coreChainLockedHeight
   * @param {Buffer} rawMetadata.signature - signature
   * @param {ITimestamp} rawMetadata.time - block time
   * @param {Long} rawMetadata.protocolVersion - protocol version
   */
  constructor(rawMetadata) {
    this.blockHeight = rawMetadata.blockHeight;
    this.coreChainLockedHeight = rawMetadata.coreChainLockedHeight;
    this.signature = rawMetadata.signature;
    this.time = rawMetadata.time;
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
  getTime() {
    return this.time;
  }

  /**
   * Get signature
   * @return {Buffer}
   */
  getSignature() {
    return this.signature;
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
