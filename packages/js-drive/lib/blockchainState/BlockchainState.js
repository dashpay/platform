const Long = require('long');

class BlockchainState {
  /**
   *
   * @param {Long} lastBlockHeight
   * @param {Buffer} lastBlockAppHash
   */
  constructor(lastBlockHeight = Long.fromInt(0), lastBlockAppHash = Buffer.alloc(0)) {
    this.lastBlockHeight = lastBlockHeight;
    this.lastBlockAppHash = lastBlockAppHash;
  }

  /**
   * Get last block height
   *
   * @return {Long}
   */
  getLastBlockHeight() {
    return this.lastBlockHeight;
  }

  /**
   * Set last block height
   *
   * @param {Long} blockHeight
   * @return {BlockchainState}
   */
  setLastBlockHeight(blockHeight) {
    this.lastBlockHeight = blockHeight;

    return this;
  }

  /**
   * Get last block app hash
   *
   * @return {Buffer}
   */
  getLastBlockAppHash() {
    return this.lastBlockAppHash;
  }

  /**
   * Set last block app hash
   *
   * @param {Buffer} appHash
   * @return {BlockchainState}
   */
  setLastBlockAppHash(appHash) {
    this.lastBlockAppHash = appHash;

    return this;
  }

  /**
   * Get plain JS object
   *
   * @return {{
   * lastBlockHeight: string,
   * lastBlockAppHash: Buffer
   * }}
   */
  toJSON() {
    return {
      lastBlockHeight: this.lastBlockHeight.toString(),
      lastBlockAppHash: this.lastBlockAppHash,
    };
  }
}

module.exports = BlockchainState;
