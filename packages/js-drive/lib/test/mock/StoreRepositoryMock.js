class StoreRepositoryMock {
  /**
   * @param {Sandbox} sinon
   * @method store
   * @method fetch
   * @method createTree
   */
  constructor(sinon) {
    this.store = sinon.stub();
    this.fetch = sinon.stub();
    this.createTree = sinon.stub();
  }
}

module.exports = StoreRepositoryMock;
