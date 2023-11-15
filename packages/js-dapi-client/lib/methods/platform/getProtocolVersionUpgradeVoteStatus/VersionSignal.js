class VersionSignal {
  /**
   * @param {string} proTxHash
   * @param {number} version
   */
  constructor(proTxHash, version) {
    this.proTxHash = proTxHash;
    this.version = version;
  }

  /**
   * @returns {string}
   */
  getProTxHash() {
    return this.proTxHash;
  }

  /**
   * @returns {number}
   */
  getVersion() {
    return this.version;
  }
}

module.exports = VersionSignal;
