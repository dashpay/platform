const getBlockFixtures = require('../fixtures/getBlockFixtures');
const getTransitionHeaderFixtures = require('../fixtures/getTransitionHeaderFixtures');

module.exports = class RpcClientMock {
  /**
   * @param {Object} sinonSandbox
   */
  constructor(sinonSandbox) {
    this.blocks = getBlockFixtures();
    this.transitionHeaders = getTransitionHeaderFixtures();

    const { __proto__: proto } = this;
    for (const method of Object.getOwnPropertyNames(proto)) {
      if (method === 'constructor') {
        // eslint-disable-next-line no-continue
        continue;
      }

      if (proto[method].restore) {
        proto[method].restore();
      }

      sinonSandbox.stub(proto, method).callThrough();
    }
  }

  /**
   *
   */
  getBlockCount() {
    const lastBlock = this.blocks[this.blocks.length - 1];

    return Promise.resolve({ result: lastBlock ? lastBlock.height : 0 });
  }

  /**
   * @param {number} height
   */
  getBlockHash(height) {
    const block = this.blocks.find(b => b.height === height);

    return Promise.resolve({ result: block ? block.hash : null });
  }

  /**
   * @param {string} hash
   */
  getBlock(hash) {
    const block = this.blocks.find(b => b.hash === hash);

    return Promise.resolve({ result: block });
  }

  /**
   * @param {string} tsid
   */
  getTransition(tsid) {
    const header = this.transitionHeaders.find(h => h.getHash() === tsid);

    return Promise.resolve({ result: header });
  }

  // eslint-disable-next-line class-methods-use-this
  mnsync(mode) {
    if (mode !== 'status') {
      throw new Error('Not implemented yet!');
    }
    return Promise.resolve({ result: { IsBlockchainSynced: true } });
  }
};
