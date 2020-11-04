const DocumentMongoDbRepository = require('./DocumentMongoDbRepository');
const InvalidContractIdError = require('../query/errors/InvalidContractIdError');

/**
 * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
 * @param {validateQuery} validateQuery
 * @param {getDocumentDatabase} getDocumentMongoDBDatabase
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {BlockExecutionDBTransactions} blockExecutionDBTransactions
 * @returns {createDocumentMongoDbRepository}
 */
function createDocumentMongoDbRepositoryFactory(
  convertWhereToMongoDbQuery,
  validateQuery,
  getDocumentMongoDBDatabase,
  dataContractRepository,
  blockExecutionDBTransactions,
) {
  /**
   * Create DocumentMongoDbRepository
   *
   * @typedef {Promise} createDocumentMongoDbRepository
   * @param {Identifier} dataContractId
   * @param {string} documentType
   * @returns {Promise<DocumentMongoDbRepository>}
   */
  async function createDocumentMongoDbRepository(dataContractId, documentType) {
    const documentsMongoDBDatabase = await getDocumentMongoDBDatabase(dataContractId);

    // Here we have to retrieve current DB transaction
    // as we have to retrieve data contract to setup MongoDB collection
    // however this happening on block commit
    // and data contract is not visible outside of the transaction at that point
    const transaction = blockExecutionDBTransactions.getTransaction('dataContract');

    // As this function is used in other places the transaction might be
    // available, but not active. So we pass it in only if it was started
    const dataContract = await dataContractRepository.fetch(
      dataContractId, transaction.isStarted() ? transaction : undefined,
    );
    if (!dataContract) {
      throw new InvalidContractIdError(dataContractId);
    }

    return new DocumentMongoDbRepository(
      documentsMongoDBDatabase,
      convertWhereToMongoDbQuery,
      validateQuery,
      dataContract,
      documentType,
    );
  }

  return createDocumentMongoDbRepository;
}

module.exports = createDocumentMongoDbRepositoryFactory;
