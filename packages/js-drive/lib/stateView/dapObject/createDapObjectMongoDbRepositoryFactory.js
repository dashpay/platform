const PREFIX = 'dap';

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
    const mongoDb = mongoClient.db(`${PREFIX}_${dapId}`);
    return new DapObjectMongoDbRepository(mongoDb);
  }

  return createDapObjectMongoDbRepository;
}

module.exports = createDapObjectMongoDbRepositoryFactory;
