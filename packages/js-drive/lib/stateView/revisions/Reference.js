class Reference {
  /**
   * @param {Object} hashes
   * @param {string} hashes.blockHash
   * @param {number} hashes.blockHeight
   * @param {string} hashes.stHash
   * @param {string} hashes.hash
   */
  constructor(hashes) {
    this.blockHash = hashes.blockHash;
    this.blockHeight = hashes.blockHeight;
    this.stHash = hashes.stHash;
    this.hash = hashes.hash;
  }

  /**
   * @return {string}
   */
  getBlockHash() {
    return this.blockHash;
  }

  /**
   * @return {number}
   */
  getBlockHeight() {
    return this.blockHeight;
  }

  /**
   * @return {string}
   */
  getSTHash() {
    return this.stHash;
  }

  /**
   * @return {string}
   */
  getHash() {
    return this.hash;
  }

  /**
   * Get Reference as plain object
   *
   * @return {{ blockHash: string,
   *            blockHeight: number,
   *            stHash: string,
   *            hash: string }}
   */
  toJSON() {
    return {
      blockHash: this.getBlockHash(),
      blockHeight: this.getBlockHeight(),
      stHash: this.getSTHash(),
      hash: this.getHash(),
    };
  }
}

module.exports = Reference;
