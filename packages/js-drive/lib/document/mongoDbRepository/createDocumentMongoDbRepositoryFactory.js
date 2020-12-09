const DocumentMongoDbRepository = require('./DocumentMongoDbRepository');
const InvalidContractIdError = require('../query/errors/InvalidContractIdError');

/**
 * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
 * @param {validateQuery} validateQuery
 * @param {getDocumentDatabase} getDocumentMongoDBDatabase
 * @param {DataContractStoreRepository} dataContractRepository
 * @param {AwilixContainer} container
 * @param {Object} [options]
 * @param {boolean} [options.isPrevious]
 * @returns {createDocumentMongoDbRepository}
 */
function createDocumentMongoDbRepositoryFactory(
  convertWhereToMongoDbQuery,
  validateQuery,
  getDocumentMongoDBDatabase,
  dataContractRepository,
  container,
  options = {},
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

    const { isPrevious } = options;

    let blockExecutionTransactions = container.resolve('blockExecutionStoreTransactions');
    if (isPrevious) {
      blockExecutionTransactions = container.resolve('previousBlockExecutionStoreTransactions');
    }

    const dataContractTransaction = blockExecutionTransactions.getTransaction('dataContracts');

    // As documents are always created in the next block
    // we don't need transaction for data contracts here
    const dataContract = await dataContractRepository.fetch(
      dataContractId, dataContractTransaction,
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
