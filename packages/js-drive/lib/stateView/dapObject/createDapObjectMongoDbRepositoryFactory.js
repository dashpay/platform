const PREFIX = 'dap_';

/**
 * @param {MongoClient} mongoClient
 * @param {DapObjectMongoDbRepository} DapObjectMongoDbRepository
 * @returns {createDapObjectMongoDbRepository}
 */
function createDapObjectMongoDbRepositoryFactory(mongoClient, DapObjectMongoDbRepository) {
  /**
   * Create DapObjectMongoDbRepository
   *
   * @typedef {Promise} createDapObjectMongoDbRepository
   * @param {string} dapId
   * @returns {DapObjectMongoDbRepository}
   */
  function createDapObjectMongoDbRepository(dapId) {
    const mongoDb = mongoClient.db(`${process.env.MONGODB_DB_PREFIX}${PREFIX}${dapId}`);
    return new DapObjectMongoDbRepository(mongoDb);
  }

  return createDapObjectMongoDbRepository;
}

module.exports = createDapObjectMongoDbRepositoryFactory;
