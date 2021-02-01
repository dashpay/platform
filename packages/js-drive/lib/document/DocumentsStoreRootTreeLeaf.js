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

  /**
   * Get proof for leaf keys
   *
   * @param {Array<Buffer>} leafKeys
   * @return {Buffer}
   */
  getProof(leafKeys) {
    return this.documentsStore.getProof(leafKeys);
  }
}

DocumentsStoreRootTreeLeaf.INDEX = 4;

module.exports = DocumentsStoreRootTreeLeaf;
