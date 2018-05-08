const MongoDbInstanceOptions = require('./MongoDbInstanceOptions');
const Network = require('../docker/Network');
const Image = require('../docker/Image');
const Container = require('../docker/Container');
const { MongoClient } = require('mongodb');
const MongoDbInstance = require('./MongoDbInstance');

/**
 * Create MongoDb instance
 *
 * @returns {Promise<DockerInstance>}
 */
async function createMongoDbInstance() {
  const options = new MongoDbInstanceOptions();

  const { name: networkName, driver } = options.getContainerNetworkOptions();
  const network = new Network(networkName, driver);

  const imageName = options.getContainerImageName();
  const image = new Image(imageName);

  const containerOptions = options.getContainerOptions();
  const container = new Container(networkName, imageName, containerOptions);

  return new MongoDbInstance(network, image, container, MongoClient, options);
}

module.exports = createMongoDbInstance;
