class DriveMock {
  /**
   * @param {Sandbox} sinon
   * @method getGroveDB
   * @method close
   * @method createRootTree
   * @method fetchContract
   * @method createContract
   * @method updateContract
   * @method createDocument
   * @method updateDocument
   * @method deleteDocument
   * @method queryDocuments
   */
  constructor(sinon) {
    this.getGroveDB = sinon.stub();
    this.close = sinon.stub();
    this.createRootTree = sinon.stub();
    this.fetchContract = sinon.stub();
    this.createContract = sinon.stub();
    this.createContract = sinon.stub();
    this.createDocument = sinon.stub();
    this.updateDocument = sinon.stub();
    this.deleteDocument = sinon.stub();
    this.queryDocuments = sinon.stub();
  }
}

module.exports = DriveMock;
