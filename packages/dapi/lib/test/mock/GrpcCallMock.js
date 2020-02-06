const { EventEmitter } = require('events');

/**
 * @method write
 * @method end
 * @property {Object} request
 */
class GrpcCallMock extends EventEmitter {
  /**
   * @param {SinonSandbox} sinon
   * @param {Object} request
   */
  constructor(sinon, request = {}) {
    super();

    this.write = sinon.stub();
    this.end = sinon.stub();
    this.request = request;
  }
}

module.exports = GrpcCallMock;
