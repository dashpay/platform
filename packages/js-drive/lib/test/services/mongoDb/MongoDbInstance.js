const DockerInstance = require('../docker/DockerInstance');
const wait = require('../../util/wait');

class MongoDbInstance extends DockerInstance {
  /**
   * Create MongoDB instance
   *
   * @param {Network} network
   * @param {Image} image
   * @param {Container} container
   * @param {MongoClient} MongoClient
   * @param {MongoDbInstanceOptions} options
   */
  constructor(network, image, container, MongoClient, options) {
    super(network, image, container, options);
    this.MongoClient = MongoClient;
    this.options = options;
  }

  /**
   * Start instance
   *
   * @return {Promise<void>}
   */
  async start() {
    await super.start();
    await this.initialize();
  }

  /**
   * Clean container and close MongoDb connection
   *
   * @returns {Promise<void>}
   */
  async clean() {
    if (this.isMongoClientConnected()) {
      await this.mongoClient.db(this.options.mongo.name).dropDatabase();
    }
  }

  /**
   * Remove container and close MongoDb connection
   *
   * @returns {Promise<void>}
   */
  async remove() {
    if (this.isMongoClientConnected()) {
      await this.mongoClient.close();
    }

    await super.remove();
  }

  /**
   * Get Mongo client
   *
   * @return {Db}
   */
  async getMongoClient() {
    if (!this.isInitialized()) {
      return {};
    }
    if (this.mongoClient && this.mongoClient.isConnected(this.options.mongo.name)) {
      return this.mongoClient.db(this.options.mongo.name);
    }

    return this.mongoClient.db(this.options.mongo.name);
  }

  /**
   * @private
   *
   * @return {Promise<void>}
   */
  async initialize() {
    let mongoStarting = true;
    while (mongoStarting) {
      try {
        const address = `mongodb://127.0.0.1:${this.options.mongo.port}`;
        this.mongoClient = await this.MongoClient.connect(address);
        mongoStarting = false;
      } catch (error) {
        if (error.name !== 'MongoNetworkError') {
          throw error;
        }
        await wait(1000);
      }
    }
  }

  /**
   * @private
   *
   * @return {boolean}
   */
  isMongoClientConnected() {
    return this.mongoClient && this.mongoClient.isConnected(this.options.mongo.name);
  }
}

module.exports = MongoDbInstance;
