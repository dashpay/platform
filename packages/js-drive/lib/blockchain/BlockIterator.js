const promisifyMethods = require('../util/promisifyMethods');
const WrongBlocksSequenceError = require('./WrongBlocksSequenceError');

// TODO: It might be part of SDK in the future

module.exports = class BlockIterator {
  /**
   * @param {RpcClient} rpcClient
   * @param {number} fromBlockHeight
   */
  constructor(rpcClient, fromBlockHeight = 1) {
    this.rpcClient = rpcClient;
    this.promisifiedRpcClient = promisifyMethods(rpcClient, ['getBlockHash', 'getBlock']);

    this.setBlockHeight(fromBlockHeight);
  }

  /**
   * Set block height
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
    if (this.previousBlock) {
      return this.previousBlock.height;
    }

    return this.fromBlockHeight;
  }

  /**
   * Reset iterator
   */
  reset() {
    this.nextBlockHash = null;
    this.nextBlockHeight = this.fromBlockHeight;

    this.previousBlock = null;
  }

  /**
   * Get next block
   *
   * @return {Promise<Object>}
   */
  async next() {
    await this.initializeNextBlockHash();

    if (this.nextBlockHash) {
      const { result: block } = await this.promisifiedRpcClient.getBlock(this.nextBlockHash);

      if (!block) {
        throw new WrongBlocksSequenceError();
      }

      if (!this.previousBlock) {
        this.previousBlock = block;
      } else if (block.previousblockhash && block.previousblockhash !== this.previousBlock.hash) {
        throw new WrongBlocksSequenceError();
      }

      this.nextBlockHeight = block.height + 1;
      this.nextBlockHash = block.nextblockhash;

      this.previousBlock = block;

      return { done: false, value: block };
    }

    return { done: true };
  }

  /**
   * @private
   * @return {Promise<void>}
   */
  async initializeNextBlockHash() {
    if (this.previousBlock) {
      return;
    }

    const response = await this.promisifiedRpcClient.getBlockHash(this.nextBlockHeight);
    this.nextBlockHash = response.result;
  }
};
