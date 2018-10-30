const InvalidBlockHeightError = require('./InvalidBlockHeightError');

class RpcBlockIterator {
  /**
   * @param {RpcClient} rpcClient
   * @param {number} fromBlockHeight
   */
  constructor(rpcClient, fromBlockHeight = 1) {
    this.rpcClient = rpcClient;

    this.setBlockHeight(fromBlockHeight);
  }

  /**
   * Set block height since iterator starts
   *
   * @param {number} height
   */
  setBlockHeight(height) {
    this.fromBlockHeight = height;

    this.reset();
  }

  /**
   * Get current block height
   *
   * @return {number}
   */
  getBlockHeight() {
    if (this.currentBlock) {
      return this.currentBlock.height;
    }

    return this.fromBlockHeight;
  }

  /**
   * Reset iterator
   */
  reset() {
    this.nextBlockHash = null;

    this.currentBlock = null;
    this.firstIteration = true;
  }

  /**
   * Get next block
   *
   * @return {Promise<object>}
   */
  async next() {
    await this.initializeNextBlockHash();

    if (this.nextBlockHash) {
      const { result: block } = await this.rpcClient.getBlock(this.nextBlockHash);

      this.currentBlock = block;
      this.nextBlockHash = block.nextblockhash;

      return { done: false, value: block };
    }

    return { done: true };
  }

  /**
   * @return {{next: RpcBlockIterator.next}}
   */
  [Symbol.asyncIterator]() {
    return {
      next: this.next.bind(this),
    };
  }

  /**
   * @private
   * @return {Promise<void>}
   */
  async initializeNextBlockHash() {
    if (!this.firstIteration) {
      return;
    }

    let blockHash;
    try {
      ({ result: blockHash } = await this.rpcClient.getBlockHash(this.fromBlockHeight));
    } catch (e) {
      if (e.message === 'Block height out of range') {
        throw new InvalidBlockHeightError(this.fromBlockHeight);
      }

      throw e;
    }

    this.nextBlockHash = blockHash;
    this.firstIteration = false;
  }
}

module.exports = RpcBlockIterator;
