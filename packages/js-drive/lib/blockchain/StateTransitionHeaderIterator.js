/* eslint-disable no-await-in-loop */

// TODO: It might be part of SDK in the future

const StateTransitionHeader = require('./StateTransitionHeader');
const promisifyMethods = require('../util/promisifyMethods');

module.exports = class StateTransitionHeaderIterator {
  /**
   * @param {BlockIterator} blockIterator
   */
  constructor(blockIterator) {
    this.blockIterator = blockIterator;

    this.promisifiedRpcClient = promisifyMethods(this.blockIterator.rpcClient, ['getTransitionHeader']);

    this.currentBlock = null;
    this.currentTransitionIndex = 0;
  }

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
