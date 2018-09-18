class ChainInfo {
  /**
   * @param {string} lastChainBlockHeight
   * @param {string} lastChainBlockHash
   * @param {boolean} isBlockchainSynced
   */
  constructor(lastChainBlockHeight, lastChainBlockHash, isBlockchainSynced) {
    this.lastChainBlockHeight = lastChainBlockHeight;
    this.lastChainBlockHash = lastChainBlockHash;
    this.isBlockchainSynced = isBlockchainSynced;
  }

  /**
   * Get last chain block height
   *
   * @returns {string}
   */
  getLastBlockHeight() {
    return this.lastChainBlockHeight;
  }

  /**
   * Get last chain block hash
   *
   * @returns {string}
   */
  getLastBlockHash() {
    return this.lastChainBlockHash;
  }

  /**
   * Get if blockchain is synced
   *
   * @returns {boolean}
   */
  getIsBlockchainSynced() {
    return this.isBlockchainSynced;
  }

  /**
   * Returns ChainInfo JSON representation
   *
   * @returns {{lastChainBlockHeight: string, lastChainBlockHash: string,
   *              isBlockchainSynced: boolean}}
   */
  toJSON() {
    return {
      lastChainBlockHeight: this.lastChainBlockHeight,
      lastChainBlockHash: this.lastChainBlockHash,
      isBlockchainSynced: this.isBlockchainSynced,
    };
  }
}

module.exports = ChainInfo;
