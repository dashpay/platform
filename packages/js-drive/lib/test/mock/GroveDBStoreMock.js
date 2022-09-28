class GroveDBStoreMock {
  /**
   * @param {Sandbox} sinon
   * @method put
   * @method putReference
   * @method createTree
   * @method get
   * @method delete
   * @method getAux
   * @method putAux
   * @method deleteAux
   * @method getRootHash
   * @method startTransaction
   * @method isTransactionStarted
   * @method rollbackTransaction
   * @method commitTransaction
   * @method abortTransaction
   * @method getDrive
   * @method getDB
   * @method setDB
   */
  constructor(sinon) {
    this.put = sinon.stub();
    this.putReference = sinon.stub();
    this.createTree = sinon.stub();
    this.get = sinon.stub();
    this.delete = sinon.stub();
    this.getAux = sinon.stub();
    this.putAux = sinon.stub();
    this.deleteAux = sinon.stub();
    this.getRootHash = sinon.stub();
    this.startTransaction = sinon.stub();
    this.isTransactionStarted = sinon.stub();
    this.rollbackTransaction = sinon.stub();
    this.commitTransaction = sinon.stub();
    this.abortTransaction = sinon.stub();
    this.getDrive = sinon.stub();
    this.getDB = sinon.stub();
    this.setDB = sinon.stub();
  }
}

module.exports = GroveDBStoreMock;
