// TODO: It might be part of SDK in the future

const StateTransitionHeader = require('./StateTransitionHeader');
const promisifyMethods = require('../util/promisifyMethods');

module.exports = class StateTransitionHeaderIterator {
  /**
   * @param {Iterator} blockIterator
   * @param {RpcClient} rpcClient
   */
  constructor(blockIterator, rpcClient) {
    this.blockIterator = blockIterator;

    this.rpcClient = rpcClient;
    this.promisifiedRpcClient = promisifyMethods(rpcClient, ['getTransitionHeader']);

    this.reset(true);
  }

  /**
   * Reset iterator
   */
  reset(onlyHeaders = false) {
    if (!onlyHeaders) {
      this.blockIterator.reset();
    }

    this.currentBlock = null;
    this.currentTransitionIndex = 0;
  }

  /**
   * Get next ST header
   *
   * @return {Promise<Object>}
   */
  async next() {
    for (;;) {
      if (!this.currentBlock) {
        const { done, value: block } = await this.blockIterator.next();

        if (done) {
          return { done: true };
        }

        this.currentBlock = block;
        this.currentTransitionIndex = 0;
      }

      const transitionId = this.currentBlock.ts[this.currentTransitionIndex];

      if (transitionId) {
        const { result: transitionHeader } =
          await this.promisifiedRpcClient.getTransitionHeader(transitionId);

        this.currentTransitionIndex++;

        return { done: false, value: new StateTransitionHeader(transitionHeader) };
      }

      this.currentBlock = null;
    }
  }
};
