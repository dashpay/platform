/**
 * @method start
 * @method commit
 * @method abort
 * @method getTransaction
 */
class BlockExecutionDBTransactionsMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.start = sinon.stub();
    this.commit = sinon.stub();
    this.abort = sinon.stub();
    this.getTransaction = sinon.stub();
  }
}

module.exports = BlockExecutionDBTransactionsMock;
