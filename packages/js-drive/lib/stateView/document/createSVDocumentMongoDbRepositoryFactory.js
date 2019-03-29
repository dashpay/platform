const PREFIX = 'dpa_';

/**
 * @param {MongoClient} mongoClient
 * @param {SVDocumentMongoDbRepository} SVDocumentMongoDbRepository
 * @param {sanitizer} sanitizer
 * @returns {createSVDocumentMongoDbRepository}
 */
function createSVDocumentMongoDbRepositoryFactory(
  mongoClient,
  SVDocumentMongoDbRepository,
  sanitizer,
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
    const mongoDb = mongoClient.db(`${process.env.MONGODB_DB_PREFIX}${PREFIX}${contractId}`);
    return new SVDocumentMongoDbRepository(mongoDb, sanitizer, documentType);
  }

  return createSVDocumentMongoDbRepository;
}

module.exports = createSVDocumentMongoDbRepositoryFactory;
