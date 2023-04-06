const dotenv = require('dotenv');
const { asValue } = require('awilix');
const createDIContainer = require('../src/createDIContainer');

const httpApiFactory = require('../src/helper/api/httpApiFactory')

(async function main() {
  // Read environment variables from .env file
  dotenv.config();

  const args = process.argv.slice(2);

  if (args.length !== 1) {
    throw new Error('please specify config name: "yarn workspace dashmate helper testnet"');
  }

  const [configName] = args;

  // eslint-disable-next-line no-console
  console.info('Starting dashmate helper');

  const container = await createDIContainer();

  // Load configs
  /**
   * @type {ConfigFileJsonRepository}
   */
  const configFileRepository = container.resolve('configFileRepository');

  const configFile = await configFileRepository.read();

  const config = configFile.getConfig(configName);

  // Register config collection in the container
  container.register({
    configFile: asValue(configFile),
    config: asValue(config),
    httpApi: asValue(httpApiFactory),
    flags: asValue({format: 'json'})
  });

  const provider = config.get('platform.dapi.envoy.ssl.provider');

  if (provider === 'zerossl') {
    const scheduleRenewZeroSslCertificate = container.resolve('scheduleRenewZeroSslCertificate');
    await scheduleRenewZeroSslCertificate(config);
  } else {
    // prevent infinite restarts
    setInterval(() => {}, 60 * 1000);
  }

  const httpApi = container.resolve('httpApi');
  httpApi(container)
}());
