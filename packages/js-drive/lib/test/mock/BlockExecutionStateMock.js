/**
 * @method addDataContract
 * @method getDataContracts
 * @method getAccumulativeFees
 * @method incrementAccumulativeFees
 * @method reset
 */
class BlockExecutionStateMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.addDataContract = sinon.stub();
    this.getDataContracts = sinon.stub();
    this.getAccumulativeFees = sinon.stub();
    this.incrementAccumulativeFees = sinon.stub();
    this.reset = sinon.stub();
  }
}

module.exports = BlockExecutionStateMock;
