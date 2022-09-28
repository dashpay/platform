/**
 * @method store
 * @method fetch
 */
class BlockExecutionContextStackRepositoryMock {
  /**
   * @param {SinonSandbox} sinon
   */
  constructor(sinon) {
    this.store = sinon.stub();
    this.fetch = sinon.stub();
  }
}

module.exports = BlockExecutionContextStackRepositoryMock;
