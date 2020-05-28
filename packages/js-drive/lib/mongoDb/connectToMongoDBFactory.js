const { MongoClient } = require('mongodb');

const clients = Object.create({});

/**
 * Connect to a MongoDB and return the client back (factory)
 *
 * @param {string} connectionUrl
 *
 * @returns {connectToMongoDB}
 */
function connectToMongoDBFactory(connectionUrl) {
  /**
   * Connect to a MongoDB and return the client back
   *
   * @typedef connectToMongoDB
   *
   * @returns {Promise<MongoClient>}
   */
  async function connectToMongoDB() {
    if (clients[connectionUrl]) {
      return clients[connectionUrl];
    }

    clients[connectionUrl] = await MongoClient.connect(
      connectionUrl, {
        useUnifiedTopology: true,
      },
    );

    return clients[connectionUrl];
  }

  return connectToMongoDB;
}

module.exports = connectToMongoDBFactory;
