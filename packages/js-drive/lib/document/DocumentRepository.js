const { createHash } = require('crypto');

const PreCalculatedOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/PreCalculatedOperation');
const createDocumentTypeTreePath = require('./groveDB/createDocumentTreePath');
const InvalidQueryError = require('./errors/InvalidQueryError');
const StorageResult = require('../storage/StorageResult');

class DocumentRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {BaseLogger} [logger]
   */
  constructor(
    groveDBStore,
    logger = undefined,
  ) {
    this.storage = groveDBStore;
    this.logger = logger;
  }

  /**
   * Store document
   *
   * @param {DataContract} document
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @return {Promise<StorageResult<void>>}
   */
  async store(document, useTransaction = false) {
    const isExistsResult = await this.isExist(document, useTransaction);

    let processingCost;
    let storageCost;

    let method = 'createDocument';

    try {
      if (isExistsResult.getValue()) {
        method = 'updateDocument';
        ([storageCost, processingCost] = await this.storage.getDrive()
          .updateDocument(
            document,
            new Date('2022-03-17T15:08:26.132Z'),
            useTransaction,
          ));
      } else {
        ([storageCost, processingCost] = await this.storage.getDrive()
          .createDocument(
            document,
            new Date('2022-03-17T15:08:26.132Z'),
            useTransaction,
          ));
      }
    } finally {
      if (this.logger) {
        this.logger.info({
          document: document.toBuffer().toString('hex'),
          documentHash: createHash('sha256')
            .update(
              document.toBuffer(),
            ).digest('hex'),
          useTransaction: Boolean(useTransaction),
          appHash: (await this.storage.getRootHash({ useTransaction })).toString('hex'),
        }, method);
      }
    }

    return new StorageResult(
      undefined,
      [
        ...isExistsResult.getOperations(),
        new PreCalculatedOperation(storageCost, processingCost),
      ],
    );
  }

  /**
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @return {Promise<StorageResult<boolean>>}
   */
  async isExist(document, useTransaction = false) {
    const documentTypeTreePath = createDocumentTypeTreePath(
      document.getDataContract(),
      document.getType(),
    );

    const documentTreePath = documentTypeTreePath.concat(
      [Buffer.from([0])],
    );

    const result = await this.storage.get(
      documentTreePath,
      document.getId().toBuffer(),
      { useTransaction },
    );

    return new StorageResult(
      Boolean(result.getValue()),
      result.getOperations(),
    );
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
   * @returns {Promise<StorageResult<Document[]>>}
   */
  async find(dataContract, documentType, query = {}, useTransaction = false) {
    // Remove undefined options before we pass them to RS Drive
    Object.keys(query)
      .forEach((queryOption) => {
        if (query[queryOption] === undefined) {
          // eslint-disable-next-line no-param-reassign
          delete query[queryOption];
        }
      });

    try {
      const [documents, , processingCost] = await this.storage.getDrive()
        .queryDocuments(
          dataContract,
          documentType,
          query,
          useTransaction,
        );

      return new StorageResult(
        documents,
        [
          new PreCalculatedOperation(0, processingCost),
        ],
      );
    } catch (e) {
      if (e.message.startsWith('query:')) {
        throw new InvalidQueryError(e.message.substring(7, e.message.length));
      }

      if (e.message.startsWith('structure:')) {
        throw new InvalidQueryError(e.message.substring(10, e.message.length));
      }

      throw e;
    }
  }

  /**
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} id
   * @param {boolean} useTransaction
   * @return {Promise<StorageResult<void>>}
   */
  async delete(dataContract, documentType, id, useTransaction = false) {
    try {
      const [storageCost, processingCost] = await this.storage.getDrive()
        .deleteDocument(
          dataContract,
          documentType,
          id,
          useTransaction,
        );

      return new StorageResult(
        undefined,
        [
          new PreCalculatedOperation(storageCost, processingCost),
        ],
      );
    } finally {
      if (this.logger) {
        this.logger.info({
          dataContractId: dataContract.getId().toString(),
          documentType,
          id: id.toString(),
          useTransaction: Boolean(useTransaction),
          appHash: (await this.storage.getRootHash({ useTransaction })).toString('hex'),
        }, 'deleteDocument');
      }
    }
  }
}

module.exports = DocumentRepository;
