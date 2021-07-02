class Metadata {
  /**
   * @param {Object} properties
   * @param {number} properties.height - block height
   * @param {number} properties.coreChainLockedHeight - core chain locked height
   */
  constructor(properties) {
    this.height = properties.height;
    this.coreChainLockedHeight = properties.coreChainLockedHeight;
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
}

module.exports = Metadata;
