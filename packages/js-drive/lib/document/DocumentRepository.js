const { createHash } = require('crypto');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

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

    let result;
    let method = 'createDocument';
    if (isExists) {
      result = await this.storage.getDrive().updateDocument(document, useTransaction);
      method = 'updateDocument';
    } else {
      result = await this.storage.getDrive().createDocument(document, useTransaction);
    }

    this.storage.logger.info({
      document: document.toBuffer().toString('hex'),
      documentHash: createHash('sha256')
        .update(
          document.toBuffer(),
        ).digest('hex'),
      useTransaction: Boolean(useTransaction),
      appHash: (await this.storage.getRootHash({ useTransaction })).toString('hex'),
    }, method);

    return result;
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

    const documentTreePath = documentTypeTreePath.concat(
      [Buffer.from([0])],
    );

    const fetchedDocument = await this.storage.get(
      documentTreePath,
      document.getId().toBuffer(),
      { useTransaction },
    );

    return Boolean(fetchedDocument);
  }

  /**
   * Find documents with query
   *
   * @param {DataContract} dataContract
   * @param {string} documentType
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
    await this.storage.getDrive().deleteDocument(
      dataContract,
      documentType,
      id,
      useTransaction,
    );

    this.storage.logger.info({
      dataContractId: dataContract.getId().toString(),
      documentType,
      id: Identifier.from(id).toString(),
      useTransaction: Boolean(useTransaction),
      appHash: (await this.storage.getRootHash({ useTransaction })).toString('hex'),
    }, 'deleteDocument');
  }
}

module.exports = DocumentRepository;
