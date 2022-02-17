/**
 * @method setContexts
 * @method getContexts
 * @method getFirst
 * @method getLast
 * @method getLatest
 * @method removeLatest
 * @method add
 * @method getSize
 */
class BlockExecutionContextStackMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.setContexts = sinon.stub();
    this.getContexts = sinon.stub();
    this.getFirst = sinon.stub();
    this.getLast = sinon.stub();
    this.getLatest = sinon.stub();
    this.removeLatest = sinon.stub();
    this.add = sinon.stub();
    this.getSize = sinon.stub();
  }
}

module.exports = BlockExecutionContextStackMock;
