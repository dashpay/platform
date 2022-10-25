/**
 * @method store
 * @method fetch
 */
class BlockExecutionContextRepositoryMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.store = sinon.stub();
    this.fetch = sinon.stub();
  }
}

module.exports = BlockExecutionContextRepositoryMock;
