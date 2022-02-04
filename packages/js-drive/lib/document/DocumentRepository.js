const createDocumentTypeTreePath = require('./groveDB/createDocumentTreePath');
const InvalidQueryError = require('./errors/InvalidQueryError');

class DocumentRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {validateQuery} validateQuery
   */
  constructor(
    groveDBStore,
    validateQuery,
  ) {
    this.storage = groveDBStore;
    this.validateQuery = validateQuery;
  }

  /**
   * Store document
   *
   * @param {DataContract} document
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @return {Promise<number>}
   */
  async store(document, useTransaction = false) {
    const isExists = await this.isExist(document, useTransaction);

    if (isExists) {
      return this.storage.getDrive().updateDocument(document, useTransaction);
    }

    return this.storage.getDrive().createDocument(document, useTransaction);
  }

  /**
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @return {Promise<boolean>}
   */
  async isExist(document, useTransaction = false) {
    const documentTypeTreePath = createDocumentTypeTreePath(
      document.getDataContract(),
      document.getType(),
    );

    const documentTreePath = documentTypeTreePath.push(Buffer.from(0));

    const [fetchedDocument] = await this.storage.get(
      documentTreePath,
      document.getId().toBuffer(),
      { useTransaction },
    );

    return Boolean(fetchedDocument);
  }

  /**
   * Find documents with query
   *
   * @param dataContract
   * @param documentType
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   * @param {boolean} [useTransaction=false]
   *
   * @throws InvalidQueryError
   *
   * @returns {Document[]}
   */
  async find(dataContract, documentType, query = {}, useTransaction = false) {
    const documentSchema = dataContract.getDocumentSchema(documentType);

    const result = this.validateQuery(query, documentSchema);

    if (!result.isValid()) {
      throw new InvalidQueryError(result.getErrors());
    }

    return this.storage.getDrive().queryDocuments(
      dataContract,
      documentType,
      query,
      useTransaction,
    );
  }

  /**
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} id
   * @param {boolean} useTransaction
   * @return {Promise<void>}
   */
  async delete(dataContract, documentType, id, useTransaction = false) {
    return this.storage.getDrive().deleteDocument(
      dataContract,
      documentType,
      id,
      useTransaction,
    );
  }
}

module.exports = DocumentRepository;
