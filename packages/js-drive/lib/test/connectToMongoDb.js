const assert = require('assert');
const { MongoClient } = require('mongodb');

/**
 * Connect to MongoDB and select database
 */
function connectToMongoDb() {
  const { url, dbName } = connectToMongoDb;

  // TODO: Validate in better way
  assert(url);
  assert(dbName);

  return new Promise((resolve) => {
    let client;
    let db;

    before(async () => {
      client = await MongoClient.connect(url);
      db = client.db(dbName);
      resolve(db);
    });

    beforeEach(async () => {
      await db.dropDatabase();
    });

    after(async () => {
      if (client) {
        await client.close();
      }
    });
  });
}

/**
 * Set connection url
 *
 * @param {string} url
 * @return {Function}
 */
connectToMongoDb.setUrl = function setUrl(url) {
  this.url = url;

  return this;
};

/**
 * Set db name
 *
 * @param {string} dbName
 * @return {Function}
 */
connectToMongoDb.setDbName = function setDbName(dbName) {
  this.dbName = dbName;

  return this;
};

module.exports = connectToMongoDb;
