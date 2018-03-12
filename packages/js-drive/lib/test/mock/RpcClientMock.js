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

      sinonSandbox.spy(proto, method);
    }
  }

  /**
   * @param {number} height
   * @param {Function} callback
   */
  getBlockHash(height, callback) {
    const block = this.blocks.find(b => b.height === height);
    callback(null, { result: block ? block.hash : null });
  }

  /**
   * @param {string} hash
   * @param {Function} callback
   */
  getBlock(hash, callback) {
    const block = this.blocks.find(b => b.hash === hash);
    callback(null, { result: block });
  }

  /**
   * @param {string} tsid
   * @param {Function} callback
   */
  getTransitionHeader(tsid, callback) {
    const header = this.transitionHeaders.find(h => h.getHash() === tsid);
    callback(null, { result: header });
  }
};
