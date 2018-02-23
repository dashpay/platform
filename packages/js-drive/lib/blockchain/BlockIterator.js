const promisifyMethods = require('../util/promisifyMethods');

// TODO: It might be part of SDK in the future

module.exports = class BlockIterator {
  /**
   * @param {RpcClient} rpcClient
   * @param {number} fromBlockHeight
   */
  constructor(rpcClient, fromBlockHeight) {
    this.rpcClient = rpcClient;
    this.promisifiedRpcClient = promisifyMethods(rpcClient, ['getBlockHash', 'getBlock']);

    this.currentBlockHeight = fromBlockHeight;
    this.currentBlockHash = null;

    this.firstIteration = true;
  }

  async next() {
    await this.initializeBlockHash();

    if (this.currentBlockHash) {
      const { result: block } = await this.promisifiedRpcClient.getBlock(this.currentBlockHash);

      if (block) {
        this.currentBlockHeight++;
        this.currentBlockHash = block.nextblockhash;

        return { done: false, value: block };
      }
    }

    return { done: true };
  }

  async initializeBlockHash() {
    if (!this.firstIteration) {
      return;
    }

    const response = await this.promisifiedRpcClient.getBlockHash(this.currentBlockHeight);
    this.currentBlockHash = response.result;
    this.firstIteration = false;
  }
};
