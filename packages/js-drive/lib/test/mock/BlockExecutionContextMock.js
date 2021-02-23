/**
 * @method addDataContract
 * @method getDataContracts
 * @method getCumulativeFees
 * @method incrementCumulativeFees
 * @method reset
 */
class BlockExecutionContextMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.addDataContract = sinon.stub();
    this.hasDataContract = sinon.stub();
    this.getDataContracts = sinon.stub();
    this.getCumulativeFees = sinon.stub();
    this.incrementCumulativeFees = sinon.stub();
    this.reset = sinon.stub();
    this.setHeader = sinon.stub();
    this.getHeader = sinon.stub();
    this.getValidTxCount = sinon.stub();
    this.getInvalidTxCount = sinon.stub();
    this.incrementValidTxCount = sinon.stub();
    this.incrementInvalidTxCount = sinon.stub();
    this.setConsensusLogger = sinon.stub();
    this.getConsensusLogger = sinon.stub();
  }
}

module.exports = BlockExecutionContextMock;
