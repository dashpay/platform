class StoreMock {
  /**
   * @param {Sandbox} sinon
   */
  constructor(sinon) {
    this.put = sinon.stub();
    this.get = sinon.stub();
    this.createTransaction = sinon.stub();
  }
}

module.exports = StoreMock;
