/**
 * @method addDataContract
 * @method hasDataContract
 * @method getDataContracts
 * @method reset
 * @method setHeight
 * @method getHeight
 * @method setTime
 * @method getTime
 * @method setVersion
 * @method getVersion
 * @method setLastCommitInfo
 * @method getLastCommitInfo
 * @method getValidTxCount
 * @method getInvalidTxCount
 * @method setConsensusLogger
 * @method getConsensusLogger
 * @method fromObject
 * @method toObject
 */
class BlockExecutionContextMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.addDataContract = sinon.stub();
    this.hasDataContract = sinon.stub();
    this.getDataContracts = sinon.stub();
    this.getCumulativeProcessingFee = sinon.stub();
    this.getCumulativeStorageFee = sinon.stub();
    this.setCoreChainLockedHeight = sinon.stub();
    this.getCoreChainLockedHeight = sinon.stub();
    this.setHeight = sinon.stub();
    this.getHeight = sinon.stub();
    this.setTime = sinon.stub();
    this.getTime = sinon.stub();
    this.reset = sinon.stub();
    this.setVersion = sinon.stub();
    this.getVersion = sinon.stub();
    this.setLastCommitInfo = sinon.stub();
    this.getLastCommitInfo = sinon.stub();
    this.setConsensusLogger = sinon.stub();
    this.getConsensusLogger = sinon.stub();
    this.setWithdrawalTransactionsMap = sinon.stub();
    this.getWithdrawalTransactionsMap = sinon.stub();
    this.populate = sinon.stub();
    this.isEmpty = sinon.stub();
    this.fromObject = sinon.stub();
    this.toObject = sinon.stub();
  }
}

module.exports = BlockExecutionContextMock;
