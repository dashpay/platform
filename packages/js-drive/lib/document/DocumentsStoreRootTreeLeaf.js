const AbstractRootTreeLeaf = require('../rootTree/AbstractRootTreeLeaf');

class DocumentsStoreRootTreeLeaf extends AbstractRootTreeLeaf {
  /**
   * @param {MerkDbStore} documentsStore
   */
  constructor(documentsStore) {
    super(DocumentsStoreRootTreeLeaf.INDEX);

    this.documentsStore = documentsStore;
  }

  /**
   * Get leaf hash
   *
   * @return {Buffer}
   */
  getHash() {
    return this.documentsStore.getRootHash();
  }
}

DocumentsStoreRootTreeLeaf.INDEX = 4;

module.exports = DocumentsStoreRootTreeLeaf;
