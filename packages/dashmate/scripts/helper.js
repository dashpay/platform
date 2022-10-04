const dotenv = require('dotenv');
const { asValue } = require('awilix');
const createDIContainer = require('../src/createDIContainer');

(async function main() {
  // Read environment variables from .env file
  dotenv.config();

  console.info('Starting Dashmate helper');

  const container = await createDIContainer();

  // Load configs
  /**
   * @type {ConfigFileJsonRepository}
   */
  const configFileRepository = container.resolve('configFileRepository');

  const configFile = await configFileRepository.read();

  // Register config collection in the container
  container.register({
    configFile: asValue(configFile),
  });

  const defaultConfigName = configFile.getDefaultConfigName();

  if (defaultConfigName !== 'testnet') {
    return;
  }

  const config = configFile.getConfig(defaultConfigName);

  const provider = config.get('platform.dapi.envoy.ssl.provider');

  if (provider !== 'zerossl') {
    return;
  }

  const renewZeroSslCertificateHelper = container.resolve('renewZeroSslCertificateHelper');

  await renewZeroSslCertificateHelper(config);
}());
