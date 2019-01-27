class Reference {
  /**
   * @param {Object} hashes
   * @param {string} hashes.blockHash
   * @param {number} hashes.blockHeight
   * @param {string} hashes.stHeaderHash
   * @param {string} hashes.stPacketHash
   * @param {string} hashes.hash
   */
  constructor(hashes) {
    this.blockHash = hashes.blockHash;
    this.blockHeight = hashes.blockHeight;
    this.stHeaderHash = hashes.stHeaderHash;
    this.stPacketHash = hashes.stPacketHash;
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
  getSTHeaderHash() {
    return this.stHeaderHash;
  }

  /**
   * @return {string}
   */
  getSTPacketHash() {
    return this.stPacketHash;
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
   *            stHeaderHash: string,
   *            stPacketHash: string,
   *            hash: string }}
   */
  toJSON() {
    return {
      blockHash: this.getBlockHash(),
      blockHeight: this.getBlockHeight(),
      stHeaderHash: this.getSTHeaderHash(),
      stPacketHash: this.getSTPacketHash(),
      hash: this.getHash(),
    };
  }
}

module.exports = Reference;
