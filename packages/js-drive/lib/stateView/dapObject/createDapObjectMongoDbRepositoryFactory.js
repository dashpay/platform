const bs58 = require('bs58');

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
    const dapIdEncoded = bs58.encode(Buffer.from(dapId, 'hex'));
    const mongoDb = mongoClient.db(`${process.env.MONGODB_DB_PREFIX}${PREFIX}${dapIdEncoded}`);
    return new DapObjectMongoDbRepository(mongoDb);
  }

  return createDapObjectMongoDbRepository;
}

module.exports = createDapObjectMongoDbRepositoryFactory;
