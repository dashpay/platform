const { createHash } = require('crypto');

const createDocumentTypeTreePath = require('./groveDB/createDocumentTreePath');
const InvalidQueryError = require('./errors/InvalidQueryError');
const StartDocumentNotFoundError = require('./query/errors/StartDocumentNotFoundError');
const ValidationError = require('./query/errors/ValidationError');

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
   * @return {Promise<number>}
   */
  async store(document, useTransaction = false) {
    const isExists = await this.isExist(document, useTransaction);

    let result;
    let method = 'createDocument';

    try {
      if (isExists) {
        method = 'updateDocument';
        result = await this.storage.getDrive()
          .updateDocument(document, useTransaction);
      } else {
        result = await this.storage.getDrive()
          .createDocument(document, useTransaction);
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

    // Remove undefined options before we pass them to RS Drive
    Object.keys(query)
      .forEach((queryOption) => {
        if (query[queryOption] === undefined) {
          // eslint-disable-next-line no-param-reassign
          delete query[queryOption];
        }
      });

    try {
      return await this.storage.getDrive()
        .queryDocuments(
          dataContract,
          documentType,
          query,
          useTransaction,
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
   * @return {Promise<void>}
   */
  async delete(dataContract, documentType, id, useTransaction = false) {
    try {
      await this.storage.getDrive()
        .deleteDocument(
          dataContract,
          documentType,
          id,
          useTransaction,
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
