const DashDriveInstanceOptions = require('./DashDriveInstanceOptions');
const Network = require('../docker/Network');
const getAwsEcrAuthorizationToken = require('../docker/getAwsEcrAuthorizationToken');
const Image = require('../docker/Image');
const Container = require('../docker/Container');
const DashDriveInstance = require('./DashDriveInstance');
const { client: jaysonClient } = require('jayson');

/**
 * Create DashDrive instance
 *
 * @param {Array} envs
 * @returns {Promise<DashDriveInstance>}
 */
async function createDashDriveInstance(envs) {
  const options = new DashDriveInstanceOptions({ envs });

  const { name: networkName, driver } = options.getContainerNetworkOptions();
  const network = new Network(networkName, driver);

  const authorizationToken = await getAwsEcrAuthorizationToken(process.env.AWS_DEFAULT_REGION);

  const imageName = options.getContainerImageName();
  const image = new Image(imageName, authorizationToken);

  const containerOptions = options.getContainerOptions();
  const container = new Container(networkName, imageName, containerOptions);

  return new DashDriveInstance(network, image, container, jaysonClient, options);
}

module.exports = createDashDriveInstance;
