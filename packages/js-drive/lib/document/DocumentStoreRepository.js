class DocumentStoreRepository {
  /**
   *
   * @param {MerkDbStore} documentsStore
   * @param {AwilixContainer} container
   */
  constructor(documentsStore, container) {
    this.storage = documentsStore;
    this.container = container;
  }

  /**
   * Store document
   *
   * @param {Document} document
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<IdentityStoreRepository>}
   */
  async store(document, transaction = undefined) {
    this.storage.put(
      document.getId(),
      document.toBuffer(),
      transaction,
    );
  }

  /**
   * Fetch document by id
   *
   * @param {Identifier} id
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<null|Document>}
   */
  async fetch(id, transaction = undefined) {
    const encodedDocument = this.storage.get(id, transaction);

    if (!encodedDocument) {
      return null;
    }

    const dpp = this.container.resolve(transaction ? 'transactionalDpp' : 'dpp');

    return dpp.document.createFromBuffer(
      encodedDocument,
      { skipValidation: true },
    );
  }

  /**
   *
   * @param {Identifier} id
   * @param {MerkDbTransaction} transaction
   * @return {Promise<void>}
   */
  async delete(id, transaction = undefined) {
    this.storage.delete(
      id,
      transaction,
    );
  }
}

module.exports = DocumentStoreRepository;
