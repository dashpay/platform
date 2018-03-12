const Emittery = require('emittery');

const promisifyMethods = require('../util/promisifyMethods');

// TODO: It might be part of SDK in the future

module.exports = class BlockIterator extends Emittery {
  /**
   * @param {RpcClient} rpcClient
   * @param {number} fromBlockHeight
   */
  constructor(rpcClient, fromBlockHeight = 1) {
    super();

    this.rpcClient = rpcClient;
    this.promisifiedRpcClient = promisifyMethods(rpcClient, ['getBlockHash', 'getBlock']);

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
   * @return {Promise<Object>}
   */
  async next() {
    await this.initializeNextBlockHash();

    if (this.nextBlockHash) {
      const { result: block } = await this.promisifiedRpcClient.getBlock(this.nextBlockHash);
      this.currentBlock = block;
      this.nextBlockHash = block.nextblockhash;

      await this.emitSerial('block', block);

      return { done: false, value: block };
    }

    return { done: true };
  }

  /**
   * @private
   * @return {Promise<void>}
   */
  async initializeNextBlockHash() {
    if (!this.firstIteration) {
      return;
    }

    const response = await this.promisifiedRpcClient.getBlockHash(this.fromBlockHeight);
    this.nextBlockHash = response.result;
    this.firstIteration = false;
  }
};
