/**
 * @method start
 * @method commit
 * @method abort
 * @method getTransaction
 */
class BlockExecutionStoreTransactionsMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.start = sinon.stub();
    this.commit = sinon.stub();
    this.abort = sinon.stub();
    this.getTransaction = sinon.stub();
    this.clone = sinon.stub();
  }
}

module.exports = BlockExecutionStoreTransactionsMock;
