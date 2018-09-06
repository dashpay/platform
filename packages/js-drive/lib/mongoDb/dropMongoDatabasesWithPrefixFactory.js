/**
 * @param {MongoClient} mongoClient
 * @returns {dropMongoDatabasesWithPrefix}
 */
function dropMongoDatabasesWithPrefixFactory(mongoClient) {
  /**
   * Drop all DashDrive MongoDB databases
   *
   * @typedef {Promise} dropMongoDatabasesWithPrefix
   * @param {string} prefix
   * @returns {Promise<void>}
   */
  async function dropMongoDatabasesWithPrefix(prefix) {
    const { databases: dbs } = await mongoClient.db('test').admin().listDatabases();
    const driveDatabases = dbs.filter(db => db.name.includes(prefix));

    for (let index = 0; index < driveDatabases.length; index++) {
      const db = driveDatabases[index];
      await mongoClient.db(db.name).dropDatabase();
    }
  }

  return dropMongoDatabasesWithPrefix;
}

module.exports = dropMongoDatabasesWithPrefixFactory;
