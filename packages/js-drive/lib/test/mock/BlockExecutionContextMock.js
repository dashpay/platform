/**
 * @method addDataContract
 * @method hasDataContract
 * @method getDataContracts
 * @method reset
 * @method setHeight
 * @method getHeight
 * @method setVersion
 * @method getVersion
 * @method setLastCommitInfo
 * @method getLastCommitInfo
 * @method getValidTxCount
 * @method getInvalidTxCount
 * @method setContextLogger
 * @method getContextLogger
 * @method getRound
 * @method fromObject
 * @method toObject
 * @method getEpochInfo
 * @method setEpochInfo
 * @method setTimeMs
 * @method getTimeMs
 * @method getRound
 * @method setRound
 */
class BlockExecutionContextMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.addDataContract = sinon.stub();
    this.hasDataContract = sinon.stub();
    this.getDataContracts = sinon.stub();
    this.setCoreChainLockedHeight = sinon.stub();
    this.getCoreChainLockedHeight = sinon.stub();
    this.setHeight = sinon.stub();
    this.getHeight = sinon.stub();
    this.reset = sinon.stub();
    this.setVersion = sinon.stub();
    this.getVersion = sinon.stub();
    this.setLastCommitInfo = sinon.stub();
    this.getLastCommitInfo = sinon.stub();
    this.setContextLogger = sinon.stub();
    this.getContextLogger = sinon.stub();
    this.setWithdrawalTransactionsMap = sinon.stub();
    this.getWithdrawalTransactionsMap = sinon.stub();
    this.getRound = sinon.stub();
    this.populate = sinon.stub();
    this.isEmpty = sinon.stub();
    this.fromObject = sinon.stub();
    this.toObject = sinon.stub();
    this.setEpochInfo = sinon.stub();
    this.getEpochInfo = sinon.stub();
    this.setTimeMs = sinon.stub();
    this.getTimeMs = sinon.stub();
    this.setRound = sinon.stub();
    this.getRound = sinon.stub();
    this.getPrepareProposalResult = sinon.stub();
    this.setPrepareProposalResult = sinon.stub();
  }
}

module.exports = BlockExecutionContextMock;
