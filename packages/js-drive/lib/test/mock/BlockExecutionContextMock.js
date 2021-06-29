/**
 * @method addDataContract
 * @method hasDataContract
 * @method getDataContracts
 * @method getCumulativeFees
 * @method incrementCumulativeFees
 * @method reset
 * @method setHeader
 * @method getHeader
 * @method setLastCommitInfo
 * @method getLastCommitInfo
 * @method getValidTxCount
 * @method getInvalidTxCount
 * @method incrementValidTxCount
 * @method incrementInvalidTxCount
 * @method setConsensusLogger
 * @method getConsensusLogger
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
    this.setLastCommitInfo = sinon.stub();
    this.getLastCommitInfo = sinon.stub();
    this.getValidTxCount = sinon.stub();
    this.getInvalidTxCount = sinon.stub();
    this.incrementValidTxCount = sinon.stub();
    this.incrementInvalidTxCount = sinon.stub();
    this.setConsensusLogger = sinon.stub();
    this.getConsensusLogger = sinon.stub();
    this.populate = sinon.stub();
    this.isEmpty = sinon.stub();
  }
}

module.exports = BlockExecutionContextMock;
