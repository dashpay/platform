const Identifier = require('@dashevo/dpp/lib/Identifier');

const DocumentMongoDbRepository = require('./DocumentMongoDbRepository');
const InvalidContractIdError = require('../query/errors/InvalidContractIdError');

/**
 * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
 * @param {validateQuery} validateQuery
 * @param {getDocumentDatabase} getDocumentDatabase
 * @param {DataContractLevelDBRepository} dataContractRepository
 * @param {BlockExecutionDBTransactions} blockExecutionDBTransactions
 * @returns {createDocumentMongoDbRepository}
 */
function createDocumentMongoDbRepositoryFactory(
  convertWhereToMongoDbQuery,
  validateQuery,
  getDocumentDatabase,
  dataContractRepository,
  blockExecutionDBTransactions,
) {
  /**
   * Create DocumentMongoDbRepository
   *
   * @typedef {Promise} createDocumentMongoDbRepository
   * @param {Buffer|Identifier} dataContractId
   * @param {string} documentType
   * @returns {Promise<DocumentMongoDbRepository>}
   */
  async function createDocumentMongoDbRepository(dataContractId, documentType) {
    try {
      // eslint-disable-next-line no-param-reassign
      dataContractId = new Identifier(dataContractId);
    } catch (e) {
      if (e instanceof TypeError) {
        throw new InvalidContractIdError(dataContractId);
      }

      throw e;
    }

    const documentsMongoDatabase = await getDocumentDatabase(dataContractId);

    // Here we have to retrieve current LevelDB transaction
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
      documentsMongoDatabase,
      convertWhereToMongoDbQuery,
      validateQuery,
      dataContract,
      documentType,
    );
  }

  return createDocumentMongoDbRepository;
}

module.exports = createDocumentMongoDbRepositoryFactory;
