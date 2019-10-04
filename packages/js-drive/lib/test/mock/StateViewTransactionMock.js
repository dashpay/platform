/**
 * @method start
 * @method commit
 * @method abort
 * @property {boolean} isTransactionStarted
 */
class StateViewTransactionMock {
  /**
   * @param {Sandbox} sinon
   */
  constructor(sinon) {
    this.start = sinon.stub();
    this.commit = sinon.stub();
    this.abort = sinon.stub();

    this.isTransactionStarted = false;
  }
}

module.exports = StateViewTransactionMock;
