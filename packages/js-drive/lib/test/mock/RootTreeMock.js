class RootTreeMock {
  /**
   * @param {Sandbox} sinon
   */
  constructor(sinon) {
    this.getRootHash = sinon.stub();
    this.rebuild = sinon.stub();
  }
}

module.exports = RootTreeMock;
