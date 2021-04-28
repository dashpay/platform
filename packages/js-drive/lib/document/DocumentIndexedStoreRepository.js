class DocumentIndexedStoreRepository {
  /**
   *
   * @param {DocumentStoreRepository} documentStoreRepository
   * @param {createDocumentMongoDbRepository} createDocumentMongoDbRepository
   */
  constructor(documentStoreRepository, createDocumentMongoDbRepository) {
    this.documentStoreRepository = documentStoreRepository;
    this.createDocumentMongoDbRepository = createDocumentMongoDbRepository;
  }

  /**
   * Store document
   *
   * @param {Document} document
   * @param {DocumentsIndexedTransaction} [transaction]
   * @return {Promise<void>}
   */
  async store(document, transaction = undefined) {
    let storeTransaction;
    let mongoDbTransaction;

    if (transaction) {
      storeTransaction = transaction.getStoreTransaction();
      mongoDbTransaction = transaction.getMongoDbTransaction();
    }

    const indicesRepository = await this.createDocumentMongoDbRepository(
      document.getDataContractId(),
      document.getType(),
      {
        isTransactional: transaction !== undefined,
      },
    );

    await indicesRepository.store(document, mongoDbTransaction);

    await this.documentStoreRepository.store(document, storeTransaction);
  }

  /**
   * Fetch document by ID
   *
   * @param {Identifier} id
   * @param {DocumentsIndexedTransaction} [transaction]
   * @return {Promise<Document|null>}
   */
  async fetch(id, transaction = undefined) {
    const storeTransaction = transaction ? transaction.getStoreTransaction() : undefined;

    return this.documentStoreRepository.fetch(id, storeTransaction);
  }

  /**
   * Find documents by query
   *
   * @param {Identifier|Buffer} dataContractId
   * @param {string} documentType
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   * @param {DocumentsIndexedTransaction} [transaction]
   * @return {Promise<Document[]>}
   */
  async find(dataContractId, documentType, query = {}, transaction = undefined) {
    let storeTransaction;
    let mongoDbTransaction;

    if (transaction) {
      storeTransaction = transaction.getStoreTransaction();
      mongoDbTransaction = transaction.getMongoDbTransaction();
    }

    const indicesRepository = await this.createDocumentMongoDbRepository(
      dataContractId,
      documentType,
      {
        isTransactional: transaction !== undefined,
      },
    );

    const documentIds = await indicesRepository.find(query, mongoDbTransaction);

    const documents = await Promise.all(documentIds.map((id) => (
      this.documentStoreRepository.fetch(id, storeTransaction)
    )));

    return documents.filter((document) => document !== null);
  }

  /**
   * Find documents by query
   *
   * @param {Identifier} dataContractId
   * @param {string} documentType
   * @param {Identifier} documentId
   * @param {DocumentsIndexedTransaction} [transaction]
   * @return {Promise<Document[]>}
   */
  async delete(dataContractId, documentType, documentId, transaction) {
    let storeTransaction;
    let mongoDbTransaction;

    if (transaction) {
      storeTransaction = transaction.getStoreTransaction();
      mongoDbTransaction = transaction.getMongoDbTransaction();
    }

    const indicesRepository = await this.createDocumentMongoDbRepository(
      dataContractId,
      documentType,
      {
        isTransactional: transaction !== undefined,
      },
    );

    await indicesRepository.delete(documentId, mongoDbTransaction);

    await this.documentStoreRepository.delete(documentId, storeTransaction);
  }
}

module.exports = DocumentIndexedStoreRepository;
