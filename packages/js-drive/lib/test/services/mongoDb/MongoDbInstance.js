const DockerInstance = require('../docker/DockerInstance');

async function wait(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

class MongoDbInstance extends DockerInstance {
  /**
   * Create DashCore instance
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
    await this.initialization();
  }

  /**
   * Clean container and close MongoDb connection
   *
   * @return {Promise<void>}
   */
  async clean() {
    if (this.mongoClient) {
      await this.mongoClient.close();
    }

    await super.clean();
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
  async initialization() {
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
}

module.exports = MongoDbInstance;
