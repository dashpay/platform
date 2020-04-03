/**
 * @method addDataContract
 * @method getDataContracts
 * @method reset
 */
class BlockExecutionStateMock {
  /**
   * @param {Sandbox} sinon
   */
  constructor(sinon) {
    this.addDataContract = sinon.stub();
    this.getDataContracts = sinon.stub();
    this.reset = sinon.stub();
  }
}

module.exports = BlockExecutionStateMock;
