const bs58 = require('bs58');

const PREFIX = 'dpa_';

/**
 * @param {MongoClient} mongoClient
 * @param {SVDocumentMongoDbRepository} SVDocumentMongoDbRepository
 * @param {convertWhereToMongoDbQuery} convertWhereToMongoDbQuery
 * @param {validateQuery} validateQuery
 * @returns {createSVDocumentMongoDbRepository}
 */
function createSVDocumentMongoDbRepositoryFactory(
  mongoClient,
  SVDocumentMongoDbRepository,
  convertWhereToMongoDbQuery,
  validateQuery,
) {
  /**
   * Create SVDocumentMongoDbRepository
   *
   * @typedef {Promise} createSVDocumentMongoDbRepository
   * @param {string} contractId
   * @param {string} documentType
   * @returns {SVDocumentMongoDbRepository}
   */
  function createSVDocumentMongoDbRepository(contractId, documentType) {
    const base58ContractId = bs58.encode(Buffer.from(contractId, 'hex'));

    const mongoDb = mongoClient.db(`${process.env.STATEVIEW_MONGODB_DB_PREFIX}${PREFIX}${base58ContractId}`);

    return new SVDocumentMongoDbRepository(
      mongoDb,
      convertWhereToMongoDbQuery,
      validateQuery,
      documentType,
    );
  }

  return createSVDocumentMongoDbRepository;
}

module.exports = createSVDocumentMongoDbRepositoryFactory;
