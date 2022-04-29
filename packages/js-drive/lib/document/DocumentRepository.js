const { createHash } = require('crypto');

const PreCalculatedOperation = require('@dashevo/dpp/lib/stateTransition/fees/operations/PreCalculatedOperation');
const createDocumentTypeTreePath = require('./groveDB/createDocumentTreePath');
const InvalidQueryError = require('./errors/InvalidQueryError');
const StartDocumentNotFoundError = require('./query/errors/StartDocumentNotFoundError');
const ValidationError = require('./query/errors/ValidationError');
const RepositoryResult = require('../storage/RepositoryResult');

class DocumentRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   * @param {validateQuery} validateQuery
   * @param {BaseLogger} [logger]
   */
  constructor(
    groveDBStore,
    validateQuery,
    logger = undefined,
  ) {
    this.storage = groveDBStore;
    this.validateQuery = validateQuery;
    this.logger = logger;
  }

  /**
   * Store document
   *
   * @param {DataContract} document
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @return {Promise<RepositoryResult<void>>}
   */
  async store(document, useTransaction = false) {
    const isExistsResult = await this.isExist(document, useTransaction);

    let cpuCost;
    let storageCost;

    let method = 'createDocument';

    try {
      if (isExistsResult.getResult()) {
        method = 'updateDocument';
        ([storageCost, cpuCost] = await this.storage.getDrive()
          .updateDocument(
            document,
            new Date('2022-03-17T15:08:26.132Z'),
            useTransaction,
          ));
      } else {
        ([storageCost, cpuCost] = await this.storage.getDrive()
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

    return new RepositoryResult(
      undefined,
      [
        ...isExistsResult.getOperations(),
        new PreCalculatedOperation(storageCost, cpuCost),
      ],
    );
  }

  /**
   * @param {Document} document
   * @param {boolean} [useTransaction=false]
   * @return {Promise<RepositoryResult<boolean>>}
   */
  async isExist(document, useTransaction = false) {
    const documentTypeTreePath = createDocumentTypeTreePath(
      document.getDataContract(),
      document.getType(),
    );

    const documentTreePath = documentTypeTreePath.concat(
      [Buffer.from([0])],
    );

    const fetchedDocumentResult = await this.storage.get(
      documentTreePath,
      document.getId().toBuffer(),
      { useTransaction },
    );

    return new RepositoryResult(
      Boolean(fetchedDocumentResult.getResult()),
      fetchedDocumentResult.getOperations(),
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
   * @returns {Promise<RepositoryResult<Document[]>>}
   */
  async find(dataContract, documentType, query = {}, useTransaction = false) {
    const documentSchema = dataContract.getDocumentSchema(documentType);

    const result = this.validateQuery(query, documentSchema);

    if (!result.isValid()) {
      throw new InvalidQueryError(result.getErrors());
    }

    // Remove undefined options before we pass them to RS Drive
    Object.keys(query)
      .forEach((queryOption) => {
        if (query[queryOption] === undefined) {
          // eslint-disable-next-line no-param-reassign
          delete query[queryOption];
        }
      });

    try {
      const [documents, storageCost, cpuCost] = await this.storage.getDrive()
        .queryDocuments(
          dataContract,
          documentType,
          query,
          useTransaction,
        );

      return new RepositoryResult(
        documents,
        [
          new PreCalculatedOperation(storageCost, cpuCost),
        ],
      );
    } catch (e) {
      const invalidQueryMessagePrefix = 'invalid query: ';

      if (e.message.startsWith(invalidQueryMessagePrefix)) {
        let validationError;

        if (e.message === `${invalidQueryMessagePrefix}startAt document not found`) {
          validationError = new StartDocumentNotFoundError('startAt');
        }

        if (e.message === `${invalidQueryMessagePrefix}startAfter document not found`) {
          validationError = new StartDocumentNotFoundError('startAfter');
        }

        if (!validationError) {
          validationError = new ValidationError(
            e.message.substring(invalidQueryMessagePrefix.length),
          );
        }

        throw new InvalidQueryError([validationError]);
      }

      throw e;
    }
  }

  /**
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param {Identifier} id
   * @param {boolean} useTransaction
   * @return {Promise<RepositoryResult<void>>}
   */
  async delete(dataContract, documentType, id, useTransaction = false) {
    try {
      const [storageCost, cpuCost] = await this.storage.getDrive()
        .deleteDocument(
          dataContract,
          documentType,
          id,
          useTransaction,
        );

      return new RepositoryResult(
        undefined,
        [
          new PreCalculatedOperation(storageCost, cpuCost),
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
