const bs58 = require('bs58');

const PREFIX = 'dpa_';

/**
 * @param {MongoClient} mongoClient
 * @param {SVObjectMongoDbRepository} SVObjectMongoDbRepository
 * @param {sanitizer} sanitizer
 * @returns {createSVObjectMongoDbRepository}
 */
function createSVObjectMongoDbRepositoryFactory(mongoClient, SVObjectMongoDbRepository, sanitizer) {
  /**
   * Create SVObjectMongoDbRepository
   *
   * @typedef {Promise} createSVObjectMongoDbRepository
   * @param {string} contractId
   * @param {string} objectType
   * @returns {SVObjectMongoDbRepository}
   */
  function createSVObjectMongoDbRepository(contractId, objectType) {
    const contractIdEncoded = bs58.encode(Buffer.from(contractId, 'hex'));
    const mongoDb = mongoClient.db(`${process.env.MONGODB_DB_PREFIX}${PREFIX}${contractIdEncoded}`);
    return new SVObjectMongoDbRepository(mongoDb, sanitizer, objectType);
  }

  return createSVObjectMongoDbRepository;
}

module.exports = createSVObjectMongoDbRepositoryFactory;
