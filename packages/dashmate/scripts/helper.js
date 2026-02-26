import dotenv from 'dotenv';
import { asValue } from 'awilix';
import graceful from 'node-graceful';
import createDIContainer from '../src/createDIContainer.js';

// Container names that may be left orphaned from failed SSL renewal attempts
const EPHEMERAL_SSL_CONTAINERS = [
  'dashmate-zerossl-validation',
  'dashmate-letsencrypt-lego',
];

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

  const container = await createDIContainer(process.env);

  // Set up graceful shutdown to clean up any containers started during
  // SSL certificate renewal (e.g. the ZeroSSL verification server on port 80)
  const stopAllContainers = container.resolve('stopAllContainers');
  const startedContainers = container.resolve('startedContainers');

  graceful.exitOnDouble = false;
  graceful.on('exit', async () => {
    // eslint-disable-next-line no-console
    console.log('Shutting down dashmate helper, cleaning up containers...');

    await stopAllContainers(
      startedContainers.getContainers(),
      { remove: true },
    );
  });

  // Clean up any orphaned ephemeral SSL containers left from previous
  // failed renewal attempts (e.g. if the helper crashed or was killed
  // while a verification server was running on port 80)
  const docker = container.resolve('docker');

  await Promise.all(EPHEMERAL_SSL_CONTAINERS.map(async (name) => {
    try {
      const orphanedContainer = docker.getContainer(name);
      await orphanedContainer.remove({ force: true });

      // eslint-disable-next-line no-console
      console.log(`Removed orphaned container: ${name}`);
    } catch (e) {
      // 404 means container doesn't exist — that's the normal case
      if (e.statusCode !== 404) {
        // eslint-disable-next-line no-console
        console.error(`Failed to remove orphaned container ${name}: ${e.message}`);
      }
    }
  }));

  // Load configs
  /**
   * @type {ConfigFileJsonRepository}
   */
  const configFileRepository = container.resolve('configFileRepository');
  /**
   * @type {writeConfigTemplates}
   */
  const writeConfigTemplates = container.resolve('writeConfigTemplates');

  const configFile = await configFileRepository.read();

  // Persist config if it was migrated
  if (configFile.isChanged()) {
    await configFileRepository.write(configFile);

    configFile.getAllConfigs()
      .filter((config) => config.isChanged())
      .forEach(writeConfigTemplates);
  }

  const config = configFile.getConfig(configName);

  // Register config collection in the container
  container.register({
    configFile: asValue(configFile),
  });

  const provider = config.get('platform.gateway.ssl.provider');
  const isEnabled = config.get('platform.gateway.ssl.enabled');

  if (isEnabled && provider === 'zerossl') {
    const scheduleRenewZeroSslCertificate = container.resolve('scheduleRenewZeroSslCertificate');
    await scheduleRenewZeroSslCertificate(config);
  } else if (isEnabled && provider === 'letsencrypt') {
    const scheduleRenewLetsEncryptCertificate = container.resolve('scheduleRenewLetsEncryptCertificate');
    await scheduleRenewLetsEncryptCertificate(config);
  } else {
    // prevent infinite restarts
    setInterval(() => {
    }, 60 * 1000);
  }

  if (config.get('dashmate.helper.api.enable')) {
    const createHttpApiServer = container.resolve('createHttpApiServer');

    const httpAPIServer = createHttpApiServer();

    const port = config.get('dashmate.helper.api.port');

    httpAPIServer
      // eslint-disable-next-line no-console
      .listen(port, () => console.log(`Dashmate JSON-RPC API started on port ${port}`));
  }
}());
